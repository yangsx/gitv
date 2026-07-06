mod cli;
mod commands;
mod state;

use std::path::PathBuf;

const MAX_LOG_FILES: usize = 3;

fn init_tracing(cli_log_level: Option<&str>) {
    let log_dir = dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("gitv")
        .join("logs");
    std::fs::create_dir_all(&log_dir).ok();

    commands::args::set_log_path(log_dir.clone());

    let file_appender = tracing_appender::rolling::RollingFileAppender::builder()
        .max_log_files(MAX_LOG_FILES)
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

fn run_cli_self_test(cli: &cli::Cli) -> bool {
    if let Some(ref path) = cli.self_test {
        run_one_self_test(path, false, cli.self_test_max_commits);
        true
    } else if let Some(ref path) = cli.self_test_json {
        run_one_self_test(path, true, cli.self_test_max_commits);
        true
    } else {
        false
    }
}

fn run_one_self_test(path: &std::path::Path, json: bool, max_commits: Option<usize>) {
    match gitv_git_core::self_test::run_self_test(path, max_commits) {
        Ok(output) => {
            if json {
                match serde_json::to_string_pretty(&output) {
                    Ok(json_str) => println!("{json_str}"),
                    Err(e) => {
                        eprintln!("self-test serialization failed: {e}");
                        std::process::exit(1);
                    }
                }
            } else {
                eprintln!(
                    "self-test: {:.1}ms (compute {:.1}ms), {} nodes, {} edges, {} cols, {} errors",
                    output.timing_ms,
                    output.compute_ms,
                    output.node_count,
                    output.edge_count,
                    output.total_columns,
                    output.error_count,
                );
                eprintln!(
                    "  diagnostics: waste={}, waypoints={}, arrow_gaps={}, max_threads={}",
                    output.diagnostics.column_waste,
                    output.diagnostics.total_waypoints,
                    output.diagnostics.arrow_gap_count,
                    output.diagnostics.max_concurrent_threads,
                );
                eprintln!(
                    "  topology: {} merges, longest_chain={}, fork_points={}",
                    output.topology.merge_count,
                    output.topology.longest_chain,
                    output.topology.fork_point_count,
                );
                if output.error_count > 0 {
                    for err in &output.errors {
                        eprintln!("  error: {err}");
                    }
                }
                eprintln!(
                    "  hide_merges: {} nodes, {} edges, {} errors",
                    output.hide_merges_node_count,
                    output.hide_merges_edge_count,
                    output.hide_merges_error_count,
                );
                if output.hide_merges_error_count > 0 {
                    for err in &output.hide_merges_errors {
                        eprintln!("  hide_merges error: {err}");
                    }
                }
                eprintln!(
                    "  column shift histogram: {}",
                    output.column_shift_histogram
                );
                eprintln!("  row thread histogram: {}", output.row_thread_histogram);
            }
            std::process::exit(0);
        }
        Err(e) => {
            eprintln!("self-test failed: {e}");
            std::process::exit(1);
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let cli = cli::parse_cli();

    if run_cli_self_test(&cli) {
        return;
    }

    init_tracing(cli.log_level.as_deref());
    commands::diagnostics::install_panic_hook(env!("CARGO_PKG_VERSION"));

    // Canonicalize CLI paths so that `gitv .` resolves to the absolute path
    // before the frontend ever sees it.
    let canonical_paths: Vec<PathBuf> = cli
        .repo_paths
        .iter()
        .map(|p| p.canonicalize().unwrap_or_else(|_| p.clone()))
        .collect();

    // The first path (if any) is loaded in the main window.
    let startup_paths: Vec<String> = canonical_paths
        .first()
        .map(|p| vec![p.to_string_lossy().to_string()])
        .unwrap_or_default();
    commands::args::init_startup_paths(startup_paths);
    commands::args::set_debug_overlay(cli.debug_overlay);

    // Spawn additional windows for extra repo paths (Req 42.4).
    // Each path gets its own independent gitv process.
    for path in canonical_paths.iter().skip(1) {
        if let Ok(exe) = std::env::current_exe() {
            let _ = std::process::Command::new(exe).arg(path).spawn();
        }
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(state::AppState::new())
        .invoke_handler(tauri::generate_handler![
            commands::args::get_startup_info,
            commands::args::get_commit_sha,
            commands::repository::open_repository,
            commands::repository::get_refs,
            commands::repository::get_recent_repositories,
            commands::repository::save_recent_repository,
            commands::repository::open_in_new_window,
            commands::repository::set_window_title,
            commands::repository::quit_app,
            commands::commits::stream_commits,
            commands::commits::get_commits,
            commands::graph::get_graph_layout,
            commands::graph::get_initial_data,
            commands::graph::get_commits_batch,
            commands::search::search_commits,
            commands::search::cancel_patch_search,
            commands::diff::get_commit_details,
            commands::diff::get_combined_commit_details,
            commands::diff::get_combined_file_diff,
            commands::diff::get_commit_file_counts,
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
            commands::saved_searches::save_search,
            commands::saved_searches::list_saved_searches,
            commands::saved_searches::delete_saved_search,
            commands::diagnostics::log_frontend_error,
            commands::diagnostics::log_frontend_message,
            commands::diagnostics::open_log_directory,
            commands::debug::get_memory_usage,
            commands::diagnostics::run_self_test,
            commands::preferences::get_preferences,
            commands::preferences::set_preferences,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
