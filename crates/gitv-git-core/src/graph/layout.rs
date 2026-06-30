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
    pub arrow_gap_threshold: usize,
}

impl Default for GraphOptions {
    fn default() -> Self {
        Self {
            hide_merges: false,
            orientation: GraphOrientation::TopToBottom,
            color_mode: GraphColorMode::ByBranch,
            palette: GraphPalette::Default,
            arrow_gap_threshold: 100,
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
    /// Intermediate (row, col) waypoints where the thread changes direction.
    /// Empty for straight edges. Drawn as connected line segments.
    #[serde(default)]
    pub waypoints: Vec<(usize, usize)>,
    /// When the thread was removed from the rowidlist (gitk thread lifecycle),
    /// contains `(seg1_end_row, seg2_start_row)` — the boundary rows of the
    /// gap. Segment 1 is near `from_row` (child), segment 2 near `to_row` (parent).
    /// The renderer draws two short segments with arrowheads at these boundaries.
    /// Waypoints with rows ≤ seg1_end_row belong to segment 1;
    /// waypoints with rows ≥ seg2_start_row belong to segment 2.
    #[serde(default)]
    pub arrow_gap: Option<(usize, usize)>,
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
    /// Per-row maximum occupied column index + 1 (i.e. lane count).
    /// Indexed by row. Used by the frontend to compute per-row text start
    /// offset for the gitk-style flowing-text layout.
    #[serde(default)]
    pub row_max_column: Vec<usize>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GraphViewport {
    pub rows: Range<usize>,
    pub nodes: Vec<NodePosition>,
    pub stash_markers: Vec<StashMarker>,
    pub edges: Vec<Edge>,
    pub total_columns: usize,
    #[serde(default)]
    pub row_max_column: Vec<usize>,
}

impl GraphLayout {
    /// Verify layout correctness: check that no edge passes through an
    /// unrelated node. For multi-segment edges (with waypoints), traces
    /// the actual path through waypoints.
    ///
    /// Returns a list of error messages. An empty vec means the layout
    /// is valid. Errors are capped at 100_000 entries.
    #[must_use]
    pub fn verify(&self) -> Vec<String> {
        let mut errors: Vec<String> = Vec::new();
        const MAX_ERRORS: usize = 100_000;

        for edge in &self.edges {
            if errors.len() >= MAX_ERRORS {
                break;
            }

            // Build path segments, splitting at arrow_gap if present.
            // Each segment is checked independently — nodes inside the gap
            // region are NOT pass-through violations.
            let mut segments: Vec<Vec<(usize, usize)>> = Vec::new();

            if let Some((gap_lo, gap_hi)) = edge.arrow_gap {
                // Segment 1: from_row → waypoints with row ≤ gap_lo
                let mut seg1 = vec![(edge.from_row, edge.from_col)];
                for wp in &edge.waypoints {
                    if wp.0 <= gap_lo {
                        seg1.push(*wp);
                    }
                }
                segments.push(seg1);

                // Segment 2: waypoints with row ≥ gap_hi → to_row
                let mut seg2: Vec<(usize, usize)> = Vec::new();
                for wp in &edge.waypoints {
                    if wp.0 >= gap_hi {
                        seg2.push(*wp);
                    }
                }
                seg2.push((edge.to_row, edge.to_col));
                segments.push(seg2);
            } else {
                // Single continuous path
                let mut path = vec![(edge.from_row, edge.from_col)];
                path.extend(edge.waypoints.iter().copied());
                path.push((edge.to_row, edge.to_col));
                segments.push(path);
            }

            // Check each segment for pass-through
            for path in &segments {
                for window in path.windows(2) {
                    let (r1, c1) = window[0];
                    let (r2, c2) = window[1];
                    if c1 != c2 {
                        continue; // Cross-column segment, skip
                    }
                    let (min_row, max_row) = (r1.min(r2), r1.max(r2));
                    for node in &self.nodes {
                        if errors.len() >= MAX_ERRORS {
                            break;
                        }
                        if node.column != c1 {
                            continue;
                        }
                        if node.row == edge.from_row || node.row == edge.to_row {
                            continue;
                        }
                        if node.row > min_row && node.row < max_row {
                            errors.push(format!(
                                "edge ({},{})\u{2192}({},{}) segment ({},{})\u{2192}({},{}) passes through node {} at ({},{})",
                                edge.from_row,
                                edge.from_col,
                                edge.to_row,
                                edge.to_col,
                                r1,
                                c1,
                                r2,
                                c2,
                                node.oid.short_hex(),
                                node.row,
                                node.column,
                            ));
                        }
                    }
                }
            }
        }

        errors
    }
}
