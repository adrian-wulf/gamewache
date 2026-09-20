use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use crate::models::{AssetAuditReport, BudgetViolation, OrphanAsset};

pub struct AssetAuditor;

impl AssetAuditor {
    pub fn audit(project_dir: &Path) -> std::io::Result<AssetAuditReport> {
        let mut all_files = Vec::new();
        let mut asset_files = Vec::new();
        let mut root_scenes = HashSet::new();

        // 1. Wykryj scenę startową w Godot (project.godot)
        let project_godot = project_dir.join("project.godot");
        if project_godot.exists() {
            if let Ok(content) = fs::read_to_string(&project_godot) {
                for line in content.lines() {
                    if line.contains("run/main_scene=") {
                        let parts: Vec<&str> = line.split('=').collect();
                        if parts.len() > 1 {
                            let scene = parts[1].trim().trim_matches('"');
                            root_scenes.insert(scene.to_string());
                        }
                    }
                }
            }
        }

        // 2. Przeszukaj rekurencyjnie katalog projektu
        Self::collect_files(project_dir, project_dir, &mut all_files, &mut asset_files)?;

        // 3. Buduj graf osiągalności - zbadaj zawartość wszystkich plików tekstowych
        let mut referenced_paths = HashSet::new();

        // Dodaj sceny główne
        for r in &root_scenes {
            referenced_paths.insert(r.clone());
        }

        // Przeszukaj każdy skrypt, scenę i materiał w poszukiwaniu referencji "res://" lub nazw plików
        for file in &all_files {
            if Self::is_referencing_file(file) {
                if let Ok(content) = fs::read_to_string(file) {
                    Self::extract_references(&content, &mut referenced_paths);
                }
            }
        }

        // Propaguj osiągalność: jeśli plik jest w referenced_paths i sam jest sceną/materiałem,
        // jego wewnętrzne referencje są już w referenced_paths, bo powyższa pętla przeskanowała wszystko.

        let mut reachable_count = 0;
        let mut orphan_files = Vec::new();
        let mut total_orphan_bytes = 0;
        let mut budget_violations = Vec::new();

        for asset in &asset_files {
            let rel_path_godot = format!("res://{}", asset.replace('\\', "/"));
            let filename = Path::new(asset).file_name().and_then(|f| f.to_str()).unwrap_or("");

            let is_referenced = referenced_paths.contains(&rel_path_godot)
                || referenced_paths.contains(asset)
                || referenced_paths.contains(filename);

            let full_path = project_dir.join(asset);
            let size = fs::metadata(&full_path).map(|m| m.len()).unwrap_or(0);

            if is_referenced {
                reachable_count += 1;
            } else {
                let ext = Path::new(asset)
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("")
                    .to_lowercase();

                orphan_files.push(OrphanAsset {
                    path: asset.clone(),
                    size_bytes: size,
                    extension: ext.clone(),
                });
                total_orphan_bytes += size;
            }

            // Sprawdź limity budżetu
            // Reguła 1: Zbyt ciężkie tekstury (> 8 MB)
            if (asset.ends_with(".png") || asset.ends_with(".tga") || asset.ends_with(".jpg"))
                && size > 8 * 1024 * 1024
            {
                budget_violations.push(BudgetViolation {
                    path: asset.clone(),
                    rule: "max_texture_file_size".to_string(),
                    actual: format!("{:.2} MB", size as f64 / (1024.0 * 1024.0)),
                    limit: "8.00 MB".to_string(),
                });
            }

            // Reguła 2: Nieskompresowane pliki audio WAV > 2 MB
            if asset.ends_with(".wav") && size > 2 * 1024 * 1024 {
                budget_violations.push(BudgetViolation {
                    path: asset.clone(),
                    rule: "uncompressed_audio_size (Use .ogg or streaming)".to_string(),
                    actual: format!("{:.2} MB", size as f64 / (1024.0 * 1024.0)),
                    limit: "2.00 MB".to_string(),
                });
            }
        }

        orphan_files.sort_by(|a, b| b.size_bytes.cmp(&a.size_bytes));

        Ok(AssetAuditReport {
            total_files: asset_files.len(),
            reachable_files: reachable_count,
            orphan_files,
            total_orphan_bytes,
            budget_violations,
        })
    }

    fn collect_files(
        root: &Path,
        current: &Path,
        all_files: &mut Vec<PathBuf>,
        asset_files: &mut Vec<String>,
    ) -> std::io::Result<()> {
        if !current.is_dir() {
            return Ok(());
        }

        for entry in fs::read_dir(current)? {
            let entry = entry?;
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();

            // Pomiń katalogi wewnętrzne i ukryte
            if name.starts_with('.') || name == "target" || name == "node_modules" || name == "Library"
            {
                continue;
            }

            if path.is_dir() {
                Self::collect_files(root, &path, all_files, asset_files)?;
            } else if path.is_file() {
                all_files.push(path.clone());

                if let Ok(rel) = path.strip_prefix(root) {
                    let rel_str = rel.to_string_lossy().to_string();
                    if Self::is_asset(&path) {
                        asset_files.push(rel_str);
                    }
                }
            }
        }
        Ok(())
    }

    fn is_asset(path: &Path) -> bool {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        matches!(
            ext.as_str(),
            "png" | "jpg" | "jpeg" | "webp" | "svg" | "tga" | "wav" | "ogg" | "mp3" | "gltf" | "glb"
                | "fbx" | "blend" | "tres" | "material" | "gdshader"
        )
    }

    fn is_referencing_file(path: &Path) -> bool {
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        matches!(
            ext.as_str(),
            "tscn" | "tres" | "gd" | "cs" | "prefab" | "unity" | "mat" | "asset" | "cfg" | "json"
        )
    }

    fn extract_references(content: &str, set: &mut HashSet<String>) {
        for line in content.lines() {
            // Szukaj ścieżek res://
            if let Some(start_idx) = line.find("res://") {
                let rest = &line[start_idx..];
                let end_idx = rest
                    .find(|c| c == '"' || c == '\'' || c == ' ' || c == '\n' || c == '\r')
                    .unwrap_or(rest.len());
                let res_path = &rest[..end_idx];
                if !res_path.is_empty() {
                    set.insert(res_path.to_string());
                }
            }

            // Szukaj UID: uid://...
            if let Some(start_idx) = line.find("uid://") {
                let rest = &line[start_idx..];
                let end_idx = rest
                    .find(|c| c == '"' || c == '\'' || c == ' ' || c == '\n' || c == '\r')
                    .unwrap_or(rest.len());
                let uid_path = &rest[..end_idx];
                if !uid_path.is_empty() {
                    set.insert(uid_path.to_string());
                }
            }
        }
    }

    pub fn prune_orphans(project_dir: &Path, dry_run: bool) -> std::io::Result<usize> {
        let report = Self::audit(project_dir)?;
        let quarantine_dir = project_dir.join(".gamewache").join("quarantine");

        if !dry_run && !report.orphan_files.is_empty() {
            fs::create_dir_all(&quarantine_dir)?;
        }

        let mut count = 0;
        for orphan in &report.orphan_files {
            let src = project_dir.join(&orphan.path);
            if src.exists() {
                count += 1;
                if !dry_run {
                    let dest = quarantine_dir.join(&orphan.path);
                    if let Some(parent) = dest.parent() {
                        fs::create_dir_all(parent)?;
                    }
                    fs::rename(&src, &dest)?;
                }
            }
        }

        Ok(count)
    }
}
