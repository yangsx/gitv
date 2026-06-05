use gitv_git_core::gix_repo::GixRepository;
use gitv_git_core::models::{CommitInfo, Oid};
use gitv_git_core::repository::Repository;
use gitv_git_core::watcher::{RepositoryWatcher, WatchEvent};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::mpsc;
use tauri::Emitter;
use tracing::instrument;

#[derive(Debug)]
pub struct RepoWatchers {
    watchers: Mutex<HashMap<PathBuf, RepositoryWatcher>>,
}

impl RepoWatchers {
    pub fn new() -> Self {
        Self {
            watchers: Mutex::new(HashMap::new()),
        }
    }

    pub fn insert(&self, path: PathBuf, watcher: RepositoryWatcher) {
        if let Ok(mut w) = self.watchers.lock() {
            w.insert(path, watcher);
        }
    }

    pub fn remove(&self, path: &PathBuf) {
        if let Ok(mut w) = self.watchers.lock() {
            w.remove(path);
        }
    }
}

#[derive(Clone, serde::Serialize)]
pub struct RepoChangedPayload {
    pub event_type: String,
    pub timestamp: i64,
}

#[tauri::command]
#[instrument(skip(path, app), fields(command = "start_watching"))]
pub fn start_watching(
    path: String,
    app: tauri::AppHandle,
    watchers: tauri::State<'_, RepoWatchers>,
) -> Result<(), String> {
    let repo_path = PathBuf::from(&path);
    let repo = GixRepository::open(&repo_path).map_err(|e| e.to_string())?;
    let thread_repo = repo.thread_local();
    let git_dir = thread_repo.git_dir().to_path_buf();

    let (tx, rx) = mpsc::channel::<WatchEvent>();

    let watcher = RepositoryWatcher::start(&git_dir, tx)
        .map_err(|e| format!("failed to start watcher: {e}"))?;

    watchers.insert(repo_path.clone(), watcher);

    std::thread::spawn(move || {
        for event in rx {
            let event_type = match event {
                WatchEvent::RefsChanged => "refs_changed",
                WatchEvent::HeadChanged => "head_changed",
                WatchEvent::IndexChanged => "index_changed",
            }
            .to_string();

            let payload = RepoChangedPayload {
                event_type,
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as i64,
            };

            if app.emit("repo-changed", payload).is_err() {
                break;
            }
        }
    });

    Ok(())
}

#[tauri::command]
#[instrument(skip(path), fields(command = "stop_watching"))]
pub fn stop_watching(path: String, watchers: tauri::State<'_, RepoWatchers>) -> Result<(), String> {
    let repo_path = PathBuf::from(&path);
    watchers.remove(&repo_path);
    Ok(())
}

#[derive(Clone, serde::Serialize)]
pub struct NewCommitsResult {
    pub commits: Vec<CommitInfo>,
    pub history_rewritten: bool,
}

#[tauri::command]
#[instrument(skip(path), fields(command = "get_new_commits"))]
pub fn get_new_commits(path: String, since_oid: String) -> Result<NewCommitsResult, String> {
    let repo_path = PathBuf::from(&path);
    let repo = GixRepository::open(&repo_path).map_err(|e| e.to_string())?;

    let all_commits = repo.commits(None).map_err(|e| e.to_string())?;

    let since = Oid::from_hex(&since_oid).map_err(|e| e.to_string())?;

    let seen: std::collections::HashSet<Oid> = all_commits.iter().map(|c| c.oid).collect();

    if !seen.contains(&since) {
        return Ok(NewCommitsResult {
            commits: all_commits,
            history_rewritten: true,
        });
    }

    let mut new_commits = Vec::new();
    for c in &all_commits {
        if c.oid == since {
            break;
        }
        new_commits.push(c.clone());
    }

    Ok(NewCommitsResult {
        commits: new_commits,
        history_rewritten: false,
    })
}
