use gitv_git_core::gix_repo::GixRepository;
use gitv_git_core::models::Oid;
use gitv_git_core::repository::Repository;
use std::path::PathBuf;

#[tauri::command]
pub fn get_commit_details(
    path: String,
    oid: String,
) -> Result<gitv_git_core::models::CommitDetails, String> {
    let repo_path = PathBuf::from(&path);
    let repo = GixRepository::open(&repo_path).map_err(|e| e.to_string())?;
    let commit_oid = Oid::from_hex(&oid).map_err(|e| e.to_string())?;
    repo.commit(commit_oid).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_diff(
    path: String,
    from: Option<String>,
    to: String,
) -> Result<gitv_git_core::models::DiffSummary, String> {
    let repo_path = PathBuf::from(&path);
    let repo = GixRepository::open(&repo_path).map_err(|e| e.to_string())?;
    let from_oid = from
        .map(|s| Oid::from_hex(&s))
        .transpose()
        .map_err(|e| e.to_string())?;
    let to_oid = Oid::from_hex(&to).map_err(|e| e.to_string())?;
    repo.diff_summary(from_oid, to_oid)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_file_diff(
    path: String,
    from: Option<String>,
    to: String,
    file_path: String,
) -> Result<gitv_git_core::models::FileDiff, String> {
    let repo_path = PathBuf::from(&path);
    let repo = GixRepository::open(&repo_path).map_err(|e| e.to_string())?;
    let from_oid = from
        .map(|s| Oid::from_hex(&s))
        .transpose()
        .map_err(|e| e.to_string())?;
    let to_oid = Oid::from_hex(&to).map_err(|e| e.to_string())?;
    repo.file_diff(from_oid, to_oid, std::path::Path::new(&file_path))
        .map_err(|e| e.to_string())
}
