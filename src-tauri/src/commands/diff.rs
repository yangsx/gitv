use gitv_git_core::gix_repo::GixRepository;
use gitv_git_core::models::{DiffMode, Oid, WhitespaceMode};
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
    whitespace: Option<String>,
) -> Result<gitv_git_core::models::DiffSummary, String> {
    let repo_path = PathBuf::from(&path);
    let repo = GixRepository::open(&repo_path).map_err(|e| e.to_string())?;
    let from_oid = from
        .map(|s| Oid::from_hex(&s))
        .transpose()
        .map_err(|e| e.to_string())?;
    let to_oid = Oid::from_hex(&to).map_err(|e| e.to_string())?;
    let ws = parse_whitespace(whitespace.as_deref());
    repo.diff_summary(from_oid, to_oid, ws)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_file_diff(
    path: String,
    from: Option<String>,
    to: String,
    file_path: String,
    diff_mode: Option<String>,
    whitespace: Option<String>,
) -> Result<gitv_git_core::models::FileDiff, String> {
    let repo_path = PathBuf::from(&path);
    let repo = GixRepository::open(&repo_path).map_err(|e| e.to_string())?;
    let from_oid = from
        .map(|s| Oid::from_hex(&s))
        .transpose()
        .map_err(|e| e.to_string())?;
    let to_oid = Oid::from_hex(&to).map_err(|e| e.to_string())?;
    let mode = parse_diff_mode(diff_mode.as_deref());
    let ws = parse_whitespace(whitespace.as_deref());
    repo.file_diff(from_oid, to_oid, std::path::Path::new(&file_path), mode, ws)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_file_tree(
    path: String,
    at_commit: Option<String>,
) -> Result<gitv_git_core::models::FileTreeNode, String> {
    let repo_path = PathBuf::from(&path);
    let repo = GixRepository::open(&repo_path).map_err(|e| e.to_string())?;
    let oid = at_commit
        .map(|s| Oid::from_hex(&s))
        .transpose()
        .map_err(|e| e.to_string())?;
    repo.file_tree(oid).map_err(|e| e.to_string())
}

fn parse_diff_mode(s: Option<&str>) -> DiffMode {
    match s.unwrap_or("normal") {
        "word-diff" => DiffMode::WordDiff,
        "stat-only" => DiffMode::StatOnly,
        _ => DiffMode::Normal,
    }
}

#[tauri::command]
pub fn get_file_history(
    path: String,
    file_path: String,
    max_count: Option<usize>,
) -> Result<Vec<gitv_git_core::models::FileHistoryEntry>, String> {
    let repo_path = PathBuf::from(&path);
    let repo = GixRepository::open(&repo_path).map_err(|e| e.to_string())?;
    repo.file_history(std::path::Path::new(&file_path), max_count)
        .map_err(|e| e.to_string())
}

fn parse_whitespace(s: Option<&str>) -> WhitespaceMode {
    match s.unwrap_or("none") {
        "ignore-space-change" => WhitespaceMode::IgnoreSpaceChange,
        "ignore-all-space" => WhitespaceMode::IgnoreAllSpace,
        "ignore-blank-lines" => WhitespaceMode::IgnoreBlankLines,
        _ => WhitespaceMode::None,
    }
}
