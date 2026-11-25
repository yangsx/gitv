use crate::commands::util;
use crate::state::{AppState, resolve_repo_key};
use gitv_git_core::error::GitError;
use gitv_git_core::gix_repo::{GixRepository, is_remote_url, remote_cache_root};
use gitv_git_core::models::RecentRepository;
use gitv_git_core::models::RepositoryInfo;
use gitv_git_core::repository;
use gitv_git_core::repository::Repository;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Mutex;
use tauri::{AppHandle, Manager, State};
use time::OffsetDateTime;
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
#[instrument(skip(path, auth_token), fields(command = "open_repository"))]
pub fn open_repository(path: String, auth_token: Option<String>) -> Result<RepositoryInfo, String> {
    if is_remote_url(&path) {
        let cache_root = remote_cache_root();
        let repo = GixRepository::open_remote_with_token(&path, &cache_root, auth_token.as_deref())
            .map_err(|e| match e {
                GitError::AuthRequired(_) => "remote_auth_required".to_string(),
                _ => format!("remote_failed: {e}"),
            })?;
        return repo.info().map_err(|e| format!("open_failed: {e}"));
    }
    let repo_path = PathBuf::from(&path);
    let repo = repository::open(&repo_path).map_err(|e| match e {
        GitError::NotAGitRepository(_) => "not_a_git_repository".to_string(),
        _ => "open_failed".to_string(),
    })?;
    repo.info().map_err(|_| "open_failed".to_string())
}

#[tauri::command]
#[instrument(skip(state, path), fields(command = "get_refs"))]
pub fn get_refs(
    state: State<'_, AppState>,
    path: String,
) -> Result<Vec<gitv_git_core::models::Ref>, String> {
    let repo_path = resolve_repo_key(&path);
    let repo = state.get_repo(&repo_path)?;
    repo.refs().map_err(|e| e.to_string())
}

#[tauri::command]
#[instrument(fields(command = "get_recent_repositories"))]
pub async fn get_recent_repositories() -> Result<Vec<RecentRepository>, String> {
    tauri::async_runtime::spawn_blocking(load_recent_repos)
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
#[instrument(skip(path), fields(command = "open_in_new_window"))]
pub fn open_in_new_window(path: String) -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| format!("failed to get executable path: {e}"))?;
    Command::new(exe)
        .arg(&path)
        .spawn()
        .map_err(|e| format!("failed to spawn new instance: {e}"))?;
    Ok(())
}

#[tauri::command]
#[instrument(skip(app), fields(command = "set_window_title"))]
pub fn set_window_title(app: AppHandle, title: String) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.set_title(&title);
    }
}

#[tauri::command]
#[instrument(skip(app), fields(command = "quit_app"))]
pub fn quit_app(app: AppHandle) {
    app.exit(0);
}

#[tauri::command]
#[instrument(fields(command = "save_recent_repository"))]
pub async fn save_recent_repository(path: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = RECENT_REPOS_LOCK.lock().map_err(|e| e.to_string())?;

        let (canonical, name) = if is_remote_url(&path) {
            let display_name = path
                .trim_end_matches('/')
                .strip_prefix("https://")
                .or_else(|| path.strip_prefix("http://"))
                .unwrap_or(&path);
            let display_name = display_name.strip_suffix(".git").unwrap_or(display_name);
            (PathBuf::from(&path), display_name.to_string())
        } else {
            let repo_path = PathBuf::from(shellexpand::tilde(&path).as_ref());
            let canonical = repo_path.canonicalize().unwrap_or(repo_path);
            let name = canonical
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| canonical.to_string_lossy().to_string());
            (canonical, name)
        };
        let now = OffsetDateTime::now_utc();

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
    })
    .await
    .map_err(|e| e.to_string())?
}

#[derive(Serialize)]
pub struct RepoPathValidation {
    kind: String,
    valid: bool,
    is_git: bool,
    message: Option<String>,
}

#[tauri::command]
#[instrument(skip(path), fields(command = "validate_repo_path"))]
pub fn validate_repo_path(path: String) -> RepoPathValidation {
    if is_remote_url(&path) {
        return RepoPathValidation {
            kind: "remote".to_string(),
            valid: true,
            is_git: false,
            message: None,
        };
    }
    let expanded = shellexpand::tilde(&path).to_string();
    let p = std::path::PathBuf::from(&expanded);
    let exists = p.exists();
    if !exists {
        return RepoPathValidation {
            kind: "local".to_string(),
            valid: false,
            is_git: false,
            message: Some("directory does not exist".to_string()),
        };
    }
    let is_git = p.join(".git").is_dir() || (p.is_dir() && p.join("HEAD").exists());
    if !is_git {
        return RepoPathValidation {
            kind: "local".to_string(),
            valid: true,
            is_git: false,
            message: Some("not a git repository".to_string()),
        };
    }
    RepoPathValidation {
        kind: "local".to_string(),
        valid: true,
        is_git: true,
        message: None,
    }
}

#[derive(Serialize)]
pub struct PathSuggestion {
    path: String,
    is_dir: bool,
}

#[tauri::command]
#[instrument(skip(prefix), fields(command = "suggest_paths"))]
pub fn suggest_paths(prefix: String) -> Vec<PathSuggestion> {
    let expanded = shellexpand::tilde(&prefix).to_string();
    let p = std::path::PathBuf::from(&expanded);
    let parent = if p.as_os_str().is_empty() {
        std::path::PathBuf::from("/")
    } else if p.is_dir() {
        p.clone()
    } else {
        p.parent()
            .map(|d| d.to_path_buf())
            .unwrap_or_else(|| std::path::PathBuf::from("/"))
    };
    let search_prefix = p
        .file_name()
        .map(|n| n.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    let mut results: Vec<PathSuggestion> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&parent) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if name_str.starts_with('.') {
                continue;
            }
            if !search_prefix.is_empty() && !name_str.to_lowercase().starts_with(&search_prefix) {
                continue;
            }
            let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
            if !is_dir {
                continue;
            }
            let full = parent.join(&name);
            results.push(PathSuggestion {
                path: full.to_string_lossy().to_string(),
                is_dir: true,
            });
        }
    }
    results.sort_by_key(|a| a.path.to_lowercase());
    results.truncate(20);
    results
}
