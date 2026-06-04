use gitv_git_core::search::SearchQuery;
use std::path::PathBuf;
use tauri::State;
use tracing::instrument;

use crate::state::AppState;

#[tauri::command]
#[instrument(skip(state, path), fields(command = "search_commits"))]
pub fn search_commits(
    state: State<AppState>,
    path: String,
    query: SearchQuery,
) -> Result<Vec<gitv_git_core::search::SearchResult>, String> {
    let repo_path = PathBuf::from(&path);
    state.search(&repo_path, &query)
}
