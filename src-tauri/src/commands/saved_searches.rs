use chrono::Utc;
use gitv_git_core::models::SavedSearch;
use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use tracing::instrument;

fn resolve_git_dir(repo_path: &str) -> Result<PathBuf, String> {
    let p = Path::new(repo_path);
    let git_subdir = p.join(".git");
    if git_subdir.is_dir() {
        return Ok(git_subdir);
    }
    if p.join("HEAD").exists() && p.join("objects").exists() {
        return Ok(p.to_path_buf());
    }
    Err("not a git repository".to_string())
}

fn saved_searches_path(repo_path: &str) -> Result<PathBuf, String> {
    let git_dir = resolve_git_dir(repo_path)?;
    let gitv_dir = git_dir.join("gitv");
    fs::create_dir_all(&gitv_dir).map_err(|e| e.to_string())?;
    Ok(gitv_dir.join("saved_searches.json"))
}

fn load_searches(path: &Path) -> Result<Vec<SavedSearch>, String> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let data = fs::read_to_string(path).map_err(|e| e.to_string())?;
    serde_json::from_str(&data).map_err(|e| e.to_string())
}

fn save_searches_atomic(path: &Path, searches: &[SavedSearch]) -> Result<(), String> {
    let data = serde_json::to_string_pretty(searches).map_err(|e| e.to_string())?;
    let tmp_path = path.with_extension("json.tmp");
    fs::write(&tmp_path, &data).map_err(|e| e.to_string())?;
    fs::rename(&tmp_path, path).map_err(|e| {
        let _ = fs::remove_file(&tmp_path);
        e.to_string()
    })
}

fn generate_id(name: &str) -> String {
    let ts = chrono::Utc::now().timestamp_millis();
    let mut hasher = DefaultHasher::new();
    name.hash(&mut hasher);
    ts.hash(&mut hasher);
    format!("{:x}{:08x}", ts, hasher.finish())
}

#[tauri::command]
#[instrument(skip(repo_path, name, query), fields(command = "save_search"))]
pub fn save_search(repo_path: String, name: String, query: String) -> Result<SavedSearch, String> {
    let path = saved_searches_path(&repo_path)?;
    let mut searches = load_searches(&path)?;
    let id = generate_id(&name);
    let search = SavedSearch {
        id: id.clone(),
        name,
        query,
        created_at: Utc::now(),
    };
    searches.push(search.clone());
    save_searches_atomic(&path, &searches)?;
    Ok(search)
}

#[tauri::command]
#[instrument(skip(repo_path), fields(command = "list_saved_searches"))]
pub fn list_saved_searches(repo_path: String) -> Result<Vec<SavedSearch>, String> {
    let path = saved_searches_path(&repo_path)?;
    load_searches(&path)
}

#[tauri::command]
#[instrument(skip(repo_path, id), fields(command = "delete_saved_search"))]
pub fn delete_saved_search(repo_path: String, id: String) -> Result<(), String> {
    let path = saved_searches_path(&repo_path)?;
    let mut searches = load_searches(&path)?;
    let before = searches.len();
    searches.retain(|s| s.id != id);
    if searches.len() == before {
        return Err(format!("saved search {id} not found"));
    }
    save_searches_atomic(&path, &searches)
}
