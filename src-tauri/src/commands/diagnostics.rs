use std::backtrace::Backtrace;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::OnceLock;
use tracing::instrument;

use crate::commands::args;

static CRASH_DIR: OnceLock<PathBuf> = OnceLock::new();
static CRASH_LOCK: Mutex<()> = Mutex::new(());

fn get_crash_dir() -> &'static PathBuf {
    CRASH_DIR.get_or_init(|| {
        let dir = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("gitv")
            .join("crashes");
        fs::create_dir_all(&dir).ok();
        dir
    })
}

fn evict_old_crashes(dir: &Path, max: usize) {
    let mut entries: Vec<_> = if let Ok(rd) = fs::read_dir(dir) {
        rd.filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| {
                p.file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.starts_with("crash-") || n.starts_with("error-"))
            })
            .collect()
    } else {
        return;
    };
    entries.sort();
    if entries.len() > max {
        for old in entries.iter().take(entries.len() - max) {
            let _ = fs::remove_file(old);
        }
    }
}

pub fn install_panic_hook(app_version: &str) {
    let version = app_version.to_string();
    std::panic::set_hook(Box::new(move |panic_info| {
        let timestamp = chrono::Utc::now().format("%Y%m%d-%H%M%S");
        let msg = panic_info
            .payload()
            .downcast_ref::<&str>()
            .map(|s| s.to_string())
            .or_else(|| panic_info.payload().downcast_ref::<String>().cloned())
            .unwrap_or_else(|| "unknown".into());
        let location = panic_info
            .location()
            .map(|l| format!("{}:{}", l.file(), l.line()))
            .unwrap_or_else(|| "unknown".into());
        let backtrace = Backtrace::force_capture();

        let report = format!(
            "gitv crash report\n\
             version: {version}\n\
             timestamp: {timestamp}\n\
             panic: {msg}\n\
             location: {location}\n\
             backtrace:\n{backtrace}\n"
        );

        eprintln!("{report}");

        let dir = get_crash_dir();
        let path = dir.join(format!("crash-{timestamp}.txt"));
        if let Ok(mut f) = fs::File::create(&path) {
            let _ = f.write_all(report.as_bytes());
        }
        let _lock = CRASH_LOCK.lock().ok();
        evict_old_crashes(dir, 5);
    }));
}

#[tauri::command]
#[instrument(skip(message, stack), fields(command = "log_frontend_error"))]
pub fn log_frontend_error(message: String, stack: Option<String>) {
    let timestamp = chrono::Utc::now().format("%Y%m%d-%H%M%S");
    let detail = match &stack {
        Some(s) => format!("{message}\n{s}"),
        None => message.clone(),
    };
    tracing::error!("Frontend error: {detail}");

    let _lock = CRASH_LOCK.lock().ok();
    let dir = get_crash_dir();
    let report = format!(
        "gitv frontend error report\n\
         timestamp: {timestamp}\n\
         message: {message}\n\
         stack: {}\n",
        stack
            .as_deref()
            .filter(|s| !s.is_empty())
            .unwrap_or("(none)")
    );
    let path = dir.join(format!("error-{timestamp}.txt"));
    if let Ok(mut f) = fs::File::create(&path) {
        let _ = f.write_all(report.as_bytes());
    }
    evict_old_crashes(dir, 5);
}

#[tauri::command]
#[instrument(skip(message), fields(command = "log_frontend_message"))]
pub fn log_frontend_message(level: String, message: String) {
    match level.as_str() {
        "error" => tracing::error!(message),
        "warn" => tracing::warn!(message),
        _ => tracing::info!(message),
    }
}

#[tauri::command]
#[instrument(fields(command = "open_log_directory"))]
pub fn open_log_directory() -> Result<String, String> {
    let path = args::get_log_path_str();
    if path.is_empty() {
        return Err("log directory not set".into());
    }
    open::that(&path).map_err(|e| format!("failed to open log directory: {e}"))?;
    Ok(path)
}
