// Windows: ALWAYS hide the console window (debug + release).
// Rule #15 — a shipping desktop app must look like one.
#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

fn main() {
    firefly_indexer_lib::run()
}
