// Top-level indexing pipeline with per-scene streaming sub-step events.

use crate::claude_cli::{self, SubStepEvent};
use crate::excel::{CharRow, ExcelWriter, ShotRow};
use crate::prompts;

use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use tauri::Emitter;
use tokio::sync::mpsc;

static FILENAME_PREFIX_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^(?P<prefix>.+?)_(?P<num>\d{4,6})\.(?i:jpe?g|png)$").unwrap()
});

const IMAGE_EXTS: &[&str] = &["jpg", "jpeg", "png"];

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct IndexRequest {
    pub input_folder: String,
    pub output_xlsx: String,
    pub scene_size: u32,
    pub model: String,
    pub timeout_secs: u64,
    pub resume: bool,
    #[serde(default = "default_lang")]
    pub output_lang: String,   // "en" (default, Adobe Firefly standard) | "ko"
    #[serde(default)]
    pub custom_guidelines: String,  // optional Adobe/client prompt-writing rules
}

fn default_lang() -> String { "en".to_string() }

/// One unified event stream for the frontend.
/// `level`:
///   "info"     — pipeline message
///   "scene"    — a scene finished (success)
///   "fail"     — a scene failed
///   "substep"  — sub-step inside the current scene
///   "done"     — whole batch done
#[derive(Debug, Serialize, Clone)]
pub struct ProgressEvent {
    pub level: String,
    pub message: String,
    pub scene_done: u32,
    pub scene_total: u32,
    pub total_cost_usd: f64,
    pub current_scene_id: u32,
    pub current_scene_prefix: String,
    pub substep_total: u32,
    pub substep_done: u32,
    pub substep_kind: String,
}

#[derive(Debug, Serialize)]
pub struct IndexSummary {
    pub scenes_done: u32,
    pub scenes_total: u32,
    pub total_cost_usd: f64,
    pub output_path: String,
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct ProgressState {
    processed_scene_ids: Vec<u32>,
    total_cost_usd: f64,
    known_characters: Vec<CharRow>,
}

fn scan_images(dir: &Path) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = match fs::read_dir(dir) {
        Ok(rd) => rd.filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.is_file())
            .filter(|p| {
                p.extension().and_then(|e| e.to_str())
                    .map(|e| IMAGE_EXTS.contains(&e.to_ascii_lowercase().as_str()))
                    .unwrap_or(false)
            })
            .collect(),
        Err(_) => return Vec::new(),
    };
    v.sort();
    v
}

fn extract_prefix(filename: &str) -> String {
    if let Some(c) = FILENAME_PREFIX_RE.captures(filename) {
        return c.name("prefix").unwrap().as_str().to_ascii_lowercase();
    }
    Path::new(filename).file_stem().and_then(|s| s.to_str())
        .map(|s| s.to_ascii_lowercase()).unwrap_or_default()
}

struct Scene {
    sceneid: u32,
    prefix: String,
    image_paths: Vec<String>,
    filenames: Vec<String>,
}

fn group_scenes(images: &[PathBuf], scene_size: usize) -> Vec<Scene> {
    let mut scenes = Vec::new();
    let mut id = 1;
    for chunk in images.chunks(scene_size) {
        if chunk.is_empty() { continue; }
        let prefix = extract_prefix(&chunk[0].file_name().and_then(|n| n.to_str()).unwrap_or("").to_string());
        scenes.push(Scene {
            sceneid: id,
            prefix,
            image_paths: chunk.iter().map(|p| p.to_string_lossy().to_string()).collect(),
            filenames: chunk.iter().map(|p| p.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string()).collect(),
        });
        id += 1;
    }
    scenes
}

fn progress_path(output_xlsx: &Path) -> PathBuf {
    let mut p = output_xlsx.to_path_buf();
    let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("output").to_string();
    p.set_file_name(format!("{}.progress.json", stem));
    p
}
fn load_state(path: &Path) -> ProgressState {
    fs::read_to_string(path).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
}
fn save_state(path: &Path, state: &ProgressState) -> Result<(), String> {
    let json = serde_json::to_string_pretty(state).map_err(|e| e.to_string())?;
    fs::write(path, json).map_err(|e| e.to_string())
}

pub async fn run_indexing<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    req: IndexRequest,
) -> Result<IndexSummary, String> {
    if !claude_cli::is_installed() {
        return Err("claude CLI not found. Install: npm install -g @anthropic-ai/claude-code".to_string());
    }
    if !claude_cli::is_logged_in().await {
        return Err("claude CLI not logged in. Run `claude /login` first.".to_string());
    }

    let input_dir = PathBuf::from(&req.input_folder);
    let output_path = PathBuf::from(&req.output_xlsx);

    let images = scan_images(&input_dir);
    if images.is_empty() {
        return Err(format!("No images found in: {}", req.input_folder));
    }
    let scenes = group_scenes(&images, req.scene_size as usize);
    let total = scenes.len() as u32;

    let prog_path = progress_path(&output_path);
    let mut state = if req.resume && prog_path.exists() {
        load_state(&prog_path)
    } else {
        let _ = fs::remove_file(&prog_path);
        let _ = fs::remove_file(&output_path);
        ProgressState::default()
    };
    let processed: HashSet<u32> = state.processed_scene_ids.iter().copied().collect();

    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    let mut writer = ExcelWriter::create_new(&output_path)?;
    let mut known_set: HashSet<String> = HashSet::new();
    if !state.known_characters.is_empty() {
        writer.append_chars(&state.known_characters, &mut known_set)?;
    }
    let mut total_cost = state.total_cost_usd;

    for scene in scenes {
        if processed.contains(&scene.sceneid) {
            continue;
        }
        let substep_total = scene.image_paths.len() as u32;
        emit_info(&app, &state, total, total_cost, &scene, substep_total);

        let existing_chars = serde_json::to_value(&state.known_characters).unwrap_or(serde_json::Value::Null);
        let user_msg = prompts::build_user_message(
            &scene.image_paths, &scene.prefix, &existing_chars,
            &req.output_lang, &req.custom_guidelines,
        );

        // Sub-step channel — re-emit each event as a "substep" ProgressEvent.
        let (tx, mut rx) = mpsc::unbounded_channel::<SubStepEvent>();
        let app_clone = app.clone();
        let scene_id = scene.sceneid;
        let scene_prefix = scene.prefix.clone();
        let done_now = state.processed_scene_ids.len() as u32;
        let cost_now = total_cost;
        let relay = tokio::spawn(async move {
            while let Some(ev) = rx.recv().await {
                let _ = app_clone.emit("indexing-progress", ProgressEvent {
                    level: "substep".into(),
                    message: ev.message.clone(),
                    scene_done: done_now,
                    scene_total: total,
                    total_cost_usd: cost_now,
                    current_scene_id: scene_id,
                    current_scene_prefix: scene_prefix.clone(),
                    substep_total,
                    substep_done: ev.index,
                    substep_kind: ev.kind,
                });
            }
        });

        let res = claude_cli::run_scene_streaming(
            &user_msg,
            prompts::SYSTEM_PROMPT,
            &req.input_folder,
            &req.model,
            req.timeout_secs,
            tx,
        ).await;
        let _ = relay.await;

        let res = match res {
            Ok(r) => r,
            Err(e) => {
                let _ = app.emit("indexing-progress", ProgressEvent {
                    level: "fail".into(),
                    message: format!("scene {} 실패: {}", scene.sceneid, e),
                    scene_done: state.processed_scene_ids.len() as u32,
                    scene_total: total,
                    total_cost_usd: total_cost,
                    current_scene_id: scene.sceneid,
                    current_scene_prefix: scene.prefix.clone(),
                    substep_total,
                    substep_done: 0,
                    substep_kind: "error".into(),
                });
                continue;
            }
        };

        let shots_json = res.parsed.get("shots").cloned().unwrap_or(serde_json::Value::Array(vec![]));
        let chars_json = res.parsed.get("characters").cloned().unwrap_or(serde_json::Value::Array(vec![]));

        let mut shot_rows: Vec<ShotRow> = Vec::new();
        if let Some(arr) = shots_json.as_array() {
            for s in arr {
                let shotid = s.get("shotid").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
                let idx = if shotid == 0 { 0 } else { (shotid - 1) as usize };
                let filename = scene.filenames.get(idx).cloned().unwrap_or_default();
                shot_rows.push(ShotRow {
                    filename,
                    story: None,
                    sceneid: scene.sceneid,
                    shotid,
                    title: s.get("title").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    character_id:   s.get("characterId").and_then(|v| v.as_str()).map(String::from),
                    character_id_2: s.get("characterId_2").and_then(|v| v.as_str()).map(String::from),
                    caption: s.get("caption").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    edit_instruction: s.get("edit_instruction").and_then(|v| v.as_str()).map(String::from),
                    edit_type:        s.get("edit_type").and_then(|v| v.as_str()).map(String::from),
                });
            }
        }
        shot_rows.sort_by_key(|r| r.shotid);
        writer.append_shots(&shot_rows)?;

        let mut new_chars: Vec<CharRow> = Vec::new();
        if let Some(arr) = chars_json.as_array() {
            for c in arr {
                let id = c.get("character_id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                if id.is_empty() || known_set.contains(&id) { continue; }
                new_chars.push(CharRow {
                    character_id: id.clone(),
                    ethnicity: c.get("ethnicity").and_then(|v| v.as_str()).unwrap_or("Asian").to_string(),
                    age:       c.get("age").and_then(|v| v.as_str()).unwrap_or("adult").to_string(),
                    gender:    c.get("gender").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                });
            }
        }
        let new_count = writer.append_chars(&new_chars, &mut known_set)?;
        state.known_characters.extend(new_chars);

        state.processed_scene_ids.push(scene.sceneid);
        total_cost += res.cost_usd;
        state.total_cost_usd = total_cost;
        writer.save()?;
        save_state(&prog_path, &state)?;

        let _ = app.emit("indexing-progress", ProgressEvent {
            level: "scene".into(),
            message: format!("[{}/{}] scene {} OK · shots={} +chars={} · {}ms",
                state.processed_scene_ids.len() as u32, total, scene.sceneid,
                shot_rows.len(), new_count, res.duration_ms),
            scene_done: state.processed_scene_ids.len() as u32,
            scene_total: total,
            total_cost_usd: total_cost,
            current_scene_id: scene.sceneid,
            current_scene_prefix: scene.prefix.clone(),
            substep_total,
            substep_done: substep_total,
            substep_kind: "scene_done".into(),
        });
    }

    writer.save()?;
    save_state(&prog_path, &state)?;

    let summary = IndexSummary {
        scenes_done: state.processed_scene_ids.len() as u32,
        scenes_total: total,
        total_cost_usd: total_cost,
        output_path: output_path.to_string_lossy().to_string(),
    };
    let _ = app.emit("indexing-progress", ProgressEvent {
        level: "done".into(),
        message: format!("[DONE] {}/{} 씬 — {}", summary.scenes_done, summary.scenes_total, summary.output_path),
        scene_done: summary.scenes_done,
        scene_total: summary.scenes_total,
        total_cost_usd: summary.total_cost_usd,
        current_scene_id: 0,
        current_scene_prefix: String::new(),
        substep_total: 0,
        substep_done: 0,
        substep_kind: "done".into(),
    });
    Ok(summary)
}

fn emit_info<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    state: &ProgressState,
    total: u32,
    total_cost: f64,
    scene: &Scene,
    substep_total: u32,
) {
    let _ = app.emit("indexing-progress", ProgressEvent {
        level: "info".into(),
        message: format!("scene {} ({}) — Claude 호출 시작 (사진 {}장)",
            scene.sceneid, scene.prefix, scene.image_paths.len()),
        scene_done: state.processed_scene_ids.len() as u32,
        scene_total: total,
        total_cost_usd: total_cost,
        current_scene_id: scene.sceneid,
        current_scene_prefix: scene.prefix.clone(),
        substep_total,
        substep_done: 0,
        substep_kind: "scene_start".into(),
    });
}
