use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GraphOptions {
    pub hide_merges: bool,
    pub orientation: GraphOrientation,
    pub color_mode: GraphColorMode,
}

impl Default for GraphOptions {
    fn default() -> Self {
        Self {
            hide_merges: false,
            orientation: GraphOrientation::TopToBottom,
            color_mode: GraphColorMode::ByBranch,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NodePosition {
    pub commit_oid: Oid,
    pub row: usize,
    pub column: usize,
    pub is_merge: bool,
    pub dimmed: bool,
    pub highlighted: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StashMarker {
    pub stash_index: usize,
    pub stash_oid: Oid,
    pub parent_commit_oid: Oid,
    pub row: usize,
    pub message: String,
    pub file_summary: Vec<StashFileSummary>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Edge {
    pub from_row: usize,
    pub from_col: usize,
    pub to_row: usize,
    pub to_col: usize,
    pub edge_type: EdgeType,
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
    pub max_column: usize,
    pub branch_colors: HashMap<String, Color>,
    pub author_colors: HashMap<String, Color>,
    pub orientation: GraphOrientation,
    pub total_rows: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GraphViewport {
    pub rows: Range<usize>,
    pub nodes: Vec<NodePosition>,
    pub stash_markers: Vec<StashMarker>,
    pub edges: Vec<Edge>,
    pub max_column: usize,
}
