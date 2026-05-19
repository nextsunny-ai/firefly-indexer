// Gemini CLI subprocess wrapper — the second vision engine.
//
// The app SHIPS its own Node.js runtime + the Gemini CLI bundle as bundled
// resources (src-tauri/resources/gem/, fetched by CI). So the user installs
// nothing for Gemini — only a one-time browser login. If the bundle is ever
// missing (dev builds), it falls back to a system-installed gemini.
//
// Verified (2026-05-19): same indexer prompt + Gemini engine classifies
// gender correctly where Claude (Haiku/Sonnet/Opus) misclassified.

use crate::claude_cli::SceneResult;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;
use tokio::process::Command;
use tokio::time::timeout;

// tokio::process::Command exposes creation_flags() inherently on Windows.
#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

const NODE_EXE: &str = if cfg!(target_os = "windows") { "node.exe" } else { "node" };

/// The Node binary bundled inside the app (resources/gem/node[.exe]).
pub fn bundled_node(res_dir: &Path) -> Option<PathBuf> {
    let p = res_dir.join("resources").join("gem").join(NODE_EXE);
    p.is_file().then_some(p)
}

/// The Gemini CLI entry bundled inside the app (resources/gem/cli/bundle/gemini.js).
pub fn bundled_gemini(res_dir: &Path) -> Option<PathBuf> {
    let p = res_dir
        .join("resources").join("gem").join("cli").join("bundle").join("gemini.js");
    p.is_file().then_some(p)
}

/// Locate a `node` executable on the system (fallback when not bundled).
fn system_node() -> Option<PathBuf> {
    if let Some(path_var) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&path_var) {
            let p = dir.join(NODE_EXE);
            if p.is_file() {
                return Some(p);
            }
        }
    }
    let mut cand: Vec<PathBuf> = Vec::new();
    #[cfg(target_os = "windows")]
    {
        cand.push(PathBuf::from("C:\\Program Files\\nodejs\\node.exe"));
    }
    #[cfg(not(target_os = "windows"))]
    {
        cand.push(PathBuf::from("/usr/local/bin/node"));
        cand.push(PathBuf::from("/opt/homebrew/bin/node"));
        cand.push(PathBuf::from("/usr/bin/node"));
    }
    cand.into_iter().find(|p| p.is_file())
}

/// Locate a system-installed Gemini CLI bundle (fallback when not bundled).
fn system_gemini() -> Option<PathBuf> {
    let rel = Path::new("node_modules")
        .join("@google").join("gemini-cli").join("bundle").join("gemini.js");
    let mut roots: Vec<PathBuf> = Vec::new();
    #[cfg(target_os = "windows")]
    {
        if let Some(appdata) = std::env::var_os("APPDATA") {
            roots.push(PathBuf::from(&appdata).join("npm"));
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        roots.push(PathBuf::from("/usr/local/lib"));
        roots.push(PathBuf::from("/opt/homebrew/lib"));
    }
    if let Some(h) = dirs::home_dir() {
        roots.push(h.join(".npm-global").join("lib"));
        roots.push(h.join(".npm-global"));
    }
    roots.into_iter().map(|r| r.join(&rel)).find(|p| p.is_file())
}

/// Resolve (node, gemini.js) — bundled runtime first, system install second.
pub fn resolve(res_dir: Option<&Path>) -> Option<(PathBuf, PathBuf)> {
    if let Some(rd) = res_dir {
        if let (Some(n), Some(g)) = (bundled_node(rd), bundled_gemini(rd)) {
            return Some((n, g));
        }
    }
    match (system_node(), system_gemini()) {
        (Some(n), Some(g)) => Some((n, g)),
        _ => None,
    }
}

pub fn is_installed(res_dir: Option<&Path>) -> bool {
    resolve(res_dir).is_some()
}

/// Gemini CLI keeps personal-OAuth credentials in ~/.gemini/oauth_creds.json.
pub fn is_logged_in() -> bool {
    if let Some(h) = dirs::home_dir() {
        return h.join(".gemini").join("oauth_creds.json").is_file();
    }
    false
}

/// Pull the JSON object out of an assistant reply (strip ``` fences / prose).
fn extract_json(text: &str) -> Result<serde_json::Value, String> {
    let t = text.trim();
    let body = if let Some(rest) = t.strip_prefix("```") {
        let rest = rest.strip_prefix("json").unwrap_or(rest);
        rest.rsplit_once("```").map(|(a, _)| a).unwrap_or(rest)
    } else {
        t
    };
    let start = body.find('{').ok_or("Gemini 응답에 JSON 객체가 없습니다")?;
    let end = body.rfind('}').ok_or("Gemini 응답 JSON이 닫히지 않았습니다")?;
    serde_json::from_str(&body[start..=end]).map_err(|e| format!("Gemini JSON 파싱 실패: {e}"))
}

/// Run one scene through Gemini. `scene_dir` holds the scene's images (the
/// prompt references them as `@filename`) and is the process working dir.
pub async fn run_scene(
    user_msg: &str,
    scene_dir: &str,
    timeout_secs: u64,
    res_dir: Option<&Path>,
) -> Result<SceneResult, String> {
    let (node, gemini) = resolve(res_dir)
        .ok_or("Gemini 런타임을 찾을 수 없습니다 (앱 재설치가 필요할 수 있습니다).")?;

    let started = std::time::Instant::now();
    let mut cmd = Command::new(&node);
    cmd.arg(&gemini)
        .arg("-p").arg(user_msg)
        .arg("--output-format").arg("json")
        .arg("--approval-mode").arg("yolo")
        .arg("--skip-trust")
        .current_dir(scene_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(target_os = "windows")]
    cmd.creation_flags(CREATE_NO_WINDOW);

    let out = timeout(Duration::from_secs(timeout_secs), cmd.output())
        .await
        .map_err(|_| format!("Gemini 호출 시간 초과 ({timeout_secs}s)"))?
        .map_err(|e| format!("Gemini 실행 실패: {e}"))?;

    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);

    let start = stdout.find('{')
        .ok_or_else(|| format!("Gemini 출력이 비었습니다. {}", stderr.trim()))?;
    let end = stdout.rfind('}').unwrap_or(stdout.len().saturating_sub(1));
    let outer: serde_json::Value = serde_json::from_str(stdout[start..=end].trim())
        .map_err(|e| format!("Gemini 출력 파싱 실패: {e} — {}", stderr.trim()))?;

    if let Some(err) = outer.get("error") {
        let msg = err.get("message").and_then(|v| v.as_str()).unwrap_or("");
        return Err(format!("Gemini 오류: {msg}"));
    }
    let resp = outer.get("response").and_then(|v| v.as_str())
        .ok_or("Gemini 응답에 response 필드가 없습니다")?;

    let parsed = extract_json(resp)?;
    Ok(SceneResult {
        parsed,
        cost_usd: 0.0,
        duration_ms: started.elapsed().as_millis() as u64,
    })
}
