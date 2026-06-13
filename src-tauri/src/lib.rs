mod cli;
mod commands;
mod state;
mod wgpu_state;

use std::path::PathBuf;

fn init_tracing(cli_log_level: Option<&str>) {
    let log_dir = dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("gitv")
        .join("logs");
    std::fs::create_dir_all(&log_dir).ok();

    commands::args::set_log_path(log_dir.clone());

    let file_appender = tracing_appender::rolling::RollingFileAppender::builder()
        .max_log_files(3)
        .filename_prefix("gitv")
        .filename_suffix("log")
        .rotation(tracing_appender::rolling::Rotation::DAILY)
        .build(log_dir)
        .expect("failed to create log file appender");

    let (file_writer, guard) = tracing_appender::non_blocking(file_appender);
    // Leak guard intentionally: WorkerGuard must outlive the process to keep the log writer flushing.
    // Dropping it would cause logs to be silently lost on process exit.
    Box::leak(Box::new(guard));

    let default_level = if cfg!(debug_assertions) {
        "debug"
    } else {
        "info"
    };

    let filter = tracing_subscriber::EnvFilter::builder()
        .with_env_var("GITV_LOG")
        .from_env_lossy()
        .add_directive(
            cli_log_level
                .map(|l| format!("gitv={l}"))
                .unwrap_or_else(|| format!("gitv={default_level}"))
                .parse()
                .unwrap_or_else(|_| tracing_subscriber::filter::LevelFilter::INFO.into()),
        );

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
    let cli = cli::parse_cli();

    init_tracing(cli.log_level.as_deref());
    commands::diagnostics::install_panic_hook(env!("CARGO_PKG_VERSION"));

    commands::args::init_startup_paths(
        cli.repo_paths
            .iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect(),
    );
    commands::args::set_debug_overlay(cli.debug_overlay);

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(state::AppState::new())
        .manage(wgpu_state::WgpuState::new())
        .invoke_handler(tauri::generate_handler![
            commands::args::get_startup_info,
            commands::repository::open_repository,
            commands::repository::get_refs,
            commands::repository::get_recent_repositories,
            commands::repository::save_recent_repository,
            commands::repository::open_in_new_window,
            commands::repository::quit_app,
            commands::commits::stream_commits,
            commands::commits::get_commits,
            commands::graph::get_graph_layout,
            commands::graph::get_initial_data,
            commands::search::search_commits,
            commands::search::cancel_patch_search,
            commands::diff::get_commit_details,
            commands::diff::get_diff,
            commands::diff::get_file_diff,
            commands::diff::get_file_tree,
            commands::diff::get_file_history,
            commands::diff::get_blob_content,
            commands::diff::get_working_changes,
            commands::diff::get_working_changes_diffs,
            commands::diff::get_working_changes_combined_diff,
            commands::reflog_stash::get_reflog,
            commands::reflog_stash::get_stash_list,
            commands::reflog_stash::get_stash_diff,
            commands::reflog_stash::get_stash_split_diff,
            commands::reflog_stash::get_blame,
            commands::render::render_graph,
            commands::saved_searches::save_search,
            commands::saved_searches::list_saved_searches,
            commands::saved_searches::delete_saved_search,
            commands::diagnostics::log_frontend_error,
            commands::diagnostics::log_frontend_message,
            commands::diagnostics::open_log_directory,
            commands::preferences::get_preferences,
            commands::preferences::set_preferences,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
