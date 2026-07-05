use crate::state::AppState;
use gitv_git_core::models::{DiffMode, Oid, WhitespaceMode};
use gitv_git_core::repository::Repository;
use std::path::PathBuf;
use tauri::State;
use tracing::instrument;

#[tauri::command]
#[instrument(skip(state, path), fields(command = "get_commit_details"))]
pub async fn get_commit_details(
    state: State<'_, AppState>,
    path: String,
    oid: String,
    include_counts: Option<bool>,
) -> Result<gitv_git_core::models::CommitDetails, String> {
    let repo_path = PathBuf::from(&path);
    let repo = state.get_repo(&repo_path)?;
    let commit_oid = Oid::from_hex(&oid).map_err(|e| e.to_string())?;
    let include = include_counts.unwrap_or(false);
    tauri::async_runtime::spawn_blocking(move || {
        repo.commit_details(commit_oid, include)
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
#[instrument(skip(state, path), fields(command = "get_combined_commit_details"))]
pub async fn get_combined_commit_details(
    state: State<'_, AppState>,
    path: String,
    oid: String,
    include_counts: Option<bool>,
) -> Result<gitv_git_core::models::CommitDetails, String> {
    let repo_path = PathBuf::from(&path);
    let repo = state.get_repo(&repo_path)?;
    let commit_oid = Oid::from_hex(&oid).map_err(|e| e.to_string())?;
    let include = include_counts.unwrap_or(false);
    tauri::async_runtime::spawn_blocking(move || {
        repo.combined_diff(commit_oid, include)
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
#[instrument(skip(state, path), fields(command = "get_commit_file_counts"))]
pub async fn get_commit_file_counts(
    state: State<'_, AppState>,
    path: String,
    oid: String,
) -> Result<Vec<gitv_git_core::models::FileLineStats>, String> {
    let repo_path = PathBuf::from(&path);
    let repo = state.get_repo(&repo_path)?;
    let commit_oid = Oid::from_hex(&oid).map_err(|e| e.to_string())?;
    tauri::async_runtime::spawn_blocking(move || {
        repo.commit_file_counts(commit_oid)
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
#[instrument(skip(state, path), fields(command = "get_diff"))]
pub fn get_diff(
    state: State<'_, AppState>,
    path: String,
    from: Option<String>,
    to: String,
    whitespace: Option<String>,
) -> Result<gitv_git_core::models::DiffSummary, String> {
    let repo_path = PathBuf::from(&path);
    let repo = state.get_repo(&repo_path)?;
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
#[allow(clippy::too_many_arguments)]
#[instrument(skip(state, path, file_path), fields(command = "get_file_diff"))]
pub fn get_file_diff(
    state: State<'_, AppState>,
    path: String,
    from: Option<String>,
    to: String,
    file_path: String,
    diff_mode: Option<String>,
    whitespace: Option<String>,
    full: Option<bool>,
) -> Result<gitv_git_core::models::FileDiff, String> {
    let repo_path = PathBuf::from(&path);
    let repo = state.get_repo(&repo_path)?;
    let from_oid = from
        .map(|s| Oid::from_hex(&s))
        .transpose()
        .map_err(|e| e.to_string())?;
    let to_oid = Oid::from_hex(&to).map_err(|e| e.to_string())?;
    let mode = parse_diff_mode(diff_mode.as_deref());
    let ws = parse_whitespace(whitespace.as_deref());
    let line_limit = if full.unwrap_or(false) {
        None
    } else {
        Some(gitv_git_core::DIFF_LINE_LIMIT)
    };
    repo.file_diff_limited(
        from_oid,
        to_oid,
        std::path::Path::new(&file_path),
        mode,
        ws,
        line_limit,
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
#[instrument(skip(state, path), fields(command = "get_file_tree"))]
pub fn get_file_tree(
    state: State<'_, AppState>,
    path: String,
    at_commit: Option<String>,
) -> Result<gitv_git_core::models::FileTreeNode, String> {
    let repo_path = PathBuf::from(&path);
    let repo = state.get_repo(&repo_path)?;
    let oid = at_commit
        .map(|s| Oid::from_hex(&s))
        .transpose()
        .map_err(|e| e.to_string())?;
    repo.file_tree(oid).map_err(|e| e.to_string())
}

#[tauri::command]
#[allow(clippy::too_many_arguments)]
#[instrument(
    skip(state, path, file_path),
    fields(command = "get_combined_file_diff")
)]
pub fn get_combined_file_diff(
    state: State<'_, AppState>,
    path: String,
    oid: String,
    file_path: String,
    diff_mode: Option<String>,
    whitespace: Option<String>,
    full: Option<bool>,
) -> Result<gitv_git_core::models::FileDiff, String> {
    let repo_path = PathBuf::from(&path);
    let repo = state.get_repo(&repo_path)?;
    let commit_oid = Oid::from_hex(&oid).map_err(|e| e.to_string())?;
    let mode = parse_diff_mode(diff_mode.as_deref());
    let ws = parse_whitespace(whitespace.as_deref());
    let line_limit = if full.unwrap_or(false) {
        None
    } else {
        Some(gitv_git_core::DIFF_LINE_LIMIT)
    };
    let result = repo
        .combined_file_diff(
            commit_oid,
            std::path::Path::new(&file_path),
            mode,
            ws,
            line_limit,
        )
        .map_err(|e| e.to_string())?;
    Ok(result)
}

fn parse_diff_mode(s: Option<&str>) -> DiffMode {
    match s.unwrap_or("normal") {
        "word-diff" => DiffMode::WordDiff,
        "stat-only" => DiffMode::StatOnly,
        _ => DiffMode::Normal,
    }
}

#[tauri::command]
#[instrument(skip(state, path, file_path), fields(command = "get_file_history"))]
pub fn get_file_history(
    state: State<'_, AppState>,
    path: String,
    file_path: String,
    max_count: Option<usize>,
) -> Result<Vec<gitv_git_core::models::FileHistoryEntry>, String> {
    let repo_path = PathBuf::from(&path);
    let repo = state.get_repo(&repo_path)?;
    repo.file_history(std::path::Path::new(&file_path), max_count)
        .map_err(|e| e.to_string())
}

#[tauri::command]
#[instrument(skip(state, path, file_path), fields(command = "get_blob_content"))]
pub fn get_blob_content(
    state: State<'_, AppState>,
    path: String,
    at_commit: String,
    file_path: String,
) -> Result<String, String> {
    let repo_path = PathBuf::from(&path);
    let repo = state.get_repo(&repo_path)?;
    let file_path = std::path::Path::new(&file_path);

    match at_commit.as_str() {
        "__staged__" => repo.blob_content_staged(file_path),
        "__unstaged__" => repo.blob_content_worktree(file_path),
        hex => {
            let oid = Oid::from_hex(hex).map_err(|e| e.to_string())?;
            repo.blob_content(oid, file_path)
        }
    }
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

#[tauri::command]
#[instrument(skip(state, path), fields(command = "get_working_changes"))]
pub async fn get_working_changes(
    state: State<'_, AppState>,
    path: String,
) -> Result<gitv_git_core::models::WorkingChangesDiff, String> {
    let repo_path = PathBuf::from(&path);
    let repo = state.get_repo(&repo_path)?;
    tauri::async_runtime::spawn_blocking(move || {
        repo.working_changes_diff().map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
#[instrument(skip(state, path), fields(command = "get_working_changes_diffs"))]
pub fn get_working_changes_diffs(
    state: State<'_, AppState>,
    path: String,
    staged: bool,
    diff_mode: Option<String>,
    whitespace: Option<String>,
) -> Result<Vec<gitv_git_core::models::FileDiff>, String> {
    let repo_path = PathBuf::from(&path);
    let repo = state.get_repo(&repo_path)?;
    let mode = parse_diff_mode(diff_mode.as_deref());
    let ws = parse_whitespace(whitespace.as_deref());
    repo.working_changes_file_diffs(staged, mode, ws)
        .map_err(|e| e.to_string())
}

#[tauri::command]
#[instrument(
    skip(state, path),
    fields(command = "get_working_changes_combined_diff")
)]
pub fn get_working_changes_combined_diff(
    state: State<'_, AppState>,
    path: String,
    diff_mode: Option<String>,
    whitespace: Option<String>,
) -> Result<Vec<gitv_git_core::models::FileDiff>, String> {
    let repo_path = PathBuf::from(&path);
    let repo = state.get_repo(&repo_path)?;
    let mode = parse_diff_mode(diff_mode.as_deref());
    let ws = parse_whitespace(whitespace.as_deref());
    repo.working_changes_combined_diff(mode, ws)
        .map_err(|e| e.to_string())
}
