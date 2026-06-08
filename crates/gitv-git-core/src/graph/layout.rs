use serde::{Deserialize, Serialize};
use std::ops::Range;

use crate::models::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum GraphOrientation {
    TopToBottom,
    BottomToTop,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum GraphColorMode {
    ByBranch,
    ByAuthor,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum GraphPalette {
    #[default]
    Default,
    DeuteranopiaSafe,
    ProtanopiaSafe,
    TritanopiaSafe,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum EdgeStyle {
    #[default]
    Solid,
    Dashed,
    Dotted,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GraphOptions {
    pub hide_merges: bool,
    pub orientation: GraphOrientation,
    pub color_mode: GraphColorMode,
    pub palette: GraphPalette,
}

impl Default for GraphOptions {
    fn default() -> Self {
        Self {
            hide_merges: false,
            orientation: GraphOrientation::TopToBottom,
            color_mode: GraphColorMode::ByBranch,
            palette: GraphPalette::Default,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NodePosition {
    pub oid: Oid,
    pub row: usize,
    pub column: usize,
    pub is_merge: bool,
    pub is_stash: bool,
    pub color: Color,
    pub is_dimmed: bool,
    pub is_highlighted: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StashMarker {
    pub row: usize,
    pub column: usize,
    pub stash_index: usize,
    pub stash_oid: Oid,
    pub parent_oid: Oid,
    pub message: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Edge {
    pub from_row: usize,
    pub from_col: usize,
    pub to_row: usize,
    pub to_col: usize,
    pub edge_type: EdgeType,
    pub color: Color,
    pub is_dimmed: bool,
    pub edge_style: EdgeStyle,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EdgeType {
    Straight,
    Branch,
    Merge,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GraphLayout {
    pub nodes: Vec<NodePosition>,
    pub stash_markers: Vec<StashMarker>,
    pub edges: Vec<Edge>,
    pub total_columns: usize,
    pub orientation: GraphOrientation,
    pub total_rows: usize,
    pub stash_commits: Vec<CommitInfo>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GraphViewport {
    pub rows: Range<usize>,
    pub nodes: Vec<NodePosition>,
    pub stash_markers: Vec<StashMarker>,
    pub edges: Vec<Edge>,
    pub total_columns: usize,
}
