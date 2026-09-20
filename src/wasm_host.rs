use std::fs;
use std::path::{Path, PathBuf};

use axum::body::Body;
use axum::extract::{Path as AxumPath, Query, State};
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use serde::Deserialize;

use crate::db::Database;

#[derive(Clone)]
pub struct WasmHost {
    storage_dir: PathBuf,
    db: Database,
}

#[derive(Deserialize)]
pub struct PlayQuery {
    pub t: Option<String>,
}

impl WasmHost {
    pub fn new<P: AsRef<Path>>(storage_dir: P, db: Database) -> Self {
        let dir = storage_dir.as_ref().to_path_buf();
        let _ = fs::create_dir_all(&dir);
        Self {
            storage_dir: dir,
            db,
        }
    }

    pub fn deploy_build(
        &self,
        project_id: &str,
        channel: &str,
        source_dir: &Path,
    ) -> std::io::Result<String> {
        let target_dir = self.storage_dir.join(project_id).join(channel);
        if target_dir.exists() {
            let _ = fs::remove_dir_all(&target_dir);
        }
        fs::create_dir_all(&target_dir)?;

        Self::copy_recursive(source_dir, &target_dir)?;

        // Wygeneruj unikalny token dostępowy
        let token = format!(
            "gw_{}_{}",
            channel,
            uuid::Uuid::new_v4().to_string().replace('-', "")[..12].to_string()
        );
        let expires_at = chrono::Utc::now() + chrono::Duration::days(7);

        let _ = self
            .db
            .create_playtest_token(&token, project_id, channel, expires_at, 50);

        Ok(token)
    }

    fn copy_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
        for entry in fs::read_dir(src)? {
            let entry = entry?;
            let path = entry.path();
            let dest_path = dst.join(entry.file_name());
            if path.is_dir() {
                fs::create_dir_all(&dest_path)?;
                Self::copy_recursive(&path, &dest_path)?;
            } else {
                fs::copy(&path, &dest_path)?;
            }
        }
        Ok(())
    }

    pub async fn handle_play(
        State(host): State<WasmHost>,
        AxumPath((project_id, channel, file)): AxumPath<(String, String, String)>,
        Query(query): Query<PlayQuery>,
    ) -> Response {
        // Jeśli żądany jest plik główny (index.html lub brak pliku), sprawdź token
        let is_root = file.is_empty() || file == "index.html";
        if is_root {
            if let Some(token) = query.t {
                if host.db.validate_and_use_playtest_token(&token).is_err() {
                    return (StatusCode::FORBIDDEN, "Invalid or expired playtest token")
                        .into_response();
                }
            }
        }

        let file_name = if file.is_empty() { "index.html" } else { &file };
        let file_path = host
            .storage_dir
            .join(&project_id)
            .join(&channel)
            .join(file_name);

        if !file_path.exists() || !file_path.is_file() {
            return (StatusCode::NOT_FOUND, "Build asset not found").into_response();
        }

        let content = match fs::read(&file_path) {
            Ok(c) => c,
            Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "Error reading file").into_response(),
        };

        let mime = mime_guess::from_path(&file_path)
            .first_or_octet_stream()
            .to_string();

        let mut headers = HeaderMap::new();
        headers.insert(header::CONTENT_TYPE, HeaderValue::from_str(&mime).unwrap());

        // KRYTYCZNE DLA GODOT 4 I UNITY WEB (WASM MULTITHREADING):
        headers.insert(
            header::HeaderName::from_static("cross-origin-opener-policy"),
            HeaderValue::from_static("same-origin"),
        );
        headers.insert(
            header::HeaderName::from_static("cross-origin-embedder-policy"),
            HeaderValue::from_static("require-corp"),
        );

        if file_name.ends_with(".wasm") || file_name.ends_with(".pck") {
            headers.insert(
                header::CACHE_CONTROL,
                HeaderValue::from_static("public, max-age=86400"),
            );
        } else {
            headers.insert(
                header::CACHE_CONTROL,
                HeaderValue::from_static("no-cache, no-store, must-revalidate"),
            );
        }

        (StatusCode::OK, headers, Body::from(content)).into_response()
    }
}
