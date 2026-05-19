// Top-level indexing pipeline.
//
// Two vision engines (user-selectable):
//   - "claude" — Claude Code CLI, per-image streaming sub-step events.
//   - "gemini" — Gemini CLI, one call per scene, free Google-login tier.
//
// Two folder modes:
//   - "single" — images sit directly in the chosen folder; grouped into
//     scenes of `scene_size`.
//   - "nested" — the chosen folder is a PARENT of many set-folders; every
//     leaf folder that holds images becomes ONE scene. sceneid runs
//     continuously across all folders → one Excel, numbered like the sample.
//
// Resume: progress.json stores every finished scene (key + rows + chars).
// On resume the manifest is rebuilt deterministically, finished scenes are
// replayed from the file (sceneid re-stamped from manifest position), and
// only the unfinished scenes hit the API. The Excel is therefore always
// complete and continuously numbered, even across interruptions.

use crate::claude_cli::{self, SubStepEvent};
use crate::excel::{CharRow, ExcelWriter, ShotRow};
use crate::gemini_cli;
use crate::prompts;

use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use tauri::{Emitter, Manager};
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
    pub output_lang: String,        // "en" (Adobe Firefly standard) | "ko"
    #[serde(default)]
    pub custom_guidelines: String,  // optional Adobe/client prompt-writing rules
    #[serde(default = "default_engine")]
    pub vision_engine: String,      // "claude" | "gemini"
    #[serde(default = "default_mode")]
    pub folder_mode: String,        // "single" | "nested"
}

fn default_lang() -> String { "en".to_string() }
fn default_engine() -> String { "claude".to_string() }
fn default_mode() -> String { "single".to_string() }

#[derive(Debug, Serialize, Clone)]
pub struct ProgressEvent {
    pub level: String,            // info | scene | fail | substep | done
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

// ── Resume state ──────────────────────────────────────────────────────
#[derive(Debug, Serialize, Deserialize, Clone)]
struct DoneScene {
    key: String,            // stable scene identity (folder path / chunk)
    shots: Vec<ShotRow>,    // sceneid re-stamped from manifest on replay
    chars: Vec<CharRow>,    // characters newly minted in this scene
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct ProgressState {
    done: Vec<DoneScene>,
    total_cost_usd: f64,
}

// ── Scene manifest ────────────────────────────────────────────────────
struct Scene {
    sceneid: u32,                 // 1..N — assigned from manifest position
    prefix: String,               // scene_prefix for character_id minting
    dir: PathBuf,                 // folder holding the images (gemini cwd)
    image_paths: Vec<String>,
    filenames: Vec<String>,
    key: String,                  // stable identity for resume
}

fn is_image(p: &Path) -> bool {
    p.is_file()
        && p.extension()
            .and_then(|e| e.to_str())
            .map(|e| IMAGE_EXTS.contains(&e.to_ascii_lowercase().as_str()))
            .unwrap_or(false)
}

fn scan_images(dir: &Path) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = match fs::read_dir(dir) {
        Ok(rd) => rd
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| is_image(p))
            .collect(),
        Err(_) => return Vec::new(),
    };
    v.sort();
    v
}

/// Recursively collect every folder that DIRECTLY contains image files.
/// Deterministic order (sorted) so scene numbering is stable across runs.
fn find_image_folders(root: &Path) -> Vec<PathBuf> {
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        let mut has_img = false;
        let mut subdirs: Vec<PathBuf> = Vec::new();
        if let Ok(rd) = fs::read_dir(dir) {
            for e in rd.flatten() {
                let p = e.path();
                if p.is_dir() {
                    subdirs.push(p);
                } else if is_image(&p) {
                    has_img = true;
                }
            }
        }
        if has_img {
            out.push(dir.to_path_buf());
        }
        subdirs.sort();
        for d in subdirs {
            walk(&d, out);
        }
    }
    let mut out = Vec::new();
    walk(root, &mut out);
    out
}

fn extract_prefix(filename: &str) -> String {
    if let Some(c) = FILENAME_PREFIX_RE.captures(filename) {
        return c.name("prefix").unwrap().as_str().to_ascii_lowercase();
    }
    Path::new(filename)
        .file_stem()
        .and_then(|s| s.to_str())
        .map(|s| s.to_ascii_lowercase())
        .unwrap_or_default()
}

fn make_scene(dir: &Path, images: &[PathBuf], key: String) -> Scene {
    let filenames: Vec<String> = images
        .iter()
        .map(|p| p.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string())
        .collect();
    let prefix = extract_prefix(filenames.first().map(|s| s.as_str()).unwrap_or(""));
    Scene {
        sceneid: 0,
        prefix,
        dir: dir.to_path_buf(),
        image_paths: images.iter().map(|p| p.to_string_lossy().to_string()).collect(),
        filenames,
        key,
    }
}

/// Build the ordered scene manifest.
fn build_manifest(input: &Path, scene_size: usize, nested: bool) -> Vec<Scene> {
    let mut scenes: Vec<Scene> = Vec::new();
    if nested {
        // Each leaf folder with images = exactly one scene.
        for dir in find_image_folders(input) {
            let imgs = scan_images(&dir);
            if imgs.is_empty() {
                continue;
            }
            let key = format!("dir::{}", dir.to_string_lossy());
            scenes.push(make_scene(&dir, &imgs, key));
        }
    } else {
        // Flat folder — chunk into scenes of scene_size.
        let imgs = scan_images(input);
        let size = scene_size.max(1);
        for (ci, chunk) in imgs.chunks(size).enumerate() {
            if chunk.is_empty() {
                continue;
            }
            let key = format!("single::{ci}");
            scenes.push(make_scene(input, chunk, key));
        }
    }
    for (i, s) in scenes.iter_mut().enumerate() {
        s.sceneid = (i + 1) as u32;
    }
    scenes
}

fn progress_path(output_xlsx: &Path) -> PathBuf {
    let mut p = output_xlsx.to_path_buf();
    let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("output").to_string();
    p.set_file_name(format!("{stem}.progress.json"));
    p
}
fn load_state(path: &Path) -> ProgressState {
    fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}
fn save_state(path: &Path, state: &ProgressState) -> Result<(), String> {
    let json = serde_json::to_string(state).map_err(|e| e.to_string())?;
    fs::write(path, json).map_err(|e| e.to_string())
}

fn emit<R: tauri::Runtime>(app: &tauri::AppHandle<R>, ev: ProgressEvent) {
    let _ = app.emit("indexing-progress", ev);
}

/// Convert one engine result into shot + char rows for `scene`.
fn rows_from_result(
    scene: &Scene,
    parsed: &serde_json::Value,
    known_set: &HashSet<String>,
) -> (Vec<ShotRow>, Vec<CharRow>) {
    let shots_json = parsed.get("shots").cloned().unwrap_or(serde_json::Value::Array(vec![]));
    let chars_json = parsed.get("characters").cloned().unwrap_or(serde_json::Value::Array(vec![]));

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
                character_id: s.get("characterId").and_then(|v| v.as_str()).map(String::from),
                character_id_2: s.get("characterId_2").and_then(|v| v.as_str()).map(String::from),
                caption: s.get("caption").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                edit_instruction: s.get("edit_instruction").and_then(|v| v.as_str()).map(String::from),
                edit_type: s.get("edit_type").and_then(|v| v.as_str()).map(String::from),
            });
        }
    }
    shot_rows.sort_by_key(|r| r.shotid);

    let mut new_chars: Vec<CharRow> = Vec::new();
    if let Some(arr) = chars_json.as_array() {
        for c in arr {
            let id = c.get("character_id").and_then(|v| v.as_str()).unwrap_or("").to_string();
            if id.is_empty() || known_set.contains(&id) {
                continue;
            }
            new_chars.push(CharRow {
                character_id: id,
                ethnicity: c.get("ethnicity").and_then(|v| v.as_str()).unwrap_or("Asian").to_string(),
                age: c.get("age").and_then(|v| v.as_str()).unwrap_or("adult").to_string(),
                gender: c.get("gender").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            });
        }
    }
    (shot_rows, new_chars)
}

pub async fn run_indexing<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    req: IndexRequest,
) -> Result<IndexSummary, String> {
    let engine = req.vision_engine.to_ascii_lowercase();
    let nested = req.folder_mode.eq_ignore_ascii_case("nested");
    let res_dir = app.path().resource_dir().ok();

    // ── Preflight: the SELECTED engine must be ready ───────────────────
    if engine == "gemini" {
        if !gemini_cli::is_installed(res_dir.as_deref()) {
            return Err("Gemini 런타임을 찾을 수 없습니다 — 앱을 다시 설치해 주세요.".to_string());
        }
        if !gemini_cli::is_logged_in() {
            return Err("Gemini 로그인이 필요합니다 — Setup에서 구글 로그인 해주세요.".to_string());
        }
    } else {
        if !claude_cli::is_installed() {
            return Err("Claude Code CLI가 설치되지 않았습니다 — Setup에서 설치해주세요.".to_string());
        }
        if !claude_cli::is_logged_in().await {
            return Err("Claude Code 로그인이 필요합니다 — Setup에서 로그인해주세요.".to_string());
        }
    }

    let input_dir = PathBuf::from(&req.input_folder);
    let output_path = PathBuf::from(&req.output_xlsx);

    let manifest = build_manifest(&input_dir, req.scene_size as usize, nested);
    if manifest.is_empty() {
        return Err(format!("이미지를 찾지 못했습니다: {}", req.input_folder));
    }
    let total = manifest.len() as u32;

    // ── Resume state ───────────────────────────────────────────────────
    let prog_path = progress_path(&output_path);
    let mut state = if req.resume && prog_path.exists() {
        load_state(&prog_path)
    } else {
        let _ = fs::remove_file(&prog_path);
        let _ = fs::remove_file(&output_path);
        ProgressState::default()
    };
    let done_by_key: HashMap<String, usize> = state
        .done
        .iter()
        .enumerate()
        .map(|(i, d)| (d.key.clone(), i))
        .collect();

    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    let mut writer = ExcelWriter::create_new(&output_path)?;
    let mut known_set: HashSet<String> = HashSet::new();
    let mut known_chars: Vec<CharRow> = Vec::new(); // accumulates in manifest order
    let mut total_cost = state.total_cost_usd;
    let mut scenes_done: u32 = 0;
    let engine_label = if engine == "gemini" { "Gemini" } else { "Claude" };

    for scene in &manifest {
        let substep_total = scene.image_paths.len() as u32;

        // ── Already finished (resume) — replay from progress.json ──────
        if let Some(&di) = done_by_key.get(&scene.key) {
            let mut shots = state.done[di].shots.clone();
            for s in &mut shots {
                s.sceneid = scene.sceneid; // re-stamp to current manifest position
            }
            writer.append_shots(&shots)?;
            writer.append_chars(&state.done[di].chars, &mut known_set)?;
            known_chars.extend(state.done[di].chars.iter().cloned());
            scenes_done += 1;
            emit(&app, ProgressEvent {
                level: "scene".into(),
                message: format!("[{}/{}] scene {} (이어하기 — 이미 완료)", scenes_done, total, scene.sceneid),
                scene_done: scenes_done,
                scene_total: total,
                total_cost_usd: total_cost,
                current_scene_id: scene.sceneid,
                current_scene_prefix: scene.prefix.clone(),
                substep_total,
                substep_done: substep_total,
                substep_kind: "scene_cached".into(),
            });
            continue;
        }

        // ── New scene — call the engine ────────────────────────────────
        emit(&app, ProgressEvent {
            level: "info".into(),
            message: format!("scene {} ({}) — {} 호출 (사진 {}장)",
                scene.sceneid, scene.prefix, engine_label, scene.image_paths.len()),
            scene_done: scenes_done,
            scene_total: total,
            total_cost_usd: total_cost,
            current_scene_id: scene.sceneid,
            current_scene_prefix: scene.prefix.clone(),
            substep_total,
            substep_done: 0,
            substep_kind: "scene_start".into(),
        });

        let existing = serde_json::to_value(&known_chars).unwrap_or(serde_json::Value::Null);
        let user_msg = prompts::build_user_message(
            &scene.image_paths, &scene.prefix, &existing,
            &req.output_lang, &req.custom_guidelines, &engine,
        );
        let scene_dir = scene.dir.to_string_lossy().to_string();

        let res: Result<claude_cli::SceneResult, String> = if engine == "gemini" {
            emit(&app, ProgressEvent {
                level: "substep".into(),
                message: format!("Gemini 분석 중 (사진 {}장)", scene.image_paths.len()),
                scene_done: scenes_done,
                scene_total: total,
                total_cost_usd: total_cost,
                current_scene_id: scene.sceneid,
                current_scene_prefix: scene.prefix.clone(),
                substep_total,
                substep_done: 0,
                substep_kind: "analyzing".into(),
            });
            gemini_cli::run_scene(&user_msg, &scene_dir, req.timeout_secs, res_dir.as_deref()).await
        } else {
            let (tx, mut rx) = mpsc::unbounded_channel::<SubStepEvent>();
            let app_clone = app.clone();
            let scene_id = scene.sceneid;
            let scene_prefix = scene.prefix.clone();
            let done_now = scenes_done;
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
            let r = claude_cli::run_scene_streaming(
                &user_msg, prompts::SYSTEM_PROMPT, &scene_dir,
                &req.model, req.timeout_secs, tx,
            ).await;
            let _ = relay.await;
            r.map_err(|e| e.to_string())
        };

        let res = match res {
            Ok(r) => r,
            Err(e) => {
                emit(&app, ProgressEvent {
                    level: "fail".into(),
                    message: format!("scene {} 실패: {}", scene.sceneid, e),
                    scene_done: scenes_done,
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

        let (shot_rows, new_chars) = rows_from_result(scene, &res.parsed, &known_set);
        writer.append_shots(&shot_rows)?;
        let new_count = writer.append_chars(&new_chars, &mut known_set)?;
        known_chars.extend(new_chars.iter().cloned());

        total_cost += res.cost_usd;
        scenes_done += 1;
        state.total_cost_usd = total_cost;
        state.done.push(DoneScene {
            key: scene.key.clone(),
            shots: shot_rows.clone(),
            chars: new_chars,
        });
        writer.save()?;
        save_state(&prog_path, &state)?;

        emit(&app, ProgressEvent {
            level: "scene".into(),
            message: format!("[{}/{}] scene {} OK · shots={} +chars={} · {}ms",
                scenes_done, total, scene.sceneid, shot_rows.len(), new_count, res.duration_ms),
            scene_done: scenes_done,
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
        scenes_done,
        scenes_total: total,
        total_cost_usd: total_cost,
        output_path: output_path.to_string_lossy().to_string(),
    };
    emit(&app, ProgressEvent {
        level: "done".into(),
        message: format!("[완료] {}/{} 세트 — {}", summary.scenes_done, summary.scenes_total, summary.output_path),
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
