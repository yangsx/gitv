mod cli;
mod commands;
mod state;

use std::path::PathBuf;
use tauri::Emitter;

fn init_tracing() {
    let log_dir = dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("gitv")
        .join("logs");
    std::fs::create_dir_all(&log_dir).ok();

    let file_appender = tracing_appender::rolling::RollingFileAppender::builder()
        .max_log_files(3)
        .filename_prefix("gitv")
        .filename_suffix("log")
        .rotation(tracing_appender::rolling::Rotation::DAILY)
        .build(log_dir)
        .expect("failed to create log file appender");

    let (file_writer, guard) = tracing_appender::non_blocking(file_appender);
    Box::leak(Box::new(guard));

    let default_level = if cfg!(debug_assertions) {
        "debug"
    } else {
        "info"
    };
    let filter = tracing_subscriber::EnvFilter::builder()
        .with_env_var("GITV_LOG")
        .from_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(default_level));

    use tracing_subscriber::util::SubscriberInitExt;
    let subscriber = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(file_writer)
        .with_ansi(false)
        .json()
        .finish();
    if subscriber.try_init().is_err() {
        eprintln!("gitv: tracing subscriber already initialized");
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    init_tracing();
    commands::diagnostics::install_panic_hook(env!("CARGO_PKG_VERSION"));

    let repo_paths = cli::parse_cli();
    commands::args::init_startup_paths(
        repo_paths
            .iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect(),
    );

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
            let repo_paths: Vec<String> = args
                .iter()
                .skip(1)
                .filter(|a| !a.starts_with('-'))
                .map(|a| a.to_string())
                .collect();
            if !repo_paths.is_empty() {
                let _ = app.emit("new-repo-request", repo_paths);
            } else {
                let _ = app.emit("focus-request", ());
            }
        }))
        .manage(state::AppState::new())
        .invoke_handler(tauri::generate_handler![
            commands::args::get_startup_info,
            commands::repository::open_repository,
            commands::repository::get_refs,
            commands::repository::get_recent_repositories,
            commands::commits::stream_commits,
            commands::commits::get_commits,
            commands::graph::get_graph_layout,
            commands::search::search_commits,
            commands::diff::get_commit_details,
            commands::diff::get_diff,
            commands::diff::get_file_diff,
            commands::diff::get_file_tree,
            commands::diff::get_file_history,
            commands::diff::get_blob_content,
            commands::diff::get_working_changes,
            commands::diff::get_working_changes_diffs,
            commands::reflog_stash::get_reflog,
            commands::reflog_stash::get_stash_list,
            commands::reflog_stash::get_stash_diff,
            commands::reflog_stash::get_stash_split_diff,
            commands::reflog_stash::get_blame,
            commands::saved_searches::save_search,
            commands::saved_searches::list_saved_searches,
            commands::saved_searches::delete_saved_search,
            commands::diagnostics::log_frontend_error,
            commands::diagnostics::log_frontend_message,
            commands::preferences::get_preferences,
            commands::preferences::set_preferences,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
