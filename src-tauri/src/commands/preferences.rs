use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tracing::instrument;

const PREFERENCES_FILENAME: &str = "preferences.json";

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum GraphColorMode {
    ByBranch,
    ByAuthor,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum GraphOrientation {
    TopToBottom,
    BottomToTop,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ColorPalette {
    Default,
    Deuteranopia,
    Protanopia,
    Tritanopia,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DiffMode {
    Normal,
    WordDiff,
    StatOnly,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DiffWhitespace {
    None,
    IgnoreSpaceChange,
    IgnoreAllSpace,
    IgnoreBlankLines,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Theme {
    Dark,
    Light,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct AppPreferences {
    pub graph_color_mode: GraphColorMode,
    pub graph_hide_merges: bool,
    pub graph_orientation: GraphOrientation,
    pub graph_palette: ColorPalette,
    pub diff_mode: DiffMode,
    pub diff_whitespace: DiffWhitespace,
    pub theme: Theme,
}

impl Default for AppPreferences {
    fn default() -> Self {
        Self {
            graph_color_mode: GraphColorMode::ByBranch,
            graph_hide_merges: false,
            graph_orientation: GraphOrientation::TopToBottom,
            graph_palette: ColorPalette::Default,
            diff_mode: DiffMode::Normal,
            diff_whitespace: DiffWhitespace::None,
            theme: Theme::Dark,
        }
    }
}

fn preferences_dir() -> Result<PathBuf, String> {
    let dir = dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("gitv");
    fs::create_dir_all(&dir).map_err(|e| format!("failed to create config dir: {e}"))?;
    Ok(dir)
}

fn preferences_path() -> Result<PathBuf, String> {
    Ok(preferences_dir()?.join(PREFERENCES_FILENAME))
}

fn load_preferences() -> Result<AppPreferences, String> {
    let path = preferences_path()?;
    if !path.exists() {
        return Ok(AppPreferences::default());
    }
    let data = fs::read_to_string(&path).map_err(|e| format!("failed to read preferences: {e}"))?;
    serde_json::from_str(&data).map_err(|e| format!("failed to parse preferences: {e}"))
}

fn save_preferences_atomic(prefs: &AppPreferences) -> Result<(), String> {
    let path = preferences_path()?;
    let data =
        serde_json::to_string_pretty(prefs).map_err(|e| format!("failed to serialize: {e}"))?;
    let tmp_path = path.with_extension("json.tmp");
    fs::write(&tmp_path, &data).map_err(|e| format!("failed to write: {e}"))?;
    fs::rename(&tmp_path, &path).map_err(|e| {
        let _ = fs::remove_file(&tmp_path);
        format!("failed to rename: {e}")
    })
}

#[tauri::command]
#[instrument(fields(command = "get_preferences"))]
pub fn get_preferences() -> Result<AppPreferences, String> {
    load_preferences()
}

#[tauri::command]
#[instrument(skip(prefs), fields(command = "set_preferences"))]
pub fn set_preferences(prefs: AppPreferences) -> Result<(), String> {
    save_preferences_atomic(&prefs)
}
