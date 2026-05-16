// claude CLI subprocess wrapper with stream-json sub-step events.
//
// Verified facts (코딩실수모음.md 2026-05-13~14):
//   - Anthropic Messages API + OAuth Bearer = HTTP 401 (외부 앱 OAuth 사용 금지 정책)
//   - 진짜 작동 path = `claude` CLI subprocess (= 공식 도구 = 약관 OK)
//   - macOS = ~/.claude/.credentials.json 없음 → Keychain 저장
//     → `claude auth status` JSON 호출이 OS-invariant 정공법
//   - 자식이 부모 Claude-Code 세션 컨텍스트 흡수 사고
//     → env scrub (CLAUDE_*/ANTHROPIC_*) + cwd 격리 + stdin pipe + system-prompt override
//
// stream-json events tapped for sub-step progress UI (대표님 명시 2026-05-15):
//   stream_event.event.type == "content_block_start" with content_block.type == "tool_use"
//     → 새 Read tool 호출 시작 = "사진 N 분석 중"

use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;

use serde::Serialize;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;
use tokio::time::timeout;

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

const HAIKU_MODEL: &str = "claude-haiku-4-5-20251001";
const SONNET_MODEL: &str = "claude-sonnet-4-6";
const OPUS_MODEL: &str = "claude-opus-4-7";

#[derive(Debug, thiserror::Error)]
pub enum CliError {
    #[error("claude CLI not found in PATH. Install: npm install -g @anthropic-ai/claude-code")]
    NotFound,
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("subprocess timed out after {0}s")]
    Timeout(u64),
    #[error("subprocess exited with code {0}: {1}")]
    Subprocess(i32, String),
    #[error("invalid JSON from claude CLI: {0}")]
    Parse(String),
}

pub fn model_for_alias(alias: &str) -> &str {
    match alias.to_ascii_lowercase().as_str() {
        "sonnet" => SONNET_MODEL,
        "opus"   => OPUS_MODEL,
        _        => HAIKU_MODEL,
    }
}

pub fn find_bin() -> Option<PathBuf> {
    let names: &[&str] = if cfg!(target_os = "windows") {
        &["claude.cmd", "claude.exe", "claude"]
    } else {
        &["claude"]
    };

    // 1) Standard PATH search
    if let Some(path_var) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&path_var) {
            for name in names {
                let p = dir.join(name);
                if p.is_file() {
                    return Some(p);
                }
            }
        }
    }

    // 2) Common npm-global / package-manager locations (handles the "다음날 또
    //    체크 안 됨" / PATH-not-refreshed issue — old learning #claude_PATH_미반영).
    let home = dirs::home_dir();
    let mut candidates: Vec<PathBuf> = Vec::new();

    // ★ Native installer location (Anthropic official, no Node.js) — check FIRST.
    if let Some(h) = home.as_ref() {
        candidates.push(h.join(".local").join("bin"));
    }

    #[cfg(target_os = "windows")]
    {
        if let Some(appdata) = std::env::var_os("APPDATA") {
            candidates.push(PathBuf::from(&appdata).join("npm"));
        }
        if let Some(localappdata) = std::env::var_os("LOCALAPPDATA") {
            candidates.push(PathBuf::from(&localappdata).join("npm"));
        }
        if let Some(h) = home.as_ref() {
            candidates.push(h.join(".npm-global"));
            candidates.push(h.join("AppData").join("Roaming").join("npm"));
        }
        candidates.push(PathBuf::from("C:\\Program Files\\nodejs"));
    }

    #[cfg(target_os = "macos")]
    {
        candidates.push(PathBuf::from("/usr/local/bin"));
        candidates.push(PathBuf::from("/opt/homebrew/bin"));
        candidates.push(PathBuf::from("/opt/local/bin"));
        if let Some(h) = home.as_ref() {
            candidates.push(h.join(".npm-global").join("bin"));
            candidates.push(h.join(".volta").join("bin"));
            candidates.push(h.join(".fnm").join("aliases").join("default").join("bin"));
            candidates.push(h.join(".nvm").join("versions").join("node").join("current").join("bin"));
        }
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        candidates.push(PathBuf::from("/usr/local/bin"));
        candidates.push(PathBuf::from("/usr/bin"));
        if let Some(h) = home.as_ref() {
            candidates.push(h.join(".npm-global").join("bin"));
        }
    }

    for dir in candidates {
        for name in names {
            let p = dir.join(name);
            if p.is_file() {
                return Some(p);
            }
        }
    }

    None
}

pub fn is_installed() -> bool { find_bin().is_some() }

pub async fn is_logged_in() -> bool {
    if let Some(bin) = find_bin() {
        let mut cmd = Command::new(&bin);
        cmd.arg("auth").arg("status");
        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
        #[cfg(target_os = "windows")]
        cmd.creation_flags(CREATE_NO_WINDOW);
        if let Ok(out) = cmd.output().await {
            let stdout = String::from_utf8_lossy(&out.stdout);
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&stdout) {
                if v.get("loggedIn").and_then(|b| b.as_bool()).unwrap_or(false) { return true; }
            }
            let lower = stdout.to_lowercase();
            if lower.contains("logged in") && !lower.contains("not logged in") { return true; }
        }
    }
    if let Some(mut home) = dirs::home_dir() {
        home.push(".claude");
        home.push(".credentials.json");
        if home.is_file() { return true; }
    }
    false
}

/// Strip CLAUDE_*/ANTHROPIC_* env vars so the child doesn't inherit the
/// parent Claude-Code session context.
pub fn child_env() -> std::collections::HashMap<String, String> {
    std::env::vars()
        .filter(|(k, _)| !k.starts_with("CLAUDE") && !k.starts_with("ANTHROPIC"))
        .collect()
}

#[derive(Debug, Clone, Serialize)]
pub struct SubStepEvent {
    pub kind: String,       // "prompt_built" | "read_start" | "read_done" | "final_text" | "result_ok" | "result_err"
    pub index: u32,         // 1-based tool_use ordinal
    pub message: String,    // user-facing message
}

pub struct SceneResult {
    pub parsed: serde_json::Value,
    pub cost_usd: f64,
    pub duration_ms: u64,
}

/// Run vision on a single scene; streams sub-step events through `tx`.
pub async fn run_scene_streaming(
    user_msg: &str,
    system_prompt: &str,
    add_dir: &str,
    model_alias: &str,
    timeout_secs: u64,
    tx: mpsc::UnboundedSender<SubStepEvent>,
) -> Result<SceneResult, CliError> {
    let bin = find_bin().ok_or(CliError::NotFound)?;
    let model = model_for_alias(model_alias);

    let cwd = tempfile::tempdir()?;

    let mut cmd = Command::new(&bin);
    cmd.arg("-p")
        .arg("--model").arg(model)
        .arg("--output-format").arg("stream-json")
        .arg("--include-partial-messages")
        .arg("--verbose")
        .arg("--allowedTools").arg("Read")
        .arg("--add-dir").arg(add_dir)
        .arg("--permission-mode").arg("acceptEdits")
        .arg("--system-prompt").arg(system_prompt)
        .arg("--no-session-persistence")
        .arg("--disable-slash-commands")
        .current_dir(cwd.path())
        .env_clear()
        .envs(child_env())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    #[cfg(target_os = "windows")]
    cmd.creation_flags(CREATE_NO_WINDOW);

    let _ = tx.send(SubStepEvent {
        kind: "prompt_built".into(),
        index: 0,
        message: "프롬프트 빌드 — Claude 호출".into(),
    });

    let mut child = cmd.spawn()?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(user_msg.as_bytes()).await?;
        stdin.shutdown().await?;
    }

    let stdout = child.stdout.take().expect("piped");
    let stderr = child.stderr.take().expect("piped");

    let tx_stdout = tx.clone();
    let stdout_task = tokio::spawn(async move {
        let mut reader = BufReader::new(stdout).lines();
        let mut tool_use_ordinal: u32 = 0;
        let mut final_text = String::new();
        let mut final_cost: f64 = 0.0;
        let mut final_duration_ms: u64 = 0;
        let mut final_is_error = false;
        let mut current_tool_use_index: Option<u64> = None;

        while let Ok(Some(line)) = reader.next_line().await {
            if line.trim().is_empty() { continue; }
            let v: serde_json::Value = match serde_json::from_str(&line) {
                Ok(v) => v,
                Err(_) => continue,
            };
            let outer_type = v.get("type").and_then(|s| s.as_str()).unwrap_or("");
            match outer_type {
                "stream_event" => {
                    let ev = match v.get("event") { Some(e) => e, None => continue };
                    let ev_type = ev.get("type").and_then(|s| s.as_str()).unwrap_or("");
                    match ev_type {
                        "content_block_start" => {
                            let cb = ev.get("content_block").cloned().unwrap_or(serde_json::Value::Null);
                            let cb_type = cb.get("type").and_then(|s| s.as_str()).unwrap_or("");
                            if cb_type == "tool_use" {
                                let name = cb.get("name").and_then(|s| s.as_str()).unwrap_or("");
                                if name == "Read" {
                                    tool_use_ordinal += 1;
                                    current_tool_use_index = ev.get("index").and_then(|i| i.as_u64());
                                    let _ = tx_stdout.send(SubStepEvent {
                                        kind: "read_start".into(),
                                        index: tool_use_ordinal,
                                        message: format!("사진 {} 분석 중…", tool_use_ordinal),
                                    });
                                }
                            }
                        }
                        "content_block_stop" => {
                            if let Some(stop_idx) = ev.get("index").and_then(|i| i.as_u64()) {
                                if Some(stop_idx) == current_tool_use_index {
                                    let _ = tx_stdout.send(SubStepEvent {
                                        kind: "read_done".into(),
                                        index: tool_use_ordinal,
                                        message: format!("사진 {} 분석 완료", tool_use_ordinal),
                                    });
                                    current_tool_use_index = None;
                                }
                            }
                        }
                        _ => {}
                    }
                }
                "user" => {
                    // tool_result blocks land here; surface for completeness
                    if let Some(arr) = v.get("content").and_then(|c| c.as_array()) {
                        for b in arr {
                            if b.get("type").and_then(|s| s.as_str()) == Some("tool_result") {
                                // already covered by content_block_stop on assistant side
                            }
                        }
                    }
                }
                "result" => {
                    final_text = v.get("result").and_then(|s| s.as_str()).unwrap_or("").to_string();
                    final_cost = v.get("total_cost_usd").and_then(|x| x.as_f64()).unwrap_or(0.0);
                    final_duration_ms = v.get("duration_ms").and_then(|x| x.as_u64()).unwrap_or(0);
                    final_is_error = v.get("is_error").and_then(|b| b.as_bool()).unwrap_or(false);
                    let _ = tx_stdout.send(SubStepEvent {
                        kind: if final_is_error { "result_err".into() } else { "final_text".into() },
                        index: tool_use_ordinal,
                        message: if final_is_error {
                            "Claude 응답 = 에러".into()
                        } else {
                            "씬 통합 JSON 응답 받음".into()
                        },
                    });
                }
                _ => {}
            }
        }
        (final_text, final_cost, final_duration_ms, final_is_error)
    });

    let stderr_task = tokio::spawn(async move {
        let mut reader = BufReader::new(stderr).lines();
        let mut buf = String::new();
        while let Ok(Some(line)) = reader.next_line().await {
            buf.push_str(&line); buf.push('\n');
        }
        buf
    });

    let wait_result = timeout(Duration::from_secs(timeout_secs), child.wait()).await;
    let status = match wait_result {
        Ok(r) => r?,
        Err(_) => {
            let _ = child.kill().await;
            return Err(CliError::Timeout(timeout_secs));
        }
    };
    let (final_text, final_cost, final_duration_ms, final_is_error) = stdout_task.await.unwrap_or_default();
    let stderr_buf = stderr_task.await.unwrap_or_default();

    if !status.success() && !final_is_error {
        return Err(CliError::Subprocess(status.code().unwrap_or(-1), stderr_buf));
    }
    if final_is_error {
        return Err(CliError::Subprocess(-2, final_text));
    }

    let cleaned = strip_code_fence(&final_text);
    let parsed: serde_json::Value = serde_json::from_str(&cleaned)
        .map_err(|e| CliError::Parse(format!("inner JSON: {e} — head: {}", &cleaned.chars().take(160).collect::<String>())))?;

    Ok(SceneResult {
        parsed,
        cost_usd: final_cost,
        duration_ms: final_duration_ms,
    })
}

fn strip_code_fence(s: &str) -> String {
    let trimmed = s.trim();
    if let Some(rest) = trimmed.strip_prefix("```json") {
        return rest.trim_end_matches("```").trim().to_string();
    }
    if let Some(rest) = trimmed.strip_prefix("```") {
        return rest.trim_end_matches("```").trim().to_string();
    }
    trimmed.to_string()
}
