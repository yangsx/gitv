use gitv_git_core::models::{CommitBatch, CommitFilter};
use gitv_git_core::stream::CommitStream;
use std::path::PathBuf;
use tauri::{Emitter, State};
use tracing::instrument;

use crate::state::AppState;

#[tauri::command]
#[instrument(skip(state, path, filter), fields(command = "get_commits"))]
pub fn get_commits(
    state: State<'_, AppState>,
    path: String,
    filter: Option<CommitFilter>,
) -> Result<Vec<gitv_git_core::models::CommitInfo>, String> {
    let repo_path = PathBuf::from(&path);
    let repo = state.get_repo(&repo_path).map_err(|e| e.to_string())?;
    let repo = repo.as_ref().clone();
    let commit_filter = filter.unwrap_or_default();
    let mut stream = CommitStream::new(Box::new(repo), commit_filter);

    let mut all = Vec::new();
    while stream.has_more() {
        let batch = stream.next_batch(100).map_err(|e| e.to_string())?;
        match batch {
            Some(commits) => {
                all.extend(commits);
            }
            None => break,
        }
    }
    Ok(all)
}

#[tauri::command]
#[instrument(skip(state, path, filter, window), fields(command = "stream_commits"))]
pub fn stream_commits(
    state: State<'_, AppState>,
    path: String,
    filter: Option<CommitFilter>,
    window: tauri::Window,
) -> Result<(), String> {
    let repo_path = PathBuf::from(&path);
    let repo = state.get_repo(&repo_path).map_err(|e| e.to_string())?;
    let repo = repo.as_ref().clone();
    let commit_filter = filter.unwrap_or_default();
    let mut stream = CommitStream::new(Box::new(repo), commit_filter);

    let mut batch_index = 0usize;
    while stream.has_more() {
        let batch = stream.next_batch(100).map_err(|e| e.to_string())?;
        match batch {
            Some(commits) => {
                let payload = CommitBatch {
                    commits,
                    batch_index,
                    has_more: stream.has_more(),
                };
                window
                    .emit("commit-batch", payload)
                    .map_err(|e| e.to_string())?;
                batch_index += 1;
            }
            None => break,
        }
    }
    Ok(())
}
