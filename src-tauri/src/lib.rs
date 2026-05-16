mod claude_cli;
mod prompts;
mod naming;
mod excel;
mod indexer;
mod evaluator;

use serde::Serialize;
use std::path::PathBuf;

// ── Preflight status ──────────────────────────────────────────────────
#[derive(Debug, Serialize)]
pub struct CliStatus {
    pub installed: bool,
    pub logged_in: bool,
    pub bin_path: Option<String>,
    pub install_hint: String,
    pub login_hint: String,
}

#[tauri::command]
async fn check_claude_status() -> CliStatus {
    let bin = claude_cli::find_bin();
    let installed = bin.is_some();
    let logged_in = if installed { claude_cli::is_logged_in().await } else { false };
    CliStatus {
        installed,
        logged_in,
        bin_path: bin.map(|p| p.to_string_lossy().to_string()),
        install_hint: "Open a terminal and run:  npm install -g @anthropic-ai/claude-code".to_string(),
        login_hint:   "Open a terminal and run:  claude /login   (then sign into your Pro/Max account)".to_string(),
    }
}

// ── Folder scan / rename ──────────────────────────────────────────────
#[tauri::command]
fn scan_folder(folder: String) -> naming::ScanReport {
    naming::scan_folder(&PathBuf::from(folder))
}

#[tauri::command]
fn apply_rename(req: naming::RenameRequest) -> Result<naming::RenameReport, String> {
    naming::apply_rename(&req)
}

// ── Indexing ──────────────────────────────────────────────────────────
#[tauri::command]
async fn run_indexing(app: tauri::AppHandle, req: indexer::IndexRequest) -> Result<indexer::IndexSummary, String> {
    indexer::run_indexing(app, req).await
}

// ── Evaluation ────────────────────────────────────────────────────────
#[tauri::command]
fn run_evaluation(req: evaluator::EvalRequest) -> Result<evaluator::EvalReport, String> {
    evaluator::run_evaluation(&req)
}

#[tauri::command]
async fn run_vision_evaluation(app: tauri::AppHandle, req: evaluator::VisionEvalRequest) -> Result<evaluator::VisionEvalReport, String> {
    evaluator::run_vision_evaluation(app, req).await
}

#[tauri::command]
fn cancel_vision_evaluation() {
    evaluator::cancel_vision_eval();
}

// ── Setup: install Claude Code in the background (no terminal window) ──
// Runs the official Anthropic native installer (NO Node.js) headless and
// returns the combined log. The user never sees or has to close a terminal.
#[tauri::command]
async fn run_install() -> Result<String, String> {
    use std::time::Duration;
    use tokio::process::Command;

    #[cfg(target_os = "windows")]
    let mut cmd = {
        // tokio::process::Command has its own inherent creation_flags() on Windows.
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        let mut c = Command::new("powershell");
        c.args([
            "-NoProfile", "-NonInteractive", "-Command",
            "irm https://claude.ai/install.ps1 | iex",
        ]);
        c.creation_flags(CREATE_NO_WINDOW);
        c
    };
    #[cfg(not(target_os = "windows"))]
    let mut cmd = {
        let mut c = Command::new("sh");
        c.args(["-c", "curl -fsSL https://claude.ai/install.sh | bash"]);
        c
    };

    let out = tokio::time::timeout(Duration::from_secs(300), cmd.output())
        .await
        .map_err(|_| "설치 시간 초과 (5분). 인터넷 연결을 확인해주세요.".to_string())?
        .map_err(|e| format!("설치 실행 실패: {e}"))?;

    let log = format!(
        "{}\n{}",
        String::from_utf8_lossy(&out.stdout).trim(),
        String::from_utf8_lossy(&out.stderr).trim()
    );
    if out.status.success() {
        Ok(log.trim().to_string())
    } else {
        Err(format!("설치 실패 (exit {:?}):\n{}", out.status.code(), log.trim()))
    }
}

fn home_dir() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    { std::env::var_os("USERPROFILE").map(PathBuf::from) }
    #[cfg(not(target_os = "windows"))]
    { std::env::var_os("HOME").map(PathBuf::from) }
}

// Pre-answer the `claude` first-run prompts so a non-technical user only
// has to do the browser login — no theme-picker wizard, no "trust this
// folder?" question. Merges into ~/.claude.json, preserving every other
// key; if the file is missing it is created, if it is corrupt it is left
// untouched (we never destroy an existing config).
fn preseed_claude_config() {
    let Some(home) = home_dir() else { return };
    let cfg_path = home.join(".claude.json");
    let mut root: serde_json::Value = match std::fs::read_to_string(&cfg_path) {
        Ok(s) => match serde_json::from_str(&s) {
            Ok(v) => v,
            Err(_) => return, // corrupt — do not overwrite the user's file
        },
        Err(_) => serde_json::json!({}),
    };
    let Some(obj) = root.as_object_mut() else { return };
    // skip the welcome / theme-selection wizard
    obj.insert("hasCompletedOnboarding".into(), serde_json::Value::Bool(true));
    // pre-trust the home directory (where the login shell runs) so the
    // "Do you trust the files in this folder?" prompt never appears
    let home_key = home.to_string_lossy().replace('\\', "/");
    let projects = obj
        .entry("projects")
        .or_insert_with(|| serde_json::json!({}));
    if let Some(pobj) = projects.as_object_mut() {
        let entry = pobj
            .entry(home_key)
            .or_insert_with(|| serde_json::json!({}));
        if let Some(eobj) = entry.as_object_mut() {
            eobj.insert("hasTrustDialogAccepted".into(), serde_json::Value::Bool(true));
        }
    }
    if let Ok(s) = serde_json::to_string_pretty(&root) {
        let _ = std::fs::write(&cfg_path, s);
    }
}

// ── Setup: open ONE terminal for `claude` login (browser OAuth) ────────
// Login is genuinely interactive (browser OAuth), so a terminal is needed.
// Returns a handle the frontend keeps: macOS = Terminal window id,
// Windows = powershell PID. close_setup_terminal() uses it to auto-close
// the window the moment login is confirmed — the user never touches it.
#[tauri::command]
fn open_setup_terminal(command: String) -> Result<i64, String> {
    if command != "login" {
        return Err(format!("unsupported setup command: {command}"));
    }
    // Silence the claude first-run prompts before the terminal opens.
    preseed_claude_config();

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        // Spawn powershell directly (not via `start`) so we own the PID.
        // CREATE_NEW_CONSOLE gives it a visible console for the login prompt.
        const CREATE_NEW_CONSOLE: u32 = 0x0000_0010;
        let child = std::process::Command::new("powershell")
            .args(["-NoExit", "-NoProfile", "-Command", "Set-Location $HOME; claude"])
            .creation_flags(CREATE_NEW_CONSOLE)
            .spawn()
            .map_err(|e| e.to_string())?;
        Ok(child.id() as i64)
    }
    #[cfg(target_os = "macos")]
    {
        let sh_cmd = "$HOME/.local/bin/claude || claude";
        let script = format!(
            "tell application \"Terminal\"\n\
             activate\n\
             do script \"{}\"\n\
             return id of front window\n\
             end tell",
            sh_cmd.replace('\\', "\\\\").replace('"', "\\\"")
        );
        let out = std::process::Command::new("osascript")
            .args(["-e", &script])
            .output()
            .map_err(|e| e.to_string())?;
        let id = String::from_utf8_lossy(&out.stdout).trim().parse::<i64>().unwrap_or(0);
        Ok(id)
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let sh_cmd = "$HOME/.local/bin/claude || claude";
        for term in &["gnome-terminal", "konsole", "xterm"] {
            let mut c = std::process::Command::new(term);
            if *term == "gnome-terminal" {
                c.args(["--", "bash", "-c", &format!("{sh_cmd}; exec bash")]);
            } else if *term == "konsole" {
                c.args(["-e", "bash", "-c", &format!("{sh_cmd}; exec bash")]);
            } else {
                c.args(["-hold", "-e", sh_cmd]);
            }
            if c.spawn().is_ok() { return Ok(0); }
        }
        Err("No supported terminal found (gnome-terminal/konsole/xterm)".to_string())
    }
}

// ── Setup: auto-close the login terminal once login is confirmed ───────
#[tauri::command]
fn close_setup_terminal(handle: i64) -> Result<(), String> {
    if handle == 0 {
        return Ok(());
    }
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        // /T kills the whole tree (powershell + the claude child); the
        // console window closes when its root process dies.
        let _ = std::process::Command::new("taskkill")
            .args(["/T", "/F", "/PID", &handle.to_string()])
            .creation_flags(CREATE_NO_WINDOW)
            .output();
    }
    #[cfg(target_os = "macos")]
    {
        // Kill whatever runs in that window (the claude REPL) via its tty,
        // then close it — with only the shell left, Terminal closes
        // without a "process still running" confirmation dialog.
        let script = format!(
            "tell application \"Terminal\"\n\
             set ws to (every window whose id is {handle})\n\
             if ws is not {{}} then\n\
             set w to item 1 of ws\n\
             set t to tty of selected tab of w\n\
             try\n\
             do shell script \"/usr/bin/pkill -t \" & (text 6 thru -1 of t)\n\
             end try\n\
             delay 0.3\n\
             try\n\
             close w\n\
             end try\n\
             end if\n\
             end tell"
        );
        let _ = std::process::Command::new("osascript")
            .args(["-e", &script])
            .output();
    }
    let _ = handle;
    Ok(())
}

// ── File / folder helpers ─────────────────────────────────────────────
#[tauri::command]
fn open_path(path: String) -> Result<(), String> {
    let path = PathBuf::from(path);
    open_external(&path)
}

fn open_external(path: &std::path::Path) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/C", "start", "", &path.to_string_lossy()])
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        std::process::Command::new("xdg-open")
            .arg(path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

// ── Suggested output path ────────────────────────────────────────────
#[tauri::command]
fn suggest_output_path(input_folder: String) -> String {
    let p = PathBuf::from(&input_folder);
    let stem = p.file_name().and_then(|n| n.to_str()).unwrap_or("index").to_string();
    let parent = p.parent().map(|x| x.to_path_buf()).unwrap_or_else(|| PathBuf::from("."));
    parent.join(format!("{}_index.xlsx", stem)).to_string_lossy().to_string()
}

// ── Entry ─────────────────────────────────────────────────────────────
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            check_claude_status,
            scan_folder,
            apply_rename,
            run_indexing,
            run_evaluation,
            run_vision_evaluation,
            cancel_vision_evaluation,
            run_install,
            open_setup_terminal,
            close_setup_terminal,
            open_path,
            suggest_output_path,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
