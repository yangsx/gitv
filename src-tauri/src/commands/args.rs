use serde::Serialize;
use std::path::PathBuf;
use std::sync::OnceLock;
use tracing::instrument;

static STARTUP_PATHS: OnceLock<Vec<String>> = OnceLock::new();
static DEBUG_OVERLAY: OnceLock<bool> = OnceLock::new();
static LOG_PATH: OnceLock<String> = OnceLock::new();

pub fn init_startup_paths(paths: Vec<String>) {
    let _ = STARTUP_PATHS.set(paths);
}

pub fn set_debug_overlay(enabled: bool) {
    let _ = DEBUG_OVERLAY.set(enabled);
}

pub fn set_log_path(path: PathBuf) {
    let _ = LOG_PATH.set(path.to_string_lossy().to_string());
}

#[derive(Serialize)]
pub struct StartupInfo {
    pub paths: Vec<String>,
    pub debug_overlay_enabled: bool,
    pub log_path: String,
}

#[tauri::command]
#[instrument(fields(command = "get_startup_info"))]
pub fn get_startup_info() -> StartupInfo {
    StartupInfo {
        paths: STARTUP_PATHS.get().cloned().unwrap_or_default(),
        debug_overlay_enabled: DEBUG_OVERLAY.get().copied().unwrap_or(false),
        log_path: LOG_PATH.get().cloned().unwrap_or_default(),
    }
}

pub fn get_log_path_str() -> String {
    LOG_PATH.get().cloned().unwrap_or_default()
}
