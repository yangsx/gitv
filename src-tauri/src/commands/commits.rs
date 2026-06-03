use gitv_git_core::gix_repo::GixRepository;
use gitv_git_core::models::{CommitBatch, CommitFilter};
use gitv_git_core::stream::CommitStream;
use std::path::PathBuf;
use tauri::Emitter;

#[tauri::command]
pub async fn get_commits(path: String, filter: Option<CommitFilter>) -> Result<Vec<gitv_git_core::models::CommitInfo>, String> {
    let repo_path = PathBuf::from(&path);
    let repo = GixRepository::open(&repo_path).map_err(|e| e.to_string())?;
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
pub async fn stream_commits(
    path: String,
    filter: Option<CommitFilter>,
    window: tauri::Window,
) -> Result<(), String> {
    let repo_path = PathBuf::from(&path);
    let repo = GixRepository::open(&repo_path).map_err(|e| e.to_string())?;
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
                let encoded = postcard::to_allocvec(&payload).map_err(|e| e.to_string())?;
                window
                    .emit("commit-batch", encoded)
                    .map_err(|e| e.to_string())?;
                batch_index += 1;
            }
            None => break,
        }
    }
    Ok(())
}
