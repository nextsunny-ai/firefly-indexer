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

// ── Open a terminal and run a setup command ───────────────────────────
#[tauri::command]
fn open_setup_terminal(command: String) -> Result<(), String> {
    // command ∈ {"install", "login"}
    // Native installer — NO Node.js required (Anthropic official scripts).

    #[cfg(target_os = "windows")]
    {
        // Windows native installer is a PowerShell one-liner.
        let ps_cmd = match command.as_str() {
            "install" => "irm https://claude.ai/install.ps1 | iex".to_string(),
            "login"   => "claude".to_string(),
            other     => other.to_string(),
        };
        std::process::Command::new("cmd")
            .args(["/C", "start", "powershell", "-NoExit", "-Command", &ps_cmd])
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "macos")]
    {
        let sh_cmd = match command.as_str() {
            "install" => "curl -fsSL https://claude.ai/install.sh | bash".to_string(),
            "login"   => "$HOME/.local/bin/claude || claude".to_string(),
            other     => other.to_string(),
        };
        let script = format!(
            "tell application \"Terminal\" to do script \"{}\"",
            sh_cmd.replace('\\', "\\\\").replace('"', "\\\"")
        );
        std::process::Command::new("osascript")
            .args(["-e", &script])
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let sh_cmd = match command.as_str() {
            "install" => "curl -fsSL https://claude.ai/install.sh | bash".to_string(),
            "login"   => "$HOME/.local/bin/claude || claude".to_string(),
            other     => other.to_string(),
        };
        for term in &["gnome-terminal", "konsole", "xterm"] {
            let mut c = std::process::Command::new(term);
            if *term == "gnome-terminal" {
                c.args(["--", "bash", "-c", &format!("{}; exec bash", sh_cmd)]);
            } else if *term == "konsole" {
                c.args(["-e", "bash", "-c", &format!("{}; exec bash", sh_cmd)]);
            } else {
                c.args(["-hold", "-e", &sh_cmd]);
            }
            if c.spawn().is_ok() { return Ok(()); }
        }
        return Err("No supported terminal found (gnome-terminal/konsole/xterm)".to_string());
    }
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
            open_setup_terminal,
            open_path,
            suggest_output_path,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
