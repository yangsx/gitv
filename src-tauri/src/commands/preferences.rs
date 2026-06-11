use crate::commands::util;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
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

#[derive(Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum DiffViewMode {
    #[default]
    Unified,
    SideBySide,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Theme {
    Dark,
    Light,
}

#[derive(Clone, Debug, Default)]
pub enum Language {
    #[default]
    En,
    ZhCn,
    Custom(String),
}

impl Language {
    pub fn code(&self) -> &str {
        match self {
            Language::En => "en",
            Language::ZhCn => "zh-cn",
            Language::Custom(s) => s,
        }
    }
}

impl Serialize for Language {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(self.code())
    }
}

impl<'de> Deserialize<'de> for Language {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        Ok(match s.as_str() {
            "en" => Language::En,
            "zh-cn" => Language::ZhCn,
            other => Language::Custom(other.to_string()),
        })
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct AppPreferences {
    pub graph_color_mode: GraphColorMode,
    pub graph_hide_merges: bool,
    pub graph_orientation: GraphOrientation,
    pub graph_palette: ColorPalette,
    #[serde(default = "default_renderer")]
    pub renderer: String,
    pub diff_mode: DiffMode,
    pub diff_whitespace: DiffWhitespace,
    #[serde(default)]
    pub diff_view_mode: DiffViewMode,
    pub theme: Theme,
    #[serde(default = "default_font_size")]
    pub font_size: f64,
    #[serde(default)]
    pub high_contrast: bool,
    #[serde(default)]
    pub language: Language,
}

fn default_renderer() -> String {
    "wgpu".to_string()
}

fn default_font_size() -> f64 {
    13.0
}

impl Default for AppPreferences {
    fn default() -> Self {
        Self {
            graph_color_mode: GraphColorMode::ByBranch,
            graph_hide_merges: false,
            graph_orientation: GraphOrientation::TopToBottom,
            graph_palette: ColorPalette::Default,
            renderer: default_renderer(),
            diff_mode: DiffMode::Normal,
            diff_whitespace: DiffWhitespace::None,
            diff_view_mode: DiffViewMode::Unified,
            theme: Theme::Dark,
            font_size: default_font_size(),
            high_contrast: false,
            language: Language::En,
        }
    }
}

fn preferences_path() -> Result<PathBuf, String> {
    Ok(util::config_dir()?.join(PREFERENCES_FILENAME))
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
