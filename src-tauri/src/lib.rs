mod commands;
mod state;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(state::AppState::new())
        .invoke_handler(tauri::generate_handler![
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
            commands::reflog_stash::get_reflog,
            commands::reflog_stash::get_stash_list,
            commands::reflog_stash::get_stash_diff,
            commands::reflog_stash::get_stash_split_diff,
            commands::reflog_stash::get_blame,
            commands::saved_searches::save_search,
            commands::saved_searches::list_saved_searches,
            commands::saved_searches::delete_saved_search,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
