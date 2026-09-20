use std::path::PathBuf;
use std::sync::Arc;

use axum::extract::{Path as AxumPath, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::Json;
use serde::Deserialize;
use serde_json::json;

use crate::asset_auditor::AssetAuditor;
use crate::cas::CasEngine;
use crate::db::Database;
use crate::live_tweaker::LiveTweakerHub;
use crate::models::{CrashReport, PlaytestFeedback, ReplayData, ShaderHitchEvent};
use crate::shader_prewarm::ShaderPrewarmGenerator;
use crate::wasm_host::WasmHost;

#[derive(Clone)]
pub struct AppState {
    pub db: Database,
    pub cas: CasEngine,
    pub tweaker: LiveTweakerHub,
    pub wasm_host: WasmHost,
    pub base_dir: PathBuf,
}

#[derive(Deserialize)]
pub struct CrashQuery {
    pub scene: Option<String>,
    pub limit: Option<u32>,
}

#[derive(Deserialize)]
pub struct SpatialQuery {
    pub scene: String,
}

#[derive(Deserialize)]
pub struct ProjectParamQuery {
    pub project_id: String,
}

#[derive(Deserialize)]
pub struct AuditQuery {
    pub path: String,
}

#[derive(Deserialize)]
pub struct TweakBody {
    pub path: String,
    pub value: serde_json::Value,
}

#[derive(Deserialize)]
pub struct CommandBody {
    pub cmd: String,
}

pub fn create_api_router(state: Arc<AppState>) -> axum::Router {
    axum::Router::new()
        // INGESTION
        .route("/api/v1/crashes", post(ingest_crash))
        .route("/api/v1/replays", post(ingest_replay))
        .route("/api/v1/shaders", post(ingest_shader))
        .route("/api/v1/feedback", post(ingest_feedback))
        // CAS
        .route("/api/v1/cas/upload", post(upload_cas))
        .route("/api/v1/cas/{hash}", get(get_cas_object))
        .route("/api/v1/cas/stats", get(get_cas_stats))
        // QUERY & DASHBOARD
        .route("/api/v1/projects/{project_id}/crashes", get(list_crashes))
        .route("/api/v1/crashes/{id}", get(get_crash_details))
        .route("/api/v1/replays/{id}", get(get_replay))
        .route(
            "/api/v1/projects/{project_id}/spatial_hotspots",
            get(get_spatial_hotspots),
        )
        .route(
            "/api/v1/projects/{project_id}/shaders",
            get(list_shader_hitches),
        )
        .route("/api/v1/projects/{project_id}/feedback", get(list_feedback))
        // LIVE TUNING & CONSOLE
        .route("/api/v1/live/tweakables", get(list_tweakables))
        .route("/api/v1/live/tweak", post(set_tweakable))
        .route("/api/v1/live/command", post(exec_command))
        // SHADER PREWARM GENERATOR
        .route("/api/v1/shader_prewarm/godot", get(generate_godot_prewarm))
        .route("/api/v1/shader_prewarm/unity", get(generate_unity_prewarm))
        // ASSET AUDITOR
        .route("/api/v1/audit", get(run_asset_audit))
        // WEBSOCKET
        .route(
            "/ws/live_tuning",
            get(|ws: axum::extract::WebSocketUpgrade, State(state): State<Arc<AppState>>| async move {
                ws.on_upgrade(move |socket| async move {
                    state.tweaker.handle_socket(socket).await;
                })
            }),
        )
        .with_state(state)
}

// HANDLERY INGESTION

async fn ingest_crash(
    State(state): State<Arc<AppState>>,
    Json(report): Json<CrashReport>,
) -> Response {
    match state.db.record_crash(&report) {
        Ok(id) => (
            StatusCode::CREATED,
            Json(json!({ "status": "ok", "crash_id": id })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

async fn ingest_replay(
    State(state): State<Arc<AppState>>,
    Json(replay): Json<ReplayData>,
) -> Response {
    match state.db.save_replay(&replay) {
        Ok(_) => (
            StatusCode::CREATED,
            Json(json!({ "status": "ok", "replay_id": replay.id })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

async fn ingest_shader(
    State(state): State<Arc<AppState>>,
    Json(hitch): Json<ShaderHitchEvent>,
) -> Response {
    match state.db.record_shader_hitch(&hitch) {
        Ok(_) => (StatusCode::CREATED, Json(json!({ "status": "ok" }))).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

async fn ingest_feedback(
    State(state): State<Arc<AppState>>,
    Json(feedback): Json<PlaytestFeedback>,
) -> Response {
    match state.db.save_feedback(&feedback) {
        Ok(_) => (
            StatusCode::CREATED,
            Json(json!({ "status": "ok", "feedback_id": feedback.id })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

// HANDLERY CAS

async fn upload_cas(
    State(state): State<Arc<AppState>>,
    body: axum::body::Bytes,
) -> Response {
    match state.cas.put(&body, "application/octet-stream", "generic") {
        Ok((hash, orig, comp)) => (
            StatusCode::CREATED,
            Json(json!({
                "hash": hash,
                "original_size": orig,
                "compressed_size": comp
            })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

async fn get_cas_object(
    State(state): State<Arc<AppState>>,
    AxumPath(hash): AxumPath<String>,
) -> Response {
    match state.cas.get(&hash) {
        Ok(data) => (StatusCode::OK, data).into_response(),
        Err(_) => (StatusCode::NOT_FOUND, "Object not found").into_response(),
    }
}

async fn get_cas_stats(State(state): State<Arc<AppState>>) -> Response {
    match state.cas.stats() {
        Ok(stats) => Json(stats).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

// HANDLERY ZAPYTAŃ (DASHBOARD & MCP)

async fn list_crashes(
    State(state): State<Arc<AppState>>,
    AxumPath(project_id): AxumPath<String>,
    Query(q): Query<CrashQuery>,
) -> Response {
    let limit = q.limit.unwrap_or(20);
    match state.db.list_crashes(&project_id, q.scene.as_deref(), limit) {
        Ok(crashes) => Json(crashes).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

async fn get_crash_details(
    State(state): State<Arc<AppState>>,
    AxumPath(id): AxumPath<i64>,
) -> Response {
    match state.db.get_crash(id) {
        Ok(Some(crash)) => Json(crash).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, "Crash not found").into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

async fn get_replay(
    State(state): State<Arc<AppState>>,
    AxumPath(id): AxumPath<String>,
) -> Response {
    match state.db.get_replay(&id) {
        Ok(Some(replay)) => Json(replay).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, "Replay not found").into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

async fn get_spatial_hotspots(
    State(state): State<Arc<AppState>>,
    AxumPath(project_id): AxumPath<String>,
    Query(q): Query<SpatialQuery>,
) -> Response {
    match state.db.get_spatial_clusters(&project_id, &q.scene) {
        Ok(clusters) => Json(clusters).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

async fn list_shader_hitches(
    State(state): State<Arc<AppState>>,
    AxumPath(project_id): AxumPath<String>,
) -> Response {
    match state.db.list_shader_hitches(&project_id) {
        Ok(hitches) => Json(hitches).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

async fn list_feedback(
    State(state): State<Arc<AppState>>,
    AxumPath(project_id): AxumPath<String>,
) -> Response {
    match state.db.list_feedback(&project_id, 50) {
        Ok(feedback) => Json(feedback).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

// LIVE TUNING REST ENDPOINTS

async fn list_tweakables(State(state): State<Arc<AppState>>) -> Response {
    let list = state.tweaker.get_tweakables().await;
    Json(list).into_response()
}

async fn set_tweakable(
    State(state): State<Arc<AppState>>,
    Json(body): Json<TweakBody>,
) -> Response {
    let success = state.tweaker.tweak_var(&body.path, body.value).await;
    Json(json!({ "status": if success { "ok" } else { "failed" } })).into_response()
}

async fn exec_command(
    State(state): State<Arc<AppState>>,
    Json(body): Json<CommandBody>,
) -> Response {
    let count = state.tweaker.send_command(&body.cmd).await;
    Json(json!({ "status": "sent", "receivers": count })).into_response()
}

// GENERATORY SHADER PREWARM

async fn generate_godot_prewarm(
    State(state): State<Arc<AppState>>,
    Query(q): Query<ProjectParamQuery>,
) -> Response {
    let generator = ShaderPrewarmGenerator::new(state.db.clone());
    match generator.generate_godot_script(&q.project_id) {
        Ok(code) => (StatusCode::OK, code).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

async fn generate_unity_prewarm(
    State(state): State<Arc<AppState>>,
    Query(q): Query<ProjectParamQuery>,
) -> Response {
    let generator = ShaderPrewarmGenerator::new(state.db.clone());
    match generator.generate_unity_script(&q.project_id) {
        Ok(code) => (StatusCode::OK, code).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

// ASSET AUDITOR

async fn run_asset_audit(Query(q): Query<AuditQuery>) -> Response {
    let path = std::path::Path::new(&q.path);
    match AssetAuditor::audit(path) {
        Ok(report) => Json(report).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}
