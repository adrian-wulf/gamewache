use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub engine: String, // "godot" | "unity" | "unreal" | "custom"
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StackFrame {
    pub filename: String,
    pub function: String,
    pub lineno: u32,
    #[serde(default)]
    pub colno: u32,
    #[serde(default)]
    pub context_lines: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Breadcrumb {
    pub timestamp_ms: u64,
    #[serde(default)]
    pub engine_tick: u64,
    pub category: String, // "scene", "state", "combat", "audio", "perf", "input"
    pub message: String,
    #[serde(default)]
    pub level_name: Option<String>,
    #[serde(default)]
    pub player_pos: Option<[f32; 3]>,
    #[serde(default)]
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrashReport {
    pub project_id: String,
    pub exception_type: String,
    pub message: String,
    #[serde(default)]
    pub stacktrace: Vec<StackFrame>,
    #[serde(default)]
    pub source_file: Option<String>,
    #[serde(default)]
    pub line_no: Option<u32>,
    #[serde(default = "default_severity")]
    pub severity: String, // "fatal", "error", "warning"
    #[serde(default)]
    pub platform: String, // "windows", "linux", "web", "android"
    #[serde(default)]
    pub engine: String,
    #[serde(default)]
    pub engine_version: String,
    #[serde(default)]
    pub engine_tick: u64,
    #[serde(default)]
    pub scene_name: Option<String>,
    #[serde(default)]
    pub spatial_pos: Option<[f32; 3]>,
    #[serde(default)]
    pub player_rot: Option<[f32; 3]>,
    #[serde(default)]
    pub breadcrumbs: Vec<Breadcrumb>,
    #[serde(default)]
    pub replay_id: Option<String>,
    #[serde(default)]
    pub hardware_info: serde_json::Value,
    #[serde(default)]
    pub release_version: Option<String>,
}

fn default_severity() -> String {
    "error".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrashRecord {
    pub id: i64,
    pub project_id: String,
    pub fingerprint: String,
    pub exception_type: String,
    pub message: String,
    pub source_file: Option<String>,
    pub line_no: Option<u32>,
    pub severity: String,
    pub platform: String,
    pub engine: String,
    pub engine_version: String,
    pub scene_name: Option<String>,
    pub spatial_x: Option<f32>,
    pub spatial_y: Option<f32>,
    pub spatial_z: Option<f32>,
    pub stacktrace_json: String,
    pub breadcrumbs_json: String,
    pub replay_id: Option<String>,
    pub hardware_json: String,
    pub occurrences: u32,
    pub first_seen: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct InputFrame {
    pub tick: u32,
    pub digital_buttons: u32,
    pub axis_lx: i8,
    pub axis_ly: i8,
    pub axis_rx: i8,
    pub axis_ry: i8,
    pub mouse_delta_x: i16,
    pub mouse_delta_y: i16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayData {
    pub id: String,
    pub project_id: String,
    #[serde(default)]
    pub crash_id: Option<i64>,
    pub duration_secs: f32,
    pub tick_count: u32,
    pub scene_name: String,
    pub rng_seed: u64,
    #[serde(default)]
    pub initial_snapshot: serde_json::Value,
    pub input_stream_base64: String, // compressed InputFrames stream
    #[serde(default)]
    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShaderHitchEvent {
    pub project_id: String,
    pub shader_path: String,
    #[serde(default)]
    pub shader_uid: Option<String>,
    pub hitch_duration_ms: f32,
    pub gpu_renderer: String,
    #[serde(default)]
    pub render_pass: Option<String>,
    #[serde(default)]
    pub level_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShaderHitchRecord {
    pub id: i64,
    pub project_id: String,
    pub shader_path: String,
    pub shader_uid: Option<String>,
    pub max_hitch_ms: f32,
    pub avg_hitch_ms: f32,
    pub occurrences: u32,
    pub gpu_renderer: String,
    pub last_seen: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaytestFeedback {
    pub id: String,
    pub project_id: String,
    pub user_comment: String,
    #[serde(default = "default_feedback_category")]
    pub category: String, // "bug", "visual", "balance", "fun"
    #[serde(default)]
    pub screenshot_base64: Option<String>,
    #[serde(default)]
    pub replay_id: Option<String>,
    #[serde(default)]
    pub savefile_snapshot: Option<String>,
    #[serde(default)]
    pub scene_name: Option<String>,
    #[serde(default)]
    pub spatial_pos: Option<[f32; 3]>,
    #[serde(default)]
    pub user_system_info: serde_json::Value,
    #[serde(default)]
    pub created_at: Option<DateTime<Utc>>,
}

fn default_feedback_category() -> String {
    "bug".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveTweakMessage {
    pub msg_type: String, // "register", "tweak", "command", "sync", "ack"
    pub param_path: Option<String>,
    pub value: Option<serde_json::Value>,
    pub meta: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TweakProperty {
    pub path: String,
    pub value: serde_json::Value,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub step: Option<f64>,
    pub prop_type: String, // "float", "int", "bool", "string"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpatialCluster {
    pub scene_name: String,
    pub centroid: [f32; 3],
    pub count: u32,
    pub sample_error: String,
    pub sample_crash_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrphanAsset {
    pub path: String,
    pub size_bytes: u64,
    pub extension: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetViolation {
    pub path: String,
    pub rule: String,
    pub actual: String,
    pub limit: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetAuditReport {
    pub total_files: usize,
    pub reachable_files: usize,
    pub orphan_files: Vec<OrphanAsset>,
    pub total_orphan_bytes: u64,
    pub budget_violations: Vec<BudgetViolation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CasStats {
    pub total_objects: u64,
    pub total_original_bytes: u64,
    pub total_compressed_bytes: u64,
    pub compression_ratio: f32,
}
