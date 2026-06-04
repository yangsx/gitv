use gitv_git_core::gix_repo::GixRepository;
use gitv_git_core::graph::{GraphCalculator, GraphColorMode, GraphOptions, GraphOrientation, GraphPalette};
use gitv_git_core::repository::Repository;
use std::collections::HashMap;
use std::path::PathBuf;
use tracing::instrument;

#[tauri::command]
#[instrument(skip(path), fields(command = "get_graph_layout"))]
pub fn get_graph_layout(
    path: String,
    hide_merges: Option<bool>,
    orientation: Option<String>,
    color_mode: Option<String>,
    palette: Option<String>,
) -> Result<gitv_git_core::graph::GraphLayout, String> {
    let repo_path = PathBuf::from(&path);
    let repo = GixRepository::open(&repo_path).map_err(|e| e.to_string())?;
    let commits = repo.commits(None).map_err(|e| e.to_string())?;

    let mut refs_map: HashMap<gitv_git_core::models::Oid, Vec<gitv_git_core::models::Ref>> =
        HashMap::new();
    for c in &commits {
        if !c.refs.is_empty() {
            refs_map.insert(c.oid, c.refs.clone());
        }
    }

    let orientation = match orientation.as_deref() {
        Some("bottom-to-top") => GraphOrientation::BottomToTop,
        _ => GraphOrientation::TopToBottom,
    };

    let color_mode = match color_mode.as_deref() {
        Some("by-author") => GraphColorMode::ByAuthor,
        _ => GraphColorMode::ByBranch,
    };

    let graph_palette = match palette.as_deref() {
        Some("deuteranopia") => GraphPalette::DeuteranopiaSafe,
        Some("protanopia") => GraphPalette::ProtanopiaSafe,
        Some("tritanopia") => GraphPalette::TritanopiaSafe,
        _ => GraphPalette::Default,
    };

    let options = GraphOptions {
        hide_merges: hide_merges.unwrap_or(false),
        orientation,
        color_mode,
        palette: graph_palette,
    };

    let calc = GraphCalculator::new(commits, refs_map, Vec::new(), options);
    Ok(calc.calculate_layout())
}
