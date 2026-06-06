use std::fs;
use std::path::PathBuf;

pub fn config_dir() -> Result<PathBuf, String> {
    let dir = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("gitv");
    fs::create_dir_all(&dir).map_err(|e| format!("failed to create config dir: {e}"))?;
    Ok(dir)
}
