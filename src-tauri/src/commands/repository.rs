use gitv_git_core::gix_repo::GixRepository;
use gitv_git_core::models::RepositoryInfo;
use gitv_git_core::repository;
use gitv_git_core::repository::Repository;
use std::path::PathBuf;

#[tauri::command]
pub fn open_repository(path: String) -> Result<RepositoryInfo, String> {
    let repo_path = PathBuf::from(&path);
    let repo = repository::open(&repo_path).map_err(|e| e.to_string())?;
    repo.info().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_refs(path: String) -> Result<Vec<gitv_git_core::models::Ref>, String> {
    let repo_path = PathBuf::from(&path);
    let repo = GixRepository::open(&repo_path).map_err(|e| e.to_string())?;
    repo.refs().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_recent_repositories() -> Result<Vec<gitv_git_core::models::RecentRepository>, String> {
    Ok(Vec::new())
}
