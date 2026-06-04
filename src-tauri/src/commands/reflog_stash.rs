use gitv_git_core::gix_repo::GixRepository;
use gitv_git_core::models::Oid;
use gitv_git_core::repository::Repository;
use std::path::PathBuf;
use tracing::instrument;

#[tauri::command]
#[instrument(skip(path), fields(command = "get_reflog"))]
pub fn get_reflog(
    path: String,
    ref_name: Option<String>,
) -> Result<Vec<gitv_git_core::models::ReflogEntry>, String> {
    let repo_path = PathBuf::from(&path);
    let repo = GixRepository::open(&repo_path).map_err(|e| e.to_string())?;
    repo.reflog(ref_name.as_deref()).map_err(|e| e.to_string())
}

#[tauri::command]
#[instrument(skip(path), fields(command = "get_stash_list"))]
pub fn get_stash_list(path: String) -> Result<Vec<gitv_git_core::models::StashEntry>, String> {
    let repo_path = PathBuf::from(&path);
    let repo = GixRepository::open(&repo_path).map_err(|e| e.to_string())?;
    repo.stash_list().map_err(|e| e.to_string())
}

#[tauri::command]
#[instrument(skip(path), fields(command = "get_stash_diff"))]
pub fn get_stash_diff(
    path: String,
    stash_index: usize,
) -> Result<gitv_git_core::models::FileDiff, String> {
    let repo_path = PathBuf::from(&path);
    let repo = GixRepository::open(&repo_path).map_err(|e| e.to_string())?;
    repo.stash_diff(stash_index).map_err(|e| e.to_string())
}

#[tauri::command]
#[instrument(skip(path), fields(command = "get_stash_split_diff"))]
pub fn get_stash_split_diff(
    path: String,
    stash_index: usize,
) -> Result<gitv_git_core::models::StashSplitDiff, String> {
    let repo_path = PathBuf::from(&path);
    let repo = GixRepository::open(&repo_path).map_err(|e| e.to_string())?;
    repo.stash_split_diff(stash_index)
        .map_err(|e| e.to_string())
}

#[tauri::command]
#[instrument(skip(path, file_path), fields(command = "get_blame"))]
pub fn get_blame(
    path: String,
    file_path: String,
    at_commit: Option<String>,
) -> Result<gitv_git_core::models::Blame, String> {
    let repo_path = PathBuf::from(&path);
    let repo = GixRepository::open(&repo_path).map_err(|e| e.to_string())?;
    let oid = at_commit
        .map(|s| Oid::from_hex(&s))
        .transpose()
        .map_err(|e| e.to_string())?;
    repo.blame(std::path::Path::new(&file_path), oid)
        .map_err(|e| e.to_string())
}
