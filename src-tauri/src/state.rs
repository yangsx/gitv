use gitv_git_core::repository::Repository;
use gitv_git_core::search::{SearchEngine, SearchQuery};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;

pub struct AppState {
    search_engines: Mutex<HashMap<PathBuf, SearchEngine>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            search_engines: Mutex::new(HashMap::new()),
        }
    }

    pub fn search(
        &self,
        repo_path: &PathBuf,
        query: &SearchQuery,
    ) -> Result<Vec<gitv_git_core::search::SearchResult>, String> {
        let mut cache = self.search_engines.lock().map_err(|e| e.to_string())?;

        if !cache.contains_key(repo_path) {
            let repo = gitv_git_core::gix_repo::GixRepository::open(repo_path)
                .map_err(|e| e.to_string())?;
            let commits = repo.commits(None).map_err(|e| e.to_string())?;
            let engine = SearchEngine::new(commits);
            cache.insert(repo_path.clone(), engine);
        }

        let engine = cache.get(repo_path).ok_or("engine not found")?;
        engine.search(query).map_err(|e| e.to_string())
    }

    #[allow(dead_code)]
    pub fn invalidate_repo(&self, repo_path: &PathBuf) {
        if let Ok(mut cache) = self.search_engines.lock() {
            cache.remove(repo_path);
        }
    }
}
