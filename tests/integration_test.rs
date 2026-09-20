use std::fs;

use gamewache::asset_auditor::AssetAuditor;
use gamewache::cas::CasEngine;
use gamewache::db::Database;
use gamewache::live_tweaker::LiveTweakerHub;
use gamewache::models::{CrashReport, PlaytestFeedback, ReplayData, ShaderHitchEvent};
use gamewache::shader_prewarm::ShaderPrewarmGenerator;

#[test]
fn test_database_and_crashes() {
    let temp_dir = std::env::temp_dir().join(format!("gw_test_db_{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&temp_dir).unwrap();
    let db_path = temp_dir.join("test.db");

    let db = Database::new(&db_path).unwrap();

    let report1 = CrashReport {
        project_id: "test-game".to_string(),
        exception_type: "NullInstance".to_string(),
        message: "Invalid call on null instance 'turret'".to_string(),
        stacktrace: vec![],
        source_file: Some("res://scripts/tank.gd".to_string()),
        line_no: Some(84),
        severity: "fatal".to_string(),
        platform: "linux".to_string(),
        engine: "godot".to_string(),
        engine_version: "4.3.0".to_string(),
        engine_tick: 4200,
        scene_name: Some("res://scenes/level_01.tscn".to_string()),
        spatial_pos: Some([10.5, 0.0, -25.2]),
        player_rot: None,
        breadcrumbs: vec![],
        replay_id: Some("rec_test_1".to_string()),
        hardware_info: serde_json::json!({"gpu": "RTX 3080"}),
        release_version: Some("v0.1.0".to_string()),
    };

    let id1 = db.record_crash(&report1).unwrap();
    assert!(id1 > 0);

    // Drugi crash z tym samym fingerprintem powinien zwiększyć licznik occurrences
    let id2 = db.record_crash(&report1).unwrap();
    assert_eq!(id1, id2);

    let crashes = db.list_crashes("test-game", None, 10).unwrap();
    assert_eq!(crashes.len(), 1);
    assert_eq!(crashes[0].occurrences, 2);
    assert_eq!(crashes[0].spatial_x, Some(10.5));

    // Spatial clusters
    let clusters = db.get_spatial_clusters("test-game", "res://scenes/level_01.tscn").unwrap();
    assert_eq!(clusters.len(), 1);
    assert_eq!(clusters[0].count, 2);

    let _ = fs::remove_dir_all(temp_dir);
}

#[test]
fn test_replays_and_feedback() {
    let temp_dir = std::env::temp_dir().join(format!("gw_test_rep_{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&temp_dir).unwrap();
    let db = Database::new(temp_dir.join("test.db")).unwrap();

    let replay = ReplayData {
        id: "rec_123".to_string(),
        project_id: "test-game".to_string(),
        crash_id: Some(1),
        duration_secs: 15.0,
        tick_count: 900,
        scene_name: "res://level.tscn".to_string(),
        rng_seed: 428192,
        initial_snapshot: serde_json::json!({"hp": 100}),
        input_stream_base64: "W10=".to_string(),
        created_at: None,
    };

    db.save_replay(&replay).unwrap();
    let loaded = db.get_replay("rec_123").unwrap().expect("Replay should exist");
    assert_eq!(loaded.rng_seed, 428192);
    assert_eq!(loaded.duration_secs, 15.0);

    let feedback = PlaytestFeedback {
        id: "fb_001".to_string(),
        project_id: "test-game".to_string(),
        user_comment: "Boss stuck in wall".to_string(),
        category: "bug".to_string(),
        screenshot_base64: None,
        replay_id: Some("rec_123".to_string()),
        savefile_snapshot: None,
        scene_name: Some("res://level.tscn".to_string()),
        spatial_pos: Some([50.0, 0.0, 10.0]),
        user_system_info: serde_json::json!({"fps": 60}),
        created_at: None,
    };

    db.save_feedback(&feedback).unwrap();
    let list = db.list_feedback("test-game", 10).unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].user_comment, "Boss stuck in wall");

    let _ = fs::remove_dir_all(temp_dir);
}

#[test]
fn test_cas_engine() {
    let temp_dir = std::env::temp_dir().join(format!("gw_test_cas_{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&temp_dir).unwrap();

    let db = Database::new(temp_dir.join("db.sqlite")).unwrap();
    let cas = CasEngine::new(&temp_dir, db).unwrap();

    let sample_texture_data = b"RIFF....WAVEfmt ....data...FAKE_AUDIO_DATA_FOR_TESTING_CAS_ENGINE";
    let (hash, orig_size, comp_size) = cas.put(sample_texture_data, "audio/wav", "godot").unwrap();

    assert_eq!(orig_size, sample_texture_data.len() as u64);
    assert!(comp_size > 0);

    let decompressed = cas.get(&hash).unwrap();
    assert_eq!(decompressed, sample_texture_data);

    let stats = cas.stats().unwrap();
    assert_eq!(stats.total_objects, 1);
    assert_eq!(stats.total_original_bytes, orig_size);

    let _ = fs::remove_dir_all(temp_dir);
}

#[tokio::test]
async fn test_live_tweaker() {
    let tweaker = LiveTweakerHub::new();

    tweaker
        .tweak_var("Combat/TurretSpeed", serde_json::json!(8.5))
        .await;

    let tweakables = tweaker.get_tweakables().await;
    assert_eq!(tweakables.len(), 1);
    assert_eq!(tweakables[0].path, "Combat/TurretSpeed");
    assert_eq!(tweakables[0].value, serde_json::json!(8.5));
}

#[test]
fn test_shader_prewarm_generation() {
    let temp_dir = std::env::temp_dir().join(format!("gw_test_shd_{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&temp_dir).unwrap();
    let db = Database::new(temp_dir.join("db.sqlite")).unwrap();

    let hitch = ShaderHitchEvent {
        project_id: "test-game".to_string(),
        shader_path: "res://shaders/laser.gdshader".to_string(),
        shader_uid: Some("uid://laser_01".to_string()),
        hitch_duration_ms: 120.5,
        gpu_renderer: "RTX 3070".to_string(),
        render_pass: Some("Forward+".to_string()),
        level_name: Some("level_01".to_string()),
    };

    db.record_shader_hitch(&hitch).unwrap();

    let generator = ShaderPrewarmGenerator::new(db);
    let gdscript = generator.generate_godot_script("test-game").unwrap();
    assert!(gdscript.contains("res://shaders/laser.gdshader"));
    assert!(gdscript.contains("execute_prewarm"));

    let csharp = generator.generate_unity_script("test-game").unwrap();
    assert!(csharp.contains("res://shaders/laser.gdshader"));
    assert!(csharp.contains("ShaderPrewarmer"));

    let _ = fs::remove_dir_all(temp_dir);
}

#[test]
fn test_asset_auditor() {
    let temp_dir = std::env::temp_dir().join(format!("gw_test_audit_{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&temp_dir).unwrap();

    // Stwórz strukturę projektu Godot
    fs::write(temp_dir.join("project.godot"), "run/main_scene=\"res://main.tscn\"\n").unwrap();
    fs::write(
        temp_dir.join("main.tscn"),
        "[ext_resource type=\"Texture2D\" path=\"res://active.png\" id=\"1_abc\"]\n",
    )
    .unwrap();
    fs::write(temp_dir.join("active.png"), b"ACTIVE_TEXTURE_PNG").unwrap();
    fs::write(temp_dir.join("orphan_heavy.png"), vec![0u8; 1000]).unwrap();

    let report = AssetAuditor::audit(&temp_dir).unwrap();
    assert_eq!(report.total_files, 2);
    assert_eq!(report.reachable_files, 1);
    assert_eq!(report.orphan_files.len(), 1);
    assert_eq!(report.orphan_files[0].path, "orphan_heavy.png");

    let _ = fs::remove_dir_all(temp_dir);
}
