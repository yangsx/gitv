use chrono::Utc;
use gitv_git_core::models::SavedSearch;
use std::fs;
use std::path::{Path, PathBuf};

fn saved_searches_path(repo_path: &str) -> Result<PathBuf, String> {
    let git_dir = Path::new(repo_path).join(".git");
    if !git_dir.exists() {
        return Err("not a git repository".to_string());
    }
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

fn save_searches(path: &Path, searches: &[SavedSearch]) -> Result<(), String> {
    let data = serde_json::to_string_pretty(searches).map_err(|e| e.to_string())?;
    fs::write(path, data).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_search(repo_path: String, name: String, query: String) -> Result<SavedSearch, String> {
    let path = saved_searches_path(&repo_path)?;
    let mut searches = load_searches(&path)?;
    let id = format!("{:x}", chrono::Utc::now().timestamp_millis());
    let search = SavedSearch {
        id: id.clone(),
        name,
        query,
        created_at: Utc::now(),
    };
    searches.push(search.clone());
    save_searches(&path, &searches)?;
    Ok(search)
}

#[tauri::command]
pub fn list_saved_searches(repo_path: String) -> Result<Vec<SavedSearch>, String> {
    let path = saved_searches_path(&repo_path)?;
    load_searches(&path)
}

#[tauri::command]
pub fn delete_saved_search(repo_path: String, id: String) -> Result<(), String> {
    let path = saved_searches_path(&repo_path)?;
    let mut searches = load_searches(&path)?;
    let before = searches.len();
    searches.retain(|s| s.id != id);
    if searches.len() == before {
        return Err(format!("saved search {id} not found"));
    }
    save_searches(&path, &searches)
}
