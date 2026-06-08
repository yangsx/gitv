use crate::state::AppState;
use gitv_git_core::models::Oid;
use gitv_git_core::repository::Repository;
use std::path::PathBuf;
use tauri::State;
use tracing::instrument;

#[tauri::command]
#[instrument(skip(state, path), fields(command = "get_reflog"))]
pub fn get_reflog(
    state: State<'_, AppState>,
    path: String,
    ref_name: Option<String>,
) -> Result<Vec<gitv_git_core::models::ReflogEntry>, String> {
    let repo_path = PathBuf::from(&path);
    let repo = state.get_repo(&repo_path)?;
    repo.reflog(ref_name.as_deref()).map_err(|e| e.to_string())
}

#[tauri::command]
#[instrument(skip(state, path), fields(command = "get_stash_list"))]
pub fn get_stash_list(
    state: State<'_, AppState>,
    path: String,
) -> Result<Vec<gitv_git_core::models::StashEntry>, String> {
    let repo_path = PathBuf::from(&path);
    let repo = state.get_repo(&repo_path)?;
    repo.stash_list().map_err(|e| e.to_string())
}

#[tauri::command]
#[instrument(skip(state, path), fields(command = "get_stash_diff"))]
pub fn get_stash_diff(
    state: State<'_, AppState>,
    path: String,
    stash_index: usize,
) -> Result<gitv_git_core::models::FileDiff, String> {
    let repo_path = PathBuf::from(&path);
    let repo = state.get_repo(&repo_path)?;
    repo.stash_diff(stash_index).map_err(|e| e.to_string())
}

#[tauri::command]
#[instrument(skip(state, path), fields(command = "get_stash_split_diff"))]
pub fn get_stash_split_diff(
    state: State<'_, AppState>,
    path: String,
    stash_index: usize,
) -> Result<gitv_git_core::models::StashSplitDiff, String> {
    let repo_path = PathBuf::from(&path);
    let repo = state.get_repo(&repo_path)?;
    repo.stash_split_diff(stash_index)
        .map_err(|e| e.to_string())
}

#[tauri::command]
#[instrument(skip(state, path, file_path), fields(command = "get_blame"))]
pub fn get_blame(
    state: State<'_, AppState>,
    path: String,
    file_path: String,
    at_commit: Option<String>,
) -> Result<gitv_git_core::models::Blame, String> {
    let repo_path = PathBuf::from(&path);
    let repo = state.get_repo(&repo_path)?;
    let oid = at_commit
        .map(|s| Oid::from_hex(&s))
        .transpose()
        .map_err(|e| e.to_string())?;
    repo.blame(std::path::Path::new(&file_path), oid)
        .map_err(|e| e.to_string())
}
