use std::backtrace::Backtrace;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::OnceLock;
use tracing::instrument;

use serde::Serialize;

use crate::commands::args;

/// Result of a self-test run: full graph layout + diagnostics.
#[derive(Serialize)]
pub struct SelfTestResult {
    /// Wall-clock time for the entire operation (open repo, load commits, compute layout, verify)
    pub timing_ms: f64,
    /// Number of nodes in the layout
    pub node_count: usize,
    /// Number of edges in the layout
    pub edge_count: usize,
    /// Total columns used
    pub total_columns: usize,
    /// Number of pass-through errors found by verify()
    pub error_count: usize,
    /// Up to 1000 error messages
    pub errors: Vec<String>,
    // --- Diagnostics ---
    pub max_concurrent_threads: usize,
    pub column_waste: usize,
    pub total_waypoints: usize,
    pub max_waypoints_per_edge: usize,
    pub straight_edges: usize,
    pub branch_edges: usize,
    pub merge_edges: usize,
    pub arrow_gap_count: usize,
    /// Comma-separated column shift histogram: "0:5123,1:1845,2:192"
    pub column_shift_histogram: String,
    /// Comma-separated row thread histogram: "1:234,2:567,3:89"
    pub row_thread_histogram: String,
    // --- Topology ---
    pub total_commits: usize,
    pub merge_count: usize,
    pub branching_factor_histogram: Vec<usize>,
    pub longest_chain: usize,
    pub fork_point_count: usize,
}

static CRASH_DIR: OnceLock<PathBuf> = OnceLock::new();
static CRASH_LOCK: Mutex<()> = Mutex::new(());

const CRASH_RETENTION_COUNT: usize = 5;

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
        evict_old_crashes(dir, CRASH_RETENTION_COUNT);
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

#[tauri::command]
#[instrument(fields(command = "run_self_test"))]
pub async fn run_self_test(path: String) -> Result<SelfTestResult, String> {
    use gitv_git_core::self_test::run_self_test as core_run;

    let core = tauri::async_runtime::spawn_blocking(move || core_run(Path::new(&path)))
        .await
        .map_err(|e| format!("self-test panicked: {e}"))?
        .map_err(|e| format!("self-test failed: {e}"))?;

    Ok(SelfTestResult {
        timing_ms: core.timing_ms,
        node_count: core.node_count,
        edge_count: core.edge_count,
        total_columns: core.total_columns,
        error_count: core.error_count,
        errors: core.errors,
        max_concurrent_threads: core.diagnostics.max_concurrent_threads,
        column_waste: core.diagnostics.column_waste,
        total_waypoints: core.diagnostics.total_waypoints,
        max_waypoints_per_edge: core.diagnostics.max_waypoints_per_edge,
        straight_edges: core.diagnostics.edge_type_counts.straight,
        branch_edges: core.diagnostics.edge_type_counts.branch,
        merge_edges: core.diagnostics.edge_type_counts.merge,
        arrow_gap_count: core.diagnostics.arrow_gap_count,
        column_shift_histogram: core.column_shift_histogram,
        row_thread_histogram: core.row_thread_histogram,
        total_commits: core.topology.total_commits,
        merge_count: core.topology.merge_count,
        branching_factor_histogram: core.topology.branching_factor_histogram,
        longest_chain: core.topology.longest_chain,
        fork_point_count: core.topology.fork_point_count,
    })
}
