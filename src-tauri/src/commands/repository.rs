use gitv_git_core::models::RepositoryInfo;
use gitv_git_core::repository;
use std::path::PathBuf;

#[tauri::command]
pub async fn open_repository(path: String) -> Result<RepositoryInfo, String> {
    let repo_path = PathBuf::from(&path);
    let repo = repository::open(&repo_path).map_err(|e| e.to_string())?;
    repo.info().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_recent_repositories()
-> Result<Vec<gitv_git_core::models::RecentRepository>, String> {
    Ok(Vec::new())
}
