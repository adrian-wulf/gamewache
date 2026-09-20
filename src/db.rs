use std::path::Path;
use std::sync::{Arc, Mutex};

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, Result};

use crate::models::{
    CrashRecord, CrashReport, PlaytestFeedback, ReplayData, ShaderHitchEvent,
    ShaderHitchRecord, SpatialCluster,
};

#[derive(Clone)]
pub struct Database {
    conn: Arc<Mutex<Connection>>,
}

impl Database {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let conn = Connection::open(path)?;

        // Optymalizacje SQLite w trybie WAL
        conn.execute_batch(
            r#"
            PRAGMA journal_mode = WAL;
            PRAGMA synchronous = NORMAL;
            PRAGMA foreign_keys = ON;
            PRAGMA busy_timeout = 5000;
            "#,
        )?;

        let db = Self {
            conn: Arc::new(Mutex::new(conn)),
        };
        db.init_schema()?;
        Ok(db)
    }

    #[allow(dead_code)]
    pub fn in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        let db = Self {
            conn: Arc::new(Mutex::new(conn)),
        };
        db.init_schema()?;
        Ok(db)
    }

    fn init_schema(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS projects (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                engine TEXT NOT NULL,
                created_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS crashes (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                project_id TEXT NOT NULL,
                fingerprint TEXT NOT NULL,
                exception_type TEXT NOT NULL,
                message TEXT NOT NULL,
                source_file TEXT,
                line_no INTEGER,
                severity TEXT NOT NULL,
                platform TEXT NOT NULL,
                engine TEXT NOT NULL,
                engine_version TEXT NOT NULL,
                scene_name TEXT,
                spatial_x REAL,
                spatial_y REAL,
                spatial_z REAL,
                stacktrace_json TEXT NOT NULL,
                breadcrumbs_json TEXT NOT NULL,
                replay_id TEXT,
                hardware_json TEXT NOT NULL,
                occurrences INTEGER NOT NULL DEFAULT 1,
                first_seen TEXT NOT NULL,
                last_seen TEXT NOT NULL,
                UNIQUE(project_id, fingerprint)
            );

            CREATE INDEX IF NOT EXISTS idx_crashes_project ON crashes(project_id);
            CREATE INDEX IF NOT EXISTS idx_crashes_scene ON crashes(scene_name);
            CREATE INDEX IF NOT EXISTS idx_crashes_last_seen ON crashes(last_seen);

            CREATE TABLE IF NOT EXISTS replays (
                id TEXT PRIMARY KEY,
                project_id TEXT NOT NULL,
                crash_id INTEGER,
                duration_secs REAL NOT NULL,
                tick_count INTEGER NOT NULL,
                scene_name TEXT NOT NULL,
                rng_seed INTEGER NOT NULL,
                initial_snapshot TEXT NOT NULL,
                input_stream TEXT NOT NULL,
                created_at TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_replays_project ON replays(project_id);

            CREATE TABLE IF NOT EXISTS shader_hitches (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                project_id TEXT NOT NULL,
                shader_path TEXT NOT NULL,
                shader_uid TEXT,
                max_hitch_ms REAL NOT NULL,
                avg_hitch_ms REAL NOT NULL,
                occurrences INTEGER NOT NULL DEFAULT 1,
                gpu_renderer TEXT NOT NULL,
                last_seen TEXT NOT NULL,
                UNIQUE(project_id, shader_path, gpu_renderer)
            );

            CREATE INDEX IF NOT EXISTS idx_shaders_project ON shader_hitches(project_id);

            CREATE TABLE IF NOT EXISTS playtest_feedback (
                id TEXT PRIMARY KEY,
                project_id TEXT NOT NULL,
                user_comment TEXT NOT NULL,
                category TEXT NOT NULL,
                screenshot_base64 TEXT,
                replay_id TEXT,
                savefile_snapshot TEXT,
                scene_name TEXT,
                spatial_x REAL,
                spatial_y REAL,
                spatial_z REAL,
                user_system_info TEXT NOT NULL,
                created_at TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_feedback_project ON playtest_feedback(project_id);

            CREATE TABLE IF NOT EXISTS cas_objects (
                hash TEXT PRIMARY KEY,
                original_size INTEGER NOT NULL,
                compressed_size INTEGER NOT NULL,
                mime_type TEXT NOT NULL,
                engine TEXT NOT NULL,
                created_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS web_playtest_tokens (
                token TEXT PRIMARY KEY,
                project_id TEXT NOT NULL,
                channel TEXT NOT NULL,
                created_at TEXT NOT NULL,
                expires_at TEXT NOT NULL,
                max_uses INTEGER NOT NULL,
                uses_count INTEGER NOT NULL DEFAULT 0
            );
            "#,
        )?;

        Ok(())
    }

    pub fn record_crash(&self, report: &CrashReport) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();

        // Fingerprint oparty o typ wyjątku, plik i linię
        let fingerprint = format!(
            "{}:{}:{}",
            report.exception_type,
            report.source_file.as_deref().unwrap_or("unknown"),
            report.line_no.unwrap_or(0)
        );

        let stacktrace_json = serde_json::to_string(&report.stacktrace).unwrap_or_default();
        let breadcrumbs_json = serde_json::to_string(&report.breadcrumbs).unwrap_or_default();
        let hardware_json = serde_json::to_string(&report.hardware_info).unwrap_or_default();

        let (sx, sy, sz) = match report.spatial_pos {
            Some([x, y, z]) => (Some(x), Some(y), Some(z)),
            None => (None, None, None),
        };

        conn.execute(
            r#"
            INSERT INTO crashes (
                project_id, fingerprint, exception_type, message, source_file, line_no,
                severity, platform, engine, engine_version, scene_name,
                spatial_x, spatial_y, spatial_z, stacktrace_json, breadcrumbs_json,
                replay_id, hardware_json, occurrences, first_seen, last_seen
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, 1, ?19, ?19)
            ON CONFLICT(project_id, fingerprint) DO UPDATE SET
                occurrences = occurrences + 1,
                last_seen = ?19,
                message = ?4,
                stacktrace_json = ?15,
                breadcrumbs_json = ?16,
                replay_id = COALESCE(?17, replay_id),
                scene_name = COALESCE(?11, scene_name),
                spatial_x = COALESCE(?12, spatial_x),
                spatial_y = COALESCE(?13, spatial_y),
                spatial_z = COALESCE(?14, spatial_z)
            "#,
            params![
                report.project_id,
                fingerprint,
                report.exception_type,
                report.message,
                report.source_file,
                report.line_no,
                report.severity,
                report.platform,
                report.engine,
                report.engine_version,
                report.scene_name,
                sx,
                sy,
                sz,
                stacktrace_json,
                breadcrumbs_json,
                report.replay_id,
                hardware_json,
                now
            ],
        )?;

        // Pobierz id wstawionego lub zaktualizowanego rekordu
        let id: i64 = conn.query_row(
            "SELECT id FROM crashes WHERE project_id = ?1 AND fingerprint = ?2",
            params![report.project_id, fingerprint],
            |row| row.get(0),
        )?;

        Ok(id)
    }

    pub fn list_crashes(
        &self,
        project_id: &str,
        scene_name: Option<&str>,
        limit: u32,
    ) -> Result<Vec<CrashRecord>> {
        let conn = self.conn.lock().unwrap();

        let (query, is_filtered) = if scene_name.is_some() {
            (
                "SELECT id, project_id, fingerprint, exception_type, message, source_file, line_no, severity, platform, engine, engine_version, scene_name, spatial_x, spatial_y, spatial_z, stacktrace_json, breadcrumbs_json, replay_id, hardware_json, occurrences, first_seen, last_seen FROM crashes WHERE project_id = ?1 AND scene_name = ?2 ORDER BY last_seen DESC LIMIT ?3",
                true,
            )
        } else {
            (
                "SELECT id, project_id, fingerprint, exception_type, message, source_file, line_no, severity, platform, engine, engine_version, scene_name, spatial_x, spatial_y, spatial_z, stacktrace_json, breadcrumbs_json, replay_id, hardware_json, occurrences, first_seen, last_seen FROM crashes WHERE project_id = ?1 ORDER BY last_seen DESC LIMIT ?2",
                false,
            )
        };

        let mut stmt = conn.prepare(query)?;
        let limit_param = limit as i64;

        let rows = if is_filtered {
            stmt.query_map(params![project_id, scene_name.unwrap(), limit_param], Self::map_crash_row)?
        } else {
            stmt.query_map(params![project_id, limit_param], Self::map_crash_row)?
        };

        let mut results = Vec::new();
        for r in rows {
            results.push(r?);
        }
        Ok(results)
    }

    pub fn get_crash(&self, id: i64) -> Result<Option<CrashRecord>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            r#"
            SELECT id, project_id, fingerprint, exception_type, message, source_file, line_no,
                   severity, platform, engine, engine_version, scene_name, spatial_x, spatial_y, spatial_z,
                   stacktrace_json, breadcrumbs_json, replay_id, hardware_json, occurrences, first_seen, last_seen
            FROM crashes WHERE id = ?1
            "#,
        )?;

        let mut rows = stmt.query_map(params![id], Self::map_crash_row)?;
        match rows.next() {
            Some(res) => Ok(Some(res?)),
            None => Ok(None),
        }
    }

    fn map_crash_row(row: &rusqlite::Row) -> rusqlite::Result<CrashRecord> {
        let first_seen_str: String = row.get(20)?;
        let last_seen_str: String = row.get(21)?;

        Ok(CrashRecord {
            id: row.get(0)?,
            project_id: row.get(1)?,
            fingerprint: row.get(2)?,
            exception_type: row.get(3)?,
            message: row.get(4)?,
            source_file: row.get(5)?,
            line_no: row.get(6)?,
            severity: row.get(7)?,
            platform: row.get(8)?,
            engine: row.get(9)?,
            engine_version: row.get(10)?,
            scene_name: row.get(11)?,
            spatial_x: row.get(12)?,
            spatial_y: row.get(13)?,
            spatial_z: row.get(14)?,
            stacktrace_json: row.get(15)?,
            breadcrumbs_json: row.get(16)?,
            replay_id: row.get(17)?,
            hardware_json: row.get(18)?,
            occurrences: row.get(19)?,
            first_seen: DateTime::parse_from_rfc3339(&first_seen_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
            last_seen: DateTime::parse_from_rfc3339(&last_seen_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
        })
    }

    pub fn save_replay(&self, replay: &ReplayData) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();
        let snapshot_json = serde_json::to_string(&replay.initial_snapshot).unwrap_or_default();

        conn.execute(
            r#"
            INSERT INTO replays (
                id, project_id, crash_id, duration_secs, tick_count, scene_name,
                rng_seed, initial_snapshot, input_stream, created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
            ON CONFLICT(id) DO UPDATE SET
                input_stream = ?9,
                tick_count = ?5
            "#,
            params![
                replay.id,
                replay.project_id,
                replay.crash_id,
                replay.duration_secs,
                replay.tick_count,
                replay.scene_name,
                replay.rng_seed as i64,
                snapshot_json,
                replay.input_stream_base64,
                now
            ],
        )?;
        Ok(())
    }

    pub fn get_replay(&self, id: &str) -> Result<Option<ReplayData>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            r#"
            SELECT id, project_id, crash_id, duration_secs, tick_count, scene_name,
                   rng_seed, initial_snapshot, input_stream, created_at
            FROM replays WHERE id = ?1
            "#,
        )?;

        let mut rows = stmt.query_map(params![id], |row| {
            let snap_str: String = row.get(7)?;
            let snap: serde_json::Value = serde_json::from_str(&snap_str).unwrap_or_default();
            let created_str: String = row.get(9)?;
            let created_at = DateTime::parse_from_rfc3339(&created_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());

            Ok(ReplayData {
                id: row.get(0)?,
                project_id: row.get(1)?,
                crash_id: row.get(2)?,
                duration_secs: row.get(3)?,
                tick_count: row.get(4)?,
                scene_name: row.get(5)?,
                rng_seed: row.get::<_, i64>(6)? as u64,
                initial_snapshot: snap,
                input_stream_base64: row.get(8)?,
                created_at: Some(created_at),
            })
        })?;

        match rows.next() {
            Some(res) => Ok(Some(res?)),
            None => Ok(None),
        }
    }

    pub fn record_shader_hitch(&self, hitch: &ShaderHitchEvent) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();

        conn.execute(
            r#"
            INSERT INTO shader_hitches (
                project_id, shader_path, shader_uid, max_hitch_ms, avg_hitch_ms,
                occurrences, gpu_renderer, last_seen
            ) VALUES (?1, ?2, ?3, ?4, ?4, 1, ?5, ?6)
            ON CONFLICT(project_id, shader_path, gpu_renderer) DO UPDATE SET
                occurrences = occurrences + 1,
                max_hitch_ms = MAX(max_hitch_ms, ?4),
                avg_hitch_ms = (avg_hitch_ms * occurrences + ?4) / (occurrences + 1),
                last_seen = ?6
            "#,
            params![
                hitch.project_id,
                hitch.shader_path,
                hitch.shader_uid,
                hitch.hitch_duration_ms,
                hitch.gpu_renderer,
                now
            ],
        )?;
        Ok(())
    }

    pub fn list_shader_hitches(&self, project_id: &str) -> Result<Vec<ShaderHitchRecord>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            r#"
            SELECT id, project_id, shader_path, shader_uid, max_hitch_ms, avg_hitch_ms,
                   occurrences, gpu_renderer, last_seen
            FROM shader_hitches WHERE project_id = ?1
            ORDER BY max_hitch_ms DESC
            "#,
        )?;

        let rows = stmt.query_map(params![project_id], |row| {
            let last_seen_str: String = row.get(8)?;
            Ok(ShaderHitchRecord {
                id: row.get(0)?,
                project_id: row.get(1)?,
                shader_path: row.get(2)?,
                shader_uid: row.get(3)?,
                max_hitch_ms: row.get(4)?,
                avg_hitch_ms: row.get(5)?,
                occurrences: row.get(6)?,
                gpu_renderer: row.get(7)?,
                last_seen: DateTime::parse_from_rfc3339(&last_seen_str)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
            })
        })?;

        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    pub fn save_feedback(&self, feedback: &PlaytestFeedback) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();
        let sys_info_json = serde_json::to_string(&feedback.user_system_info).unwrap_or_default();

        let (sx, sy, sz) = match feedback.spatial_pos {
            Some([x, y, z]) => (Some(x), Some(y), Some(z)),
            None => (None, None, None),
        };

        conn.execute(
            r#"
            INSERT INTO playtest_feedback (
                id, project_id, user_comment, category, screenshot_base64, replay_id,
                savefile_snapshot, scene_name, spatial_x, spatial_y, spatial_z,
                user_system_info, created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
            "#,
            params![
                feedback.id,
                feedback.project_id,
                feedback.user_comment,
                feedback.category,
                feedback.screenshot_base64,
                feedback.replay_id,
                feedback.savefile_snapshot,
                feedback.scene_name,
                sx,
                sy,
                sz,
                sys_info_json,
                now
            ],
        )?;
        Ok(())
    }

    pub fn list_feedback(&self, project_id: &str, limit: u32) -> Result<Vec<PlaytestFeedback>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            r#"
            SELECT id, project_id, user_comment, category, screenshot_base64, replay_id,
                   savefile_snapshot, scene_name, spatial_x, spatial_y, spatial_z,
                   user_system_info, created_at
            FROM playtest_feedback WHERE project_id = ?1
            ORDER BY created_at DESC LIMIT ?2
            "#,
        )?;

        let rows = stmt.query_map(params![project_id, limit as i64], |row| {
            let sys_info_str: String = row.get(11)?;
            let sys_info: serde_json::Value =
                serde_json::from_str(&sys_info_str).unwrap_or_default();
            let created_str: String = row.get(12)?;
            let created_at = DateTime::parse_from_rfc3339(&created_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now());

            let sx: Option<f32> = row.get(8)?;
            let sy: Option<f32> = row.get(9)?;
            let sz: Option<f32> = row.get(10)?;
            let spatial_pos = match (sx, sy, sz) {
                (Some(x), Some(y), Some(z)) => Some([x, y, z]),
                _ => None,
            };

            Ok(PlaytestFeedback {
                id: row.get(0)?,
                project_id: row.get(1)?,
                user_comment: row.get(2)?,
                category: row.get(3)?,
                screenshot_base64: row.get(4)?,
                replay_id: row.get(5)?,
                savefile_snapshot: row.get(6)?,
                scene_name: row.get(7)?,
                spatial_pos,
                user_system_info: sys_info,
                created_at: Some(created_at),
            })
        })?;

        let mut list = Vec::new();
        for r in rows {
            list.push(r?);
        }
        Ok(list)
    }

    pub fn get_spatial_clusters(&self, project_id: &str, scene_name: &str) -> Result<Vec<SpatialCluster>> {
        let conn = self.conn.lock().unwrap();
        // Agregacja punktów w przestrzeni (siatka zaokrąglona do 2 jednostek silnika)
        let mut stmt = conn.prepare(
            r#"
            SELECT
                scene_name,
                ROUND(spatial_x / 2.0) * 2.0 as grid_x,
                ROUND(spatial_y / 2.0) * 2.0 as grid_y,
                ROUND(spatial_z / 2.0) * 2.0 as grid_z,
                CAST(SUM(occurrences) AS INTEGER) as count,
                message,
                id
            FROM crashes
            WHERE project_id = ?1 AND scene_name = ?2 AND spatial_x IS NOT NULL
            GROUP BY grid_x, grid_y, grid_z
            ORDER BY count DESC
            "#,
        )?;

        let rows = stmt.query_map(params![project_id, scene_name], |row| {
            let x: f32 = row.get(1)?;
            let y: f32 = row.get(2)?;
            let z: f32 = row.get(3)?;
            let count: u32 = row.get(4)?;
            let msg: String = row.get(5)?;
            let crash_id: i64 = row.get(6)?;

            Ok(SpatialCluster {
                scene_name: row.get(0)?,
                centroid: [x, y, z],
                count,
                sample_error: msg,
                sample_crash_id: crash_id,
            })
        })?;

        let mut clusters = Vec::new();
        for r in rows {
            clusters.push(r?);
        }
        Ok(clusters)
    }

    pub fn record_cas_object(
        &self,
        hash: &str,
        orig_size: u64,
        comp_size: u64,
        mime: &str,
        engine: &str,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();

        conn.execute(
            r#"
            INSERT INTO cas_objects (hash, original_size, compressed_size, mime_type, engine, created_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            ON CONFLICT(hash) DO NOTHING
            "#,
            params![hash, orig_size as i64, comp_size as i64, mime, engine, now],
        )?;
        Ok(())
    }

    pub fn get_cas_stats(&self) -> Result<crate::models::CasStats> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT COUNT(*), COALESCE(SUM(original_size), 0), COALESCE(SUM(compressed_size), 0) FROM cas_objects",
        )?;

        stmt.query_row([], |row| {
            let total_objects: i64 = row.get(0)?;
            let orig: i64 = row.get(1)?;
            let comp: i64 = row.get(2)?;

            let ratio = if orig > 0 {
                (comp as f32) / (orig as f32)
            } else {
                1.0
            };

            Ok(crate::models::CasStats {
                total_objects: total_objects as u64,
                total_original_bytes: orig as u64,
                total_compressed_bytes: comp as u64,
                compression_ratio: ratio,
            })
        })
    }

    pub fn create_playtest_token(
        &self,
        token: &str,
        project_id: &str,
        channel: &str,
        expires_at: DateTime<Utc>,
        max_uses: i32,
    ) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();
        conn.execute(
            r#"
            INSERT INTO web_playtest_tokens (token, project_id, channel, created_at, expires_at, max_uses, uses_count)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0)
            "#,
            params![token, project_id, channel, now, expires_at.to_rfc3339(), max_uses],
        )?;
        Ok(())
    }

    pub fn validate_and_use_playtest_token(&self, token: &str) -> Result<Option<String>> {
        let conn = self.conn.lock().unwrap();
        let now = Utc::now().to_rfc3339();

        let mut stmt = conn.prepare(
            r#"
            SELECT project_id, expires_at, max_uses, uses_count
            FROM web_playtest_tokens WHERE token = ?1
            "#,
        )?;

        let res = stmt.query_row(params![token], |row| {
            let pid: String = row.get(0)?;
            let exp_str: String = row.get(1)?;
            let max_u: i32 = row.get(2)?;
            let uses: i32 = row.get(3)?;
            Ok((pid, exp_str, max_u, uses))
        });

        match res {
            Ok((pid, exp_str, max_u, uses)) => {
                if exp_str < now {
                    return Ok(None); // Wygasł
                }
                if max_u > 0 && uses >= max_u {
                    return Ok(None); // Przekroczono limit
                }
                conn.execute(
                    "UPDATE web_playtest_tokens SET uses_count = uses_count + 1 WHERE token = ?1",
                    params![token],
                )?;
                Ok(Some(pid))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }
}
