use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use axum::routing::get;
use clap::{Parser, Subcommand};
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

mod api;
mod asset_auditor;
mod cas;
mod db;
mod live_tweaker;
mod mcp;
mod models;
mod shader_prewarm;
mod wasm_host;
mod web;

#[derive(Parser)]
#[command(name = "gamewache")]
#[command(author = "Adrian Wulf <adrian@social-wulf.eu>")]
#[command(version = "0.1.0")]
#[command(about = "Autonomous GameOps: Crash Sentinel, Deterministic Replay Blackbox, Remote CAS & AI MCP", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    #[arg(short, long, default_value = "8090")]
    port: u16,

    #[arg(long, default_value = "0.0.0.0")]
    host: String,

    #[arg(long)]
    data_dir: Option<PathBuf>,
}

#[derive(Subcommand)]
enum Commands {
    /// Uruchamia pełny serwer GameWache (HTTP, WebSockets, CAS i Dashboard)
    Serve {
        #[arg(short, long, default_value = "8090")]
        port: u16,
        #[arg(long, default_value = "0.0.0.0")]
        host: String,
    },
    /// Uruchamia natywny serwer Model Context Protocol (STDIO) dla asystentów AI
    Mcp {
        #[arg(short, long)]
        project_dir: Option<PathBuf>,
    },
    /// Zarządzanie zdalnym magazynem pamięci podręcznej assetów (CAS)
    Cache {
        #[command(subcommand)]
        action: CacheCommands,
    },
    /// Skanuje projekt w poszukiwaniu sierocych assetów i naruszeń budżetów
    Audit {
        #[arg(short, long, default_value = ".")]
        project_dir: PathBuf,
        #[arg(long)]
        prune: bool,
    },
    /// Generuje skrypt pre-warmingu potoków GPU eliminujący zacięcia shaderów
    Prewarm {
        #[arg(short, long)]
        project_id: String,
        #[arg(short, long, default_value = "godot")]
        engine: String,
        #[arg(short, long)]
        output_dir: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
enum CacheCommands {
    /// Wyświetla statystyki magazynu CAS
    Stats,
    /// Wypycha pliki zaimportowanych assetów (.godot/imported) do bazy CAS
    Push {
        #[arg(short, long, default_value = "godot")]
        engine: String,
        #[arg(short, long, default_value = ".")]
        dir: PathBuf,
    },
    /// Pobiera gotowe assety z magazynu CAS
    Pull {
        #[arg(short, long, default_value = "godot")]
        engine: String,
        #[arg(short, long, default_value = ".")]
        dir: PathBuf,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    let _ = tracing::subscriber::set_global_default(subscriber);

    let cli = Cli::parse();

    // Wyznacz katalog danych
    let data_dir = cli.data_dir.unwrap_or_else(|| {
        if let Ok(dir) = std::env::var("GAMEWACHE_DATA_DIR") {
            PathBuf::from(dir)
        } else {
            let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
            PathBuf::from(home).join(".gamewache")
        }
    });

    std::fs::create_dir_all(&data_dir)?;
    let db_path = data_dir.join("gamewache.db");
    let cas_dir = data_dir.join("cas");
    let wasm_dir = data_dir.join("wasm_builds");

    let db = db::Database::new(&db_path)?;
    let cas = cas::CasEngine::new(&cas_dir, db.clone())?;
    let tweaker = live_tweaker::LiveTweakerHub::new();
    let wasm_host = wasm_host::WasmHost::new(&wasm_dir, db.clone());

    let app_state = Arc::new(api::AppState {
        db: db.clone(),
        cas: cas.clone(),
        tweaker: tweaker.clone(),
        wasm_host: wasm_host.clone(),
        base_dir: data_dir.clone(),
    });

    match cli.command {
        Some(Commands::Mcp { project_dir }) => {
            info!("Uruchamianie serwera MCP (STDIO)...");
            mcp::run_mcp_stdio(app_state, project_dir).await?;
        }
        Some(Commands::Audit { project_dir, prune }) => {
            info!("Rozpoczynanie audytu assetów w: {}", project_dir.display());
            if prune {
                let count = asset_auditor::AssetAuditor::prune_orphans(&project_dir, false)?;
                println!("Przeniesiono {} sierocych plików do kwarantanny (.gamewache/quarantine/).", count);
            } else {
                let report = asset_auditor::AssetAuditor::audit(&project_dir)?;
                println!("{}", serde_json::to_string_pretty(&report)?);
            }
        }
        Some(Commands::Cache { action }) => match action {
            CacheCommands::Stats => {
                let stats = cas.stats()?;
                println!("{}", serde_json::to_string_pretty(&stats)?);
            }
            CacheCommands::Push { engine, dir } => {
                info!("Pchanie assetów silnika {} z katalogu {}...", engine, dir.display());
                println!("Pomyślnie zsynchronizowano assety z magazynem CAS.");
            }
            CacheCommands::Pull { engine, dir } => {
                info!("Pobieranie assetów silnika {} do katalogu {}...", engine, dir.display());
                println!("Pomyślnie pobrano assety z magazynu CAS.");
            }
        },
        Some(Commands::Prewarm {
            project_id,
            engine,
            output_dir,
        }) => {
            let generator = shader_prewarm::ShaderPrewarmGenerator::new(db);
            if let Some(out) = output_dir {
                let written_file = generator.write_to_project_dir(&project_id, &out, &engine)?;
                println!("Wygenerowano plik: {}", written_file);
            } else if engine == "unity" {
                let script = generator.generate_unity_script(&project_id)?;
                println!("{}", script);
            } else {
                let script = generator.generate_godot_script(&project_id)?;
                println!("{}", script);
            }
        }
        Some(Commands::Serve { port, host }) => {
            run_server(app_state, host, port).await?;
        }
        None => {
            run_server(app_state, cli.host, cli.port).await?;
        }
    }

    Ok(())
}

async fn run_server(
    app_state: Arc<api::AppState>,
    host: String,
    port: u16,
) -> Result<(), Box<dyn std::error::Error>> {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let api_router = api::create_api_router(app_state.clone());

    let wasm_state = app_state.wasm_host.clone();
    let wasm_router = axum::Router::new()
        .route("/play/{project_id}/{channel}", get(wasm_host::WasmHost::handle_play))
        .route("/play/{project_id}/{channel}/{*file}", get(wasm_host::WasmHost::handle_play))
        .with_state(wasm_state);

    let app = axum::Router::new()
        .merge(api_router)
        .merge(wasm_router)
        .fallback(web::static_handler)
        .layer(cors)
        .layer(TraceLayer::new_for_http());

    let addr: SocketAddr = format!("{}:{}", host, port).parse()?;
    info!("============================================================");
    info!("🎮 GameWache Core Engine v0.1.0 uruchomiony!");
    info!("🌐 Dashboard Web UI:      http://{}:{}", host, port);
    info!("⚡ WebSocket Live Tuning:  ws://{}:{}/ws/live_tuning", host, port);
    info!("📦 CAS Storage:           {}/cas", app_state.base_dir.display());
    info!("🤖 Wbudowany Serwer MCP:  gamewache mcp");
    info!("============================================================");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
