use gitv_git_core::gix_repo::GixRepository;
use gitv_git_core::repository::Repository;
use gitv_git_core::search::{SearchEngine, SearchQuery};
use std::path::PathBuf;

#[tauri::command]
pub fn search_commits(
    path: String,
    query: SearchQuery,
) -> Result<Vec<gitv_git_core::search::SearchResult>, String> {
    let repo_path = PathBuf::from(&path);
    let repo = GixRepository::open(&repo_path).map_err(|e| e.to_string())?;
    let commits = repo.commits(None).map_err(|e| e.to_string())?;
    let engine = SearchEngine::new(commits);
    engine.search(&query).map_err(|e| e.to_string())
}
