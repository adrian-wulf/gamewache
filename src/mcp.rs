use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::process::Command;
use tracing::error;

use crate::api::AppState;
use crate::asset_auditor::AssetAuditor;
use crate::shader_prewarm::ShaderPrewarmGenerator;

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct JsonRpcRequest {
    jsonrpc: String,
    id: Option<Value>,
    method: String,
    #[serde(default)]
    params: Value,
}

#[derive(Debug, Serialize)]
struct JsonRpcResponse {
    jsonrpc: &'static str,
    id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<Value>,
}

pub async fn run_mcp_stdio(state: Arc<AppState>, default_project_dir: Option<PathBuf>) -> io::Result<()> {
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }

        let req: JsonRpcRequest = match serde_json::from_str(&line) {
            Ok(r) => r,
            Err(e) => {
                error!("Failed to parse JSON-RPC request: {}", e);
                continue;
            }
        };

        if req.method == "notifications/initialized" {
            continue;
        }

        let resp_result = handle_mcp_method(&state, &req.method, req.params, default_project_dir.as_deref()).await;

        if let Some(id) = req.id {
            let resp = match resp_result {
                Ok(res) => JsonRpcResponse {
                    jsonrpc: "2.0",
                    id,
                    result: Some(res),
                    error: None,
                },
                Err(err_msg) => JsonRpcResponse {
                    jsonrpc: "2.0",
                    id,
                    result: None,
                    error: Some(json!({
                        "code": -32603,
                        "message": err_msg
                    })),
                },
            };

            let json_str = serde_json::to_string(&resp).unwrap();
            writeln!(stdout, "{}", json_str)?;
            stdout.flush()?;
        }
    }

    Ok(())
}

async fn handle_mcp_method(
    state: &Arc<AppState>,
    method: &str,
    params: Value,
    default_dir: Option<&Path>,
) -> Result<Value, String> {
    match method {
        "initialize" => Ok(json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {
                "tools": {}
            },
            "serverInfo": {
                "name": "gamewache",
                "version": "0.1.0"
            }
        })),
        "tools/list" => Ok(json!({
            "tools": get_tool_definitions()
        })),
        "tools/call" => {
            let tool_name = params
                .get("name")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Missing 'name' in tools/call".to_string())?;

            let args = params.get("arguments").cloned().unwrap_or(json!({}));

            let text_output = execute_tool(state, tool_name, args, default_dir).await?;

            Ok(json!({
                "content": [
                    {
                        "type": "text",
                        "text": text_output
                    }
                ]
            }))
        }
        _ => Err(format!("Unknown method: {}", method)),
    }
}

fn get_tool_definitions() -> Vec<Value> {
    vec![
        json!({
            "name": "gw_query_crashes",
            "description": "Wyszukuje i filtruje listę ostatnich awarii oraz wyjątków w silniku gry (project_id opcjonalne, auto-detekcja z aktywnego projektu).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "project_id": { "type": "string", "description": "Opcjonalne ID projektu gry (domyślnie aktywny katalog)" },
                    "scene_name": { "type": "string", "description": "Opcjonalna nazwa sceny" },
                    "limit": { "type": "integer", "description": "Maksymalna liczba wyników (domyślnie 10)" }
                }
            }
        }),
        json!({
            "name": "gw_get_crash_details",
            "description": "Pobiera szczegółowy zrzut awarii ze stosem wywołań, zmiennymi, okruchami chleba i danymi sprzętowymi.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "crash_id": { "type": "integer", "description": "ID zgłoszenia crasha" }
                },
                "required": ["crash_id"]
            }
        }),
        json!({
            "name": "gw_inspect_replay",
            "description": "Pobiera i dekoduje sekwencję wejść gracza z czarnej skrzynki (Blackbox) poprzedzających crash.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "replay_id": { "type": "string", "description": "ID powtórki .gwrec" }
                },
                "required": ["replay_id"]
            }
        }),
        json!({
            "name": "gw_spatial_hotspots",
            "description": "Zwraca geometryczne skupiska awarii lub zgonów graczy na zadanym poziomie gry.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "project_id": { "type": "string", "description": "ID projektu" },
                    "scene_name": { "type": "string", "description": "Ścieżka sceny" }
                },
                "required": ["project_id", "scene_name"]
            }
        }),
        json!({
            "name": "gw_tweak_runtime_var",
            "description": "Modyfikuje zmienną w pamięci działającej gry w czasie rzeczywistym przez WebSocket.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "param_path": { "type": "string", "description": "Ścieżka parametru (np. 'Combat/Turret Speed')" },
                    "value": { "description": "Nowa wartość parametru (liczba, tekst, bool)" }
                },
                "required": ["param_path", "value"]
            }
        }),
        json!({
            "name": "gw_exec_console_cmd",
            "description": "Wykonuje polecenie w zdalnej konsoli developerskiej gry.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "cmd": { "type": "string", "description": "Treść komendy (np. 'god_mode', 'teleport 10 20')" }
                },
                "required": ["cmd"]
            }
        }),
        json!({
            "name": "gw_audit_assets",
            "description": "Skanuje projekt gry i wykrywa sieroce assety (nieużywane pliki) oraz naruszenia budżetu VRAM/RAM.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "project_dir": { "type": "string", "description": "Ścieżka do katalogu projektu gry" }
                }
            }
        }),
        json!({
            "name": "gw_generate_shader_prewarm",
            "description": "Generuje skrypt rozgrzewający shadery (prewarm) eliminujący zacięcia (hitch) w grze.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "project_id": { "type": "string", "description": "ID projektu" },
                    "target_engine": { "type": "string", "enum": ["godot", "unity"], "description": "Docelowy silnik" }
                },
                "required": ["project_id"]
            }
        }),
        json!({
            "name": "gw_cache_stats",
            "description": "Zwraca statystyki magazynu Content Addressable Storage (CAS): trafienia, rozmiar i oszczędności.",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        }),
        json!({
            "name": "gw_get_playtester_feedback",
            "description": "Pobiera listę zgłoszeń i opinii od graczy przesłanych przez nakładkę F11.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "project_id": { "type": "string", "description": "ID projektu" },
                    "limit": { "type": "integer", "description": "Limit zgłoszeń" }
                },
                "required": ["project_id"]
            }
        }),
        json!({
            "name": "gw_run_headless_tests",
            "description": "Uruchamia testy jednostkowe w trybie headless w silniku Godot lub Unity.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "project_dir": { "type": "string", "description": "Ścieżka do projektu" },
                    "test_filter": { "type": "string", "description": "Opcjonalny filtr testów" }
                }
            }
        }),
        json!({
            "name": "gw_deploy_web_playtest",
            "description": "Wystawia build WebAssembly na prywatnym serwerze GameWache i generuje link dostępowy.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "project_id": { "type": "string", "description": "ID projektu" },
                    "channel": { "type": "string", "description": "Kanał (np. nightly, playtest-v1)" },
                    "build_dir": { "type": "string", "description": "Ścieżka do katalogu z wyeksportowanym HTML5/WASM" }
                },
                "required": ["project_id", "channel", "build_dir"]
            }
        })
    ]
}

fn resolve_project_dir(args_val: Option<&str>, default_dir: Option<&Path>) -> PathBuf {
    if let Some(d) = args_val {
        if !d.trim().is_empty() {
            return PathBuf::from(d.trim());
        }
    }
    if let Some(d) = default_dir {
        return d.to_path_buf();
    }
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

fn resolve_project_id(args_val: Option<&str>, default_dir: Option<&Path>) -> String {
    if let Some(p) = args_val {
        if !p.trim().is_empty() {
            return p.trim().to_string();
        }
    }

    let pdir = resolve_project_dir(None, default_dir);
    let godot_cfg = pdir.join("project.godot");
    if godot_cfg.exists() {
        if let Ok(content) = std::fs::read_to_string(&godot_cfg) {
            for line in content.lines() {
                if line.contains("config/name=") {
                    let parts: Vec<&str> = line.split('=').collect();
                    if parts.len() > 1 {
                        let name = parts[1].trim().trim_matches('"');
                        return name.to_lowercase().replace(' ', "-");
                    }
                }
            }
        }
    }

    pdir.file_name()
        .and_then(|f| f.to_str())
        .unwrap_or("default-game")
        .to_string()
}

async fn execute_tool(
    state: &Arc<AppState>,
    name: &str,
    args: Value,
    default_dir: Option<&Path>,
) -> Result<String, String> {
    match name {
        "gw_query_crashes" => {
            let project_id = resolve_project_id(args["project_id"].as_str(), default_dir);
            let scene = args["scene_name"].as_str();
            let limit = args["limit"].as_u64().unwrap_or(10) as u32;

            let crashes = state
                .db
                .list_crashes(&project_id, scene, limit)
                .map_err(|e| e.to_string())?;

            Ok(serde_json::to_string_pretty(&crashes).unwrap())
        }
        "gw_get_crash_details" => {
            let crash_id = args["crash_id"].as_i64().ok_or("Missing crash_id")?;
            let crash = state
                .db
                .get_crash(crash_id)
                .map_err(|e| e.to_string())?
                .ok_or_else(|| format!("Crash #{} not found", crash_id))?;

            Ok(serde_json::to_string_pretty(&crash).unwrap())
        }
        "gw_inspect_replay" => {
            let replay_id = args["replay_id"].as_str().ok_or("Missing replay_id")?;
            let replay = state
                .db
                .get_replay(replay_id)
                .map_err(|e| e.to_string())?
                .ok_or_else(|| format!("Replay {} not found", replay_id))?;

            Ok(serde_json::to_string_pretty(&replay).unwrap())
        }
        "gw_spatial_hotspots" => {
            let project_id = resolve_project_id(args["project_id"].as_str(), default_dir);
            let scene_name = args["scene_name"].as_str().ok_or("Missing scene_name")?;

            let clusters = state
                .db
                .get_spatial_clusters(&project_id, scene_name)
                .map_err(|e| e.to_string())?;

            Ok(serde_json::to_string_pretty(&clusters).unwrap())
        }
        "gw_tweak_runtime_var" => {
            let path = args["param_path"].as_str().ok_or("Missing param_path")?;
            let val = args["value"].clone();

            let success = state.tweaker.tweak_var(path, val.clone()).await;
            Ok(format!(
                "Zaktualizowano parametr '{}' do wartości: {}\nStatus wysyłki do gry: {}",
                path,
                val,
                if success { "Sukces (Wysłano przez WebSocket)" } else { "Błąd" }
            ))
        }
        "gw_exec_console_cmd" => {
            let cmd = args["cmd"].as_str().ok_or("Missing cmd")?;
            let count = state.tweaker.send_command(cmd).await;
            Ok(format!(
                "Wysłano komendę konsoli '{}' do {} aktywnych instancji gry.",
                cmd, count
            ))
        }
        "gw_audit_assets" => {
            let pdir = resolve_project_dir(args["project_dir"].as_str(), default_dir);
            let report = AssetAuditor::audit(&pdir).map_err(|e| e.to_string())?;
            Ok(serde_json::to_string_pretty(&report).unwrap())
        }
        "gw_generate_shader_prewarm" => {
            let project_id = resolve_project_id(args["project_id"].as_str(), default_dir);
            let engine = args["target_engine"].as_str().unwrap_or("godot");

            let generator = ShaderPrewarmGenerator::new(state.db.clone());
            if engine == "unity" {
                let code = generator
                    .generate_unity_script(&project_id)
                    .map_err(|e| e.to_string())?;
                Ok(code)
            } else {
                let code = generator
                    .generate_godot_script(&project_id)
                    .map_err(|e| e.to_string())?;
                Ok(code)
            }
        }
        "gw_cache_stats" => {
            let stats = state.cas.stats().map_err(|e| e.to_string())?;
            Ok(serde_json::to_string_pretty(&stats).unwrap())
        }
        "gw_get_playtester_feedback" => {
            let project_id = resolve_project_id(args["project_id"].as_str(), default_dir);
            let limit = args["limit"].as_u64().unwrap_or(20) as u32;

            let feedback = state
                .db
                .list_feedback(&project_id, limit)
                .map_err(|e| e.to_string())?;

            Ok(serde_json::to_string_pretty(&feedback).unwrap())
        }
        "gw_run_headless_tests" => {
            let pdir = resolve_project_dir(args["project_dir"].as_str(), default_dir);

            let mut cmd = Command::new("godot");
            cmd.arg("--headless");
            cmd.arg("--path");
            cmd.arg(&pdir);
            cmd.arg("-s");
            cmd.arg("addons/gut/gut_cmdln.gd");

            if let Some(filter) = args["test_filter"].as_str() {
                cmd.arg("-gselect");
                cmd.arg(filter);
            }

            let output = cmd.output().await.map_err(|e| format!("Błąd uruchomienia silnika Godot: {}", e))?;
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);

            Ok(format!(
                "Status: {}\n--- STDOUT ---\n{}\n--- STDERR ---\n{}",
                output.status, stdout, stderr
            ))
        }
        "gw_deploy_web_playtest" => {
            let project_id = resolve_project_id(args["project_id"].as_str(), default_dir);
            let channel = args["channel"].as_str().ok_or("Missing channel")?;
            let build_dir = args["build_dir"].as_str().ok_or("Missing build_dir")?;

            let token = state
                .wasm_host
                .deploy_build(&project_id, channel, Path::new(build_dir))
                .map_err(|e| e.to_string())?;

            Ok(format!(
                "Pomyślnie wystawiono build WASM dla kanału '{}'!\nLink dla testera: http://localhost:8090/play/{}/{}?t={}",
                channel, project_id, channel, token
            ))
        }
        _ => Err(format!("Unknown tool: {}", name)),
    }
}
