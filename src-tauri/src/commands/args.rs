use serde::Serialize;
use std::sync::OnceLock;
use tracing::instrument;

static STARTUP_PATHS: OnceLock<Vec<String>> = OnceLock::new();

pub fn init_startup_paths(paths: Vec<String>) {
    let _ = STARTUP_PATHS.set(paths);
}

#[derive(Serialize)]
pub struct StartupInfo {
    pub paths: Vec<String>,
}

#[tauri::command]
#[instrument(fields(command = "get_startup_info"))]
pub fn get_startup_info() -> StartupInfo {
    StartupInfo {
        paths: STARTUP_PATHS.get().cloned().unwrap_or_default(),
    }
}
