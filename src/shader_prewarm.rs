use std::fs;
use std::path::Path;

use crate::db::Database;

pub struct ShaderPrewarmGenerator {
    db: Database,
}

impl ShaderPrewarmGenerator {
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    pub fn generate_godot_script(&self, project_id: &str) -> rusqlite::Result<String> {
        let hitches = self.db.list_shader_hitches(project_id)?;

        let mut shader_paths: Vec<String> = hitches
            .into_iter()
            .map(|h| format!("    \"{}\",", h.shader_path))
            .collect();
        shader_paths.dedup();

        let list_str = if shader_paths.is_empty() {
            "    # Brak zarejestrowanych shaderów powodujących zacięcia\n".to_string()
        } else {
            shader_paths.join("\n")
        };

        let script = format!(
            r#"# res://addons/gamewache/generated/shader_prewarm.gd
# WYGENEROWANE AUTOMATYCZNIE PRZEZ GAMEWACHE
# Ten skrypt wgrzewa potoki GPU (PSO) podczas ekranu ładowania, eliminując zacięcia (hitch) w grze.
extends Node

const WARMUP_SHADERS: Array[String] = [
{list_str}
]

func execute_prewarm(progress_callback: Callable = Callable()) -> void:
    if WARMUP_SHADERS.is_empty():
        return

    var canvas_layer = CanvasLayer.new()
    canvas_layer.layer = -100 # Niewidoczny dla gracza
    add_child(canvas_layer)

    var total = WARMUP_SHADERS.size()
    for i in range(total):
        var path = WARMUP_SHADERS[i]
        if ResourceLoader.exists(path):
            var res = load(path)
            if res is Shader:
                var mat = ShaderMaterial.new()
                mat.shader = res
                var rect = ColorRect.new()
                rect.material = mat
                rect.size = Vector2(1, 1)
                canvas_layer.add_child(rect)

        if progress_callback.is_valid():
            progress_callback.call(float(i + 1) / float(total))

    # Odczekaj 1 klatkę renderowania na skompilowanie przez sterownik GPU
    await get_tree().process_frame
    canvas_layer.queue_free()
"#
        );

        Ok(script)
    }

    pub fn generate_unity_script(&self, project_id: &str) -> rusqlite::Result<String> {
        let hitches = self.db.list_shader_hitches(project_id)?;

        let mut shader_names: Vec<String> = hitches
            .into_iter()
            .map(|h| format!("        \"{}\",", h.shader_path))
            .collect();
        shader_names.dedup();

        let list_str = if shader_names.is_empty() {
            "        // Brak zarejestrowanych shaderów\n".to_string()
        } else {
            shader_names.join("\n")
        };

        let script = format!(
            r#"// GameWache Shader Prewarm Auto-Generated for Unity
using System.Collections;
using System.Collections.Generic;
using UnityEngine;

namespace GameWache.Runtime
{{
    public static class ShaderPrewarmer
    {{
        private static readonly string[] ShadersToWarm = new string[]
        {{
{list_str}
        }};

        public static void WarmupAll()
        {{
            foreach (var name in ShadersToWarm)
            {{
                var shader = Shader.Find(name);
                if (shader != null)
                {{
                    Shader.WarmupAllShaders();
                }}
            }}
        }}
    }}
}}
"#
        );

        Ok(script)
    }

    pub fn write_to_project_dir(&self, project_id: &str, project_dir: &Path, engine: &str) -> std::io::Result<String> {
        match engine {
            "godot" => {
                let code = self.generate_godot_script(project_id).map_err(|e| {
                    std::io::Error::new(std::io::ErrorKind::Other, e.to_string())
                })?;
                let target_dir = project_dir.join("addons").join("gamewache").join("generated");
                fs::create_dir_all(&target_dir)?;
                let file_path = target_dir.join("shader_prewarm.gd");
                fs::write(&file_path, code)?;
                Ok(file_path.display().to_string())
            }
            "unity" => {
                let code = self.generate_unity_script(project_id).map_err(|e| {
                    std::io::Error::new(std::io::ErrorKind::Other, e.to_string())
                })?;
                let target_dir = project_dir.join("Assets").join("GameWache").join("Generated");
                fs::create_dir_all(&target_dir)?;
                let file_path = target_dir.join("ShaderPrewarmer.cs");
                fs::write(&file_path, code)?;
                Ok(file_path.display().to_string())
            }
            _ => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Unsupported engine: {}", engine),
            )),
        }
    }
}
