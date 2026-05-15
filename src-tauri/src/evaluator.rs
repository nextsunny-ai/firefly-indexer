// Evaluator — validate generated Excel against the Adobe Firefly schema,
// optionally compare against an answer-key xlsx, and surface AI-slop signals.
//
// 3 layers:
//   1. Form compliance       — title pattern · character_id pattern · age enum · edit_type enum
//   2. Answer-key match      — title / character_id / caption (token jaccard) / edit_instruction
//   3. AI-slop detector      — common filler phrases · low caption-token diversity

use calamine::{open_workbook_auto, Data, Reader};
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::Emitter;

use crate::claude_cli;

// ── Regex patterns ─────────────────────────────────────────────────────
static TITLE_FULL_SCENE: Lazy<Regex>     = Lazy::new(|| Regex::new(r"^full_scene$").unwrap());
static TITLE_BG_ISO:     Lazy<Regex>     = Lazy::new(|| Regex::new(r"^background_isolated$").unwrap());
static TITLE_PERSON_ISO: Lazy<Regex>     = Lazy::new(|| Regex::new(r"^(woman|man|boy|girl|baby)_\d+_isolated$").unwrap());
static TITLE_OBJECT_ISO: Lazy<Regex>     = Lazy::new(|| Regex::new(r"^[a-z][a-z0-9_]+_\d+_isolated$").unwrap());
static CHAR_ID_RE:       Lazy<Regex>     = Lazy::new(|| Regex::new(r"^[a-z0-9]+(_[a-z0-9]+)+_[fm]\d+_(1|5|10|20|30|50|70)$").unwrap());

const VALID_EDIT_TYPES: &[&str] = &[
    "composition", "object_removal", "character_reference", "object_reference",
];
const VALID_AGES: &[&str] = &[
    "infant", "child", "teen", "young adult", "adult", "middle-aged", "senior",
];

// Common AI-slop phrases in image captions
const SLOP_PHRASES: &[&str] = &[
    "smiling happily", "bright kitchen", "modern kitchen", "industrial",
    "with various", "amongst various", "wearing a", "stands in front of",
    "vibrant", "stunning", "beautiful", "amazing",
];

// ── Public types ───────────────────────────────────────────────────────
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EvalRequest {
    pub output_xlsx: String,
    pub answer_key_xlsx: Option<String>,
    pub run_form: bool,
    pub run_match: bool,
    pub run_slop: bool,
}

#[derive(Debug, Serialize, Default)]
pub struct EvalReport {
    pub output_path: String,
    pub answer_key_path: Option<String>,
    pub total_rows: u32,

    // Form compliance
    pub form_total: u32,
    pub form_pass:  u32,
    pub form_issues: Vec<FormIssue>,

    // Answer-key match
    pub match_total: u32,
    pub match_title_pass:    u32,
    pub match_char_pass:     u32,
    pub match_caption_avg:   f32,
    pub match_diffs:         Vec<MatchDiff>,

    // Slop
    pub slop_hits: Vec<SlopHit>,
    pub caption_token_diversity: f32,

    // Convenience
    pub form_score_pct:  f32,
    pub match_score_pct: f32,
}

#[derive(Debug, Serialize)]
pub struct FormIssue {
    pub row: u32,
    pub field: String,
    pub value: String,
    pub reason: String,
}

#[derive(Debug, Serialize)]
pub struct MatchDiff {
    pub row: u32,
    pub field: String,
    pub answer: String,
    pub actual: String,
}

#[derive(Debug, Serialize)]
pub struct SlopHit {
    pub row: u32,
    pub phrase: String,
    pub caption: String,
}

// ── Row reading ───────────────────────────────────────────────────────
#[derive(Default, Debug, Clone)]
struct Row {
    filename: String,
    sceneid: u32,
    shotid: u32,
    title: String,
    character_id: String,
    character_id_2: String,
    caption: String,
    edit_instruction: String,
    edit_type: String,
}

fn read_scenes_sheet(path: &Path) -> Result<Vec<Row>, String> {
    let mut wb = open_workbook_auto(path).map_err(|e| format!("open xlsx: {e}"))?;
    let sheet_name = wb.sheet_names().into_iter().find(|n| n == "scenes" || n.eq_ignore_ascii_case("scenes"))
        .or_else(|| wb.sheet_names().into_iter().next())
        .ok_or_else(|| "no sheet found".to_string())?;
    let range = wb.worksheet_range(&sheet_name).map_err(|e| format!("range: {e}"))?;
    let mut rows = Vec::new();
    let mut header: HashMap<String, usize> = HashMap::new();

    for (ridx, row) in range.rows().enumerate() {
        if ridx == 0 {
            for (i, c) in row.iter().enumerate() {
                if let Data::String(s) = c {
                    header.insert(s.trim().to_string(), i);
                }
            }
            continue;
        }
        let get = |k: &str| -> String {
            header.get(k).and_then(|&i| row.get(i)).map(cell_to_string).unwrap_or_default()
        };
        let get_u32 = |k: &str| -> u32 {
            header.get(k).and_then(|&i| row.get(i)).map(|c| match c {
                Data::Float(f) => *f as u32,
                Data::Int(i)   => *i as u32,
                Data::String(s) => s.trim().parse::<u32>().unwrap_or(0),
                _ => 0,
            }).unwrap_or(0)
        };

        let filename = get("filename");
        if filename.is_empty() { continue; }
        rows.push(Row {
            filename,
            sceneid: get_u32("sceneid"),
            shotid: get_u32("shotid"),
            title: get("title"),
            character_id: get("characterId"),
            character_id_2: get("characterId_2"),
            caption: get("caption"),
            edit_instruction: get("edit_instruction"),
            edit_type: get("edit_type"),
        });
    }
    Ok(rows)
}

fn cell_to_string(c: &Data) -> String {
    match c {
        Data::String(s)    => s.clone(),
        Data::Float(f)     => if f.fract() == 0.0 { format!("{}", *f as i64) } else { format!("{f}") },
        Data::Int(i)       => format!("{i}"),
        Data::Bool(b)      => format!("{b}"),
        Data::DateTime(d)  => format!("{d}"),
        Data::Empty        => String::new(),
        _ => String::new(),
    }
}

// ── Form compliance ────────────────────────────────────────────────────
fn check_form(row_idx: u32, r: &Row, out: &mut Vec<FormIssue>) -> bool {
    let mut ok = true;
    if r.title.is_empty() {
        out.push(FormIssue { row: row_idx, field: "title".into(), value: r.title.clone(), reason: "empty".into() });
        ok = false;
    } else if !(TITLE_FULL_SCENE.is_match(&r.title)
        || TITLE_BG_ISO.is_match(&r.title)
        || TITLE_PERSON_ISO.is_match(&r.title)
        || TITLE_OBJECT_ISO.is_match(&r.title))
    {
        out.push(FormIssue { row: row_idx, field: "title".into(), value: r.title.clone(),
            reason: "doesn't match full_scene / background_isolated / {role}_N_isolated / {object}_N_isolated".into() });
        ok = false;
    }

    let edit_type = r.edit_type.trim();
    if !edit_type.is_empty() && !VALID_EDIT_TYPES.contains(&edit_type) {
        out.push(FormIssue { row: row_idx, field: "edit_type".into(), value: edit_type.to_string(),
            reason: "not in {composition, object_removal, character_reference, object_reference}".into() });
        ok = false;
    }

    // full_scene requires composition
    if r.title == "full_scene" && !edit_type.is_empty() && edit_type != "composition" {
        out.push(FormIssue { row: row_idx, field: "edit_type".into(), value: edit_type.to_string(),
            reason: "full_scene must use 'composition'".into() });
        ok = false;
    }

    if !r.character_id.is_empty() && !CHAR_ID_RE.is_match(&r.character_id) {
        out.push(FormIssue { row: row_idx, field: "characterId".into(), value: r.character_id.clone(),
            reason: "expected `{prefix}_f{n}_{age}` / `{prefix}_m{n}_{age}` with age in {1,5,10,20,30,50,70}".into() });
        ok = false;
    }
    if !r.character_id_2.is_empty() && !CHAR_ID_RE.is_match(&r.character_id_2) {
        out.push(FormIssue { row: row_idx, field: "characterId_2".into(), value: r.character_id_2.clone(),
            reason: "expected character_id pattern".into() });
        ok = false;
    }

    if !r.caption.is_empty() {
        let words = r.caption.split_whitespace().count();
        if words < 8 {
            out.push(FormIssue { row: row_idx, field: "caption".into(), value: format!("{words} words"),
                reason: "caption too short (< 8 words)".into() });
            ok = false;
        } else if words > 60 {
            out.push(FormIssue { row: row_idx, field: "caption".into(), value: format!("{words} words"),
                reason: "caption too long (> 60 words)".into() });
            ok = false;
        }
    }

    ok
}

// ── Caption token jaccard ──────────────────────────────────────────────
fn tokens(s: &str) -> HashSet<String> {
    s.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.len() >= 3)
        .map(|w| w.to_string())
        .collect()
}
fn jaccard(a: &HashSet<String>, b: &HashSet<String>) -> f32 {
    if a.is_empty() && b.is_empty() { return 1.0; }
    let inter = a.intersection(b).count() as f32;
    let uni   = a.union(b).count() as f32;
    if uni == 0.0 { 1.0 } else { inter / uni }
}

fn cmp_match(rows: &[Row], answers: &[Row], report: &mut EvalReport) {
    // build (sceneid, shotid) -> Row map for answer key
    let map: HashMap<(u32, u32), &Row> = answers.iter().map(|r| ((r.sceneid, r.shotid), r)).collect();
    let mut caption_sum = 0.0f32;
    let mut caption_n = 0u32;
    for r in rows {
        if let Some(ans) = map.get(&(r.sceneid, r.shotid)) {
            report.match_total += 1;
            if r.title == ans.title { report.match_title_pass += 1; }
            else { report.match_diffs.push(MatchDiff { row: r.shotid, field: format!("title @ scene {}", r.sceneid),
                answer: ans.title.clone(), actual: r.title.clone() }); }
            if r.character_id == ans.character_id { report.match_char_pass += 1; }
            else if !ans.character_id.is_empty() {
                report.match_diffs.push(MatchDiff { row: r.shotid, field: format!("characterId @ scene {}", r.sceneid),
                    answer: ans.character_id.clone(), actual: r.character_id.clone() });
            }
            if !ans.caption.is_empty() && !r.caption.is_empty() {
                let j = jaccard(&tokens(&r.caption), &tokens(&ans.caption));
                caption_sum += j;
                caption_n += 1;
            }
        }
    }
    report.match_caption_avg = if caption_n == 0 { 0.0 } else { caption_sum / caption_n as f32 };
    report.match_score_pct = if report.match_total == 0 { 0.0 } else {
        (report.match_title_pass + report.match_char_pass) as f32 / (report.match_total as f32 * 2.0) * 100.0
    };
}

// ── AI slop detector ───────────────────────────────────────────────────
fn detect_slop(rows: &[Row], report: &mut EvalReport) {
    let mut all_tokens: HashMap<String, u32> = HashMap::new();
    let mut total_tokens: u32 = 0;
    for (i, r) in rows.iter().enumerate() {
        for phrase in SLOP_PHRASES {
            if r.caption.to_lowercase().contains(phrase) {
                report.slop_hits.push(SlopHit { row: i as u32 + 2, phrase: phrase.to_string(), caption: r.caption.clone() });
            }
        }
        for t in tokens(&r.caption) {
            *all_tokens.entry(t).or_insert(0) += 1;
            total_tokens += 1;
        }
    }
    report.caption_token_diversity = if total_tokens == 0 {
        0.0
    } else {
        all_tokens.len() as f32 / total_tokens as f32
    };
}

// ── L3 Vision round-trip ──────────────────────────────────────────────
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VisionEvalRequest {
    pub output_xlsx: String,
    pub source_folder: String,    // original images folder (filename → path)
    pub model: String,
    pub mode: String,             // "quick" (10% sample) | "all"
    pub timeout_secs: u64,
}

#[derive(Debug, Serialize, Clone)]
pub struct VisionEvalProgress {
    pub level: String,            // "info" | "row" | "done" | "fail" | "cancelled"
    pub message: String,
    pub done: u32,
    pub total: u32,
    pub avg_score: f32,
}

#[derive(Debug, Serialize)]
pub struct VisionEvalReport {
    pub total_evaluated: u32,
    pub avg_score: f32,
    pub avg_caption_score: f32,
    pub avg_edit_score: f32,
    pub rows: Vec<VisionEvalRow>,
    pub cancelled: bool,
}

#[derive(Debug, Serialize)]
pub struct VisionEvalRow {
    pub filename: String,
    pub sceneid: u32,
    pub shotid: u32,
    pub caption: String,
    pub edit_instruction: String,
    pub caption_score: i32,
    pub edit_score: i32,
    pub overall_score: i32,
    pub missing: Vec<String>,
    pub wrong: Vec<String>,
    pub notes: String,
}

// Global cancel flag for the running vision-eval task.
static VISION_EVAL_CANCEL: once_cell::sync::Lazy<Arc<AtomicBool>> =
    once_cell::sync::Lazy::new(|| Arc::new(AtomicBool::new(false)));

pub fn cancel_vision_eval() {
    VISION_EVAL_CANCEL.store(true, Ordering::SeqCst);
}

const EVAL_SYSTEM_PROMPT: &str = "You are a strict prompt-vs-image evaluator for an image-editing training dataset. \
Read the image with the Read tool, then judge the supplied English caption AND edit_instruction on three axes: \
(1) caption_score (0..100) — does the caption accurately describe what the image shows? \
(2) edit_score (0..100) — is the edit_instruction's referenced subjects/positions consistent with what the image actually contains? \
(3) overall_score (0..100) — combined judgment. \
Also surface missing/wrong items. \
Return ONLY a JSON object: \
{\"caption_score\": <int>, \"edit_score\": <int>, \"overall_score\": <int>, \
 \"missing\": [<things visible but not in prompt>], \
 \"wrong\": [<claims that contradict the image>], \
 \"notes\": \"<short>\"}. \
No prose. No markdown fence.";

fn build_eval_user_msg(image_path: &str, caption: &str, edit_instruction: &str) -> String {
    let edit_block = if edit_instruction.trim().is_empty() {
        "edit_instruction: (none — judge only caption; set edit_score equal to caption_score)".to_string()
    } else {
        format!("edit_instruction: \"{edit_instruction}\"")
    };
    format!(
        "Use the Read tool to view this image:\n  {image_path}\n\nThen evaluate these prompts against what you see:\n\ncaption: \"{caption}\"\n{edit_block}\n\nReply with ONLY the JSON object."
    )
}

pub async fn run_vision_evaluation<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    req: VisionEvalRequest,
) -> Result<VisionEvalReport, String> {
    VISION_EVAL_CANCEL.store(false, Ordering::SeqCst);

    if !claude_cli::is_installed() {
        return Err("claude CLI not found".to_string());
    }
    if !claude_cli::is_logged_in().await {
        return Err("claude CLI not logged in".to_string());
    }

    let xlsx_path = Path::new(&req.output_xlsx);
    if !xlsx_path.is_file() {
        return Err(format!("xlsx not found: {}", req.output_xlsx));
    }
    let source_dir = PathBuf::from(&req.source_folder);

    let mut rows = read_scenes_sheet(xlsx_path)?;
    // Sample 10% if quick mode
    if req.mode == "quick" {
        let step = (rows.len() / (rows.len() / 10).max(1)).max(1);
        let mut sampled = Vec::new();
        let mut i = 0;
        while i < rows.len() {
            sampled.push(rows[i].clone());
            i += step;
        }
        rows = sampled;
    }

    let total = rows.len() as u32;
    let _ = app.emit("vision-eval-progress", VisionEvalProgress {
        level: "info".into(),
        message: format!("vision 메타 검증 시작 — {} rows ({})", total, req.mode),
        done: 0, total, avg_score: 0.0,
    });

    let mut out_rows: Vec<VisionEvalRow> = Vec::new();
    let mut score_sum: i64 = 0;
    let mut caption_sum: i64 = 0;
    let mut edit_sum: i64 = 0;
    let mut done: u32 = 0;
    let mut cancelled = false;

    for r in rows {
        if VISION_EVAL_CANCEL.load(Ordering::SeqCst) {
            cancelled = true;
            break;
        }

        let img_path = source_dir.join(&r.filename);
        if !img_path.is_file() {
            // Skip missing files, emit info
            let _ = app.emit("vision-eval-progress", VisionEvalProgress {
                level: "info".into(),
                message: format!("[skip] {} — 원본 사진 없음", r.filename),
                done, total,
                avg_score: if done == 0 { 0.0 } else { score_sum as f32 / done as f32 },
            });
            continue;
        }

        let user_msg = build_eval_user_msg(&img_path.to_string_lossy(), &r.caption, &r.edit_instruction);
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<claude_cli::SubStepEvent>();
        // Drain channel without forwarding (we don't need per-image sub-steps here)
        tokio::spawn(async move { while rx.recv().await.is_some() {} });

        let res = claude_cli::run_scene_streaming(
            &user_msg,
            EVAL_SYSTEM_PROMPT,
            &req.source_folder,
            &req.model,
            req.timeout_secs,
            tx,
        ).await;

        match res {
            Ok(sr) => {
                let cap_s   = sr.parsed.get("caption_score").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                let edit_s  = sr.parsed.get("edit_score").and_then(|v| v.as_i64()).unwrap_or(cap_s as i64) as i32;
                let over_s  = sr.parsed.get("overall_score").and_then(|v| v.as_i64())
                    .unwrap_or(((cap_s as i64) + (edit_s as i64)) / 2) as i32;
                let missing: Vec<String> = sr.parsed.get("missing").and_then(|v| v.as_array())
                    .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect())
                    .unwrap_or_default();
                let wrong: Vec<String> = sr.parsed.get("wrong").and_then(|v| v.as_array())
                    .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect())
                    .unwrap_or_default();
                let notes = sr.parsed.get("notes").and_then(|v| v.as_str()).unwrap_or("").to_string();

                score_sum += over_s as i64;
                caption_sum += cap_s as i64;
                edit_sum += edit_s as i64;
                done += 1;
                let avg = score_sum as f32 / done as f32;
                let _ = app.emit("vision-eval-progress", VisionEvalProgress {
                    level: "row".into(),
                    message: format!("[{}/{}] {} — caption {} · edit {} · overall {} {}",
                        done, total, r.filename, cap_s, edit_s, over_s,
                        if missing.is_empty() && wrong.is_empty() { "✓" } else { "△" }),
                    done, total, avg_score: avg,
                });

                out_rows.push(VisionEvalRow {
                    filename: r.filename.clone(),
                    sceneid: r.sceneid,
                    shotid: r.shotid,
                    caption: r.caption.clone(),
                    edit_instruction: r.edit_instruction.clone(),
                    caption_score: cap_s,
                    edit_score: edit_s,
                    overall_score: over_s,
                    missing, wrong, notes,
                });
            }
            Err(e) => {
                let _ = app.emit("vision-eval-progress", VisionEvalProgress {
                    level: "fail".into(),
                    message: format!("[skip] {} — vision call failed: {e}", r.filename),
                    done, total,
                    avg_score: if done == 0 { 0.0 } else { score_sum as f32 / done as f32 },
                });
            }
        }
    }

    let avg_score = if done == 0 { 0.0 } else { score_sum as f32 / done as f32 };
    let _ = app.emit("vision-eval-progress", VisionEvalProgress {
        level: if cancelled { "cancelled".into() } else { "done".into() },
        message: if cancelled {
            format!("[중단] {} / {} rows · 평균 {:.1}", done, total, avg_score)
        } else {
            format!("[DONE] {} / {} rows · 평균 {:.1}", done, total, avg_score)
        },
        done, total, avg_score,
    });

    Ok(VisionEvalReport {
        total_evaluated: done,
        avg_score,
        avg_caption_score: if done == 0 { 0.0 } else { caption_sum as f32 / done as f32 },
        avg_edit_score:    if done == 0 { 0.0 } else { edit_sum as f32 / done as f32 },
        rows: out_rows,
        cancelled,
    })
}

// ── Main entry ────────────────────────────────────────────────────────
pub fn run_evaluation(req: &EvalRequest) -> Result<EvalReport, String> {
    let out_path = Path::new(&req.output_xlsx);
    if !out_path.is_file() {
        return Err(format!("output xlsx not found: {}", req.output_xlsx));
    }
    let rows = read_scenes_sheet(out_path)?;
    let mut report = EvalReport::default();
    report.output_path = req.output_xlsx.clone();
    report.answer_key_path = req.answer_key_xlsx.clone();
    report.total_rows = rows.len() as u32;

    if req.run_form {
        report.form_total = rows.len() as u32;
        for (i, r) in rows.iter().enumerate() {
            if check_form(i as u32 + 2, r, &mut report.form_issues) {
                report.form_pass += 1;
            }
        }
        report.form_score_pct = if report.form_total == 0 { 0.0 } else {
            (report.form_pass as f32 / report.form_total as f32) * 100.0
        };
    }

    if req.run_match {
        if let Some(ak) = &req.answer_key_xlsx {
            let ak_path = Path::new(ak);
            if ak_path.is_file() {
                let answers = read_scenes_sheet(ak_path)?;
                cmp_match(&rows, &answers, &mut report);
            }
        }
    }

    if req.run_slop {
        detect_slop(&rows, &mut report);
    }

    Ok(report)
}
