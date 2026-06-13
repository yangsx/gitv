use gitv_git_core::gix_repo::GixRepository;
use gitv_git_core::repository::Repository;
use gitv_git_core::search::{SearchEngine, SearchQuery};
use std::collections::HashMap;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;

pub struct AppState {
    search_engines: Mutex<HashMap<PathBuf, SearchEngine>>,
    repo_cache: Mutex<HashMap<PathBuf, Arc<GixRepository>>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            search_engines: Mutex::new(HashMap::new()),
            repo_cache: Mutex::new(HashMap::new()),
        }
    }

    pub fn search(
        &self,
        repo_path: &Path,
        query: &SearchQuery,
    ) -> Result<Vec<gitv_git_core::search::SearchResult>, String> {
        let mut cache = self.search_engines.lock().map_err(|e| e.to_string())?;

        if !cache.contains_key(repo_path) {
            let repo = self.get_repo(repo_path)?;
            let commits = repo.commits(None, &[]).map_err(|e| e.to_string())?;
            let engine = SearchEngine::new(commits);
            cache.insert(repo_path.to_path_buf(), engine);
        }

        let engine = cache.get(repo_path).ok_or("engine not found")?;
        engine.search(query).map_err(|e| e.to_string())
    }

    pub fn get_repo(&self, path: &Path) -> Result<Arc<GixRepository>, String> {
        let mut cache = self.repo_cache.lock().map_err(|e| e.to_string())?;
        if let Some(repo) = cache.get(path) {
            return Ok(Arc::clone(repo));
        }
        let repo = GixRepository::open(path).map_err(|e| e.to_string())?;
        let repo = Arc::new(repo);
        cache.insert(path.to_path_buf(), Arc::clone(&repo));
        Ok(repo)
    }

    #[allow(dead_code)]
    pub fn invalidate_repo(&self, repo_path: &Path) {
        if let Ok(mut cache) = self.search_engines.lock() {
            cache.remove(repo_path);
        }
        if let Ok(mut cache) = self.repo_cache.lock() {
            cache.remove(repo_path);
        }
    }
}
