use crate::commands::util;
use gitv_git_core::error::GitError;
use gitv_git_core::gix_repo::GixRepository;
use gitv_git_core::models::RecentRepository;
use gitv_git_core::models::RepositoryInfo;
use gitv_git_core::repository;
use gitv_git_core::repository::Repository;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use tracing::instrument;

const RECENT_REPOS_FILENAME: &str = "recent_repos.json";
const MAX_RECENT_REPOS: usize = 10;

static RECENT_REPOS_LOCK: Mutex<()> = Mutex::new(());

fn recent_repos_path() -> Result<PathBuf, String> {
    Ok(util::config_dir()?.join(RECENT_REPOS_FILENAME))
}

fn load_recent_repos() -> Result<Vec<RecentRepository>, String> {
    #[derive(Deserialize)]
    struct Wrapper {
        repositories: Vec<RecentRepository>,
    }
    let path = recent_repos_path()?;
    if !path.exists() {
        return Ok(Vec::new());
    }
    let data =
        fs::read_to_string(&path).map_err(|e| format!("failed to read recent repos: {e}"))?;
    let wrapper: Wrapper =
        serde_json::from_str(&data).map_err(|e| format!("failed to parse recent repos: {e}"))?;
    Ok(wrapper.repositories)
}

fn save_recent_repos(repos: &[RecentRepository]) -> Result<(), String> {
    #[derive(Serialize)]
    struct Wrapper<'a> {
        repositories: &'a [RecentRepository],
    }
    let path = recent_repos_path()?;
    let data = serde_json::to_string_pretty(&Wrapper {
        repositories: repos,
    })
    .map_err(|e| format!("failed to serialize recent repos: {e}"))?;
    let tmp_path = path.with_extension("json.tmp");
    fs::write(&tmp_path, &data).map_err(|e| format!("failed to write: {e}"))?;
    fs::rename(&tmp_path, &path).map_err(|e| {
        let _ = fs::remove_file(&tmp_path);
        format!("failed to rename: {e}")
    })
}

#[tauri::command]
#[instrument(skip(path), fields(command = "open_repository"))]
pub fn open_repository(path: String) -> Result<RepositoryInfo, String> {
    let repo_path = PathBuf::from(&path);
    let repo = repository::open(&repo_path).map_err(|e| match e {
        GitError::NotAGitRepository(_) => "not_a_git_repository".to_string(),
        _ => "open_failed".to_string(),
    })?;
    repo.info().map_err(|_| "open_failed".to_string())
}

#[tauri::command]
#[instrument(skip(path), fields(command = "get_refs"))]
pub fn get_refs(path: String) -> Result<Vec<gitv_git_core::models::Ref>, String> {
    let repo_path = PathBuf::from(&path);
    let repo = GixRepository::open(&repo_path).map_err(|e| e.to_string())?;
    repo.refs().map_err(|e| e.to_string())
}

#[tauri::command]
#[instrument(fields(command = "get_recent_repositories"))]
pub fn get_recent_repositories() -> Result<Vec<RecentRepository>, String> {
    load_recent_repos()
}

#[tauri::command]
#[instrument(fields(command = "save_recent_repository"))]
pub fn save_recent_repository(path: String) -> Result<(), String> {
    let _guard = RECENT_REPOS_LOCK.lock().map_err(|e| e.to_string())?;

    let repo_path = PathBuf::from(&path);
    let canonical = repo_path.canonicalize().unwrap_or(repo_path);
    let name = canonical
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| canonical.to_string_lossy().to_string());
    let now = chrono::Utc::now();

    let mut repos = load_recent_repos()?;
    repos.retain(|r| r.path != canonical);
    repos.insert(
        0,
        RecentRepository {
            path: canonical,
            name,
            last_opened: now,
        },
    );
    repos.truncate(MAX_RECENT_REPOS);
    save_recent_repos(&repos)
}
