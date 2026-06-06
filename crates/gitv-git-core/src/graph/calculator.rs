use std::collections::{HashMap, HashSet};

use crate::graph::layout::*;
use crate::models::*;

const BRANCH_PALETTE: &[Color] = &[
    Color {
        r: 79,
        g: 148,
        b: 205,
        a: 255,
    },
    Color {
        r: 230,
        g: 126,
        b: 34,
        a: 255,
    },
    Color {
        r: 46,
        g: 204,
        b: 113,
        a: 255,
    },
    Color {
        r: 231,
        g: 76,
        b: 60,
        a: 255,
    },
    Color {
        r: 155,
        g: 89,
        b: 182,
        a: 255,
    },
    Color {
        r: 241,
        g: 196,
        b: 15,
        a: 255,
    },
    Color {
        r: 26,
        g: 188,
        b: 156,
        a: 255,
    },
    Color {
        r: 192,
        g: 57,
        b: 43,
        a: 255,
    },
    Color {
        r: 52,
        g: 73,
        b: 94,
        a: 255,
    },
    Color {
        r: 142,
        g: 68,
        b: 173,
        a: 255,
    },
];

const DEUTERANOPIA_PALETTE: &[Color] = &[
    Color {
        r: 0,
        g: 114,
        b: 178,
        a: 255,
    },
    Color {
        r: 230,
        g: 159,
        b: 0,
        a: 255,
    },
    Color {
        r: 0,
        g: 158,
        b: 115,
        a: 255,
    },
    Color {
        r: 204,
        g: 121,
        b: 167,
        a: 255,
    },
    Color {
        r: 86,
        g: 180,
        b: 233,
        a: 255,
    },
    Color {
        r: 213,
        g: 94,
        b: 0,
        a: 255,
    },
    Color {
        r: 240,
        g: 228,
        b: 66,
        a: 255,
    },
    Color {
        r: 0,
        g: 0,
        b: 0,
        a: 255,
    },
];

const PROTANOPIA_PALETTE: &[Color] = &[
    Color {
        r: 0,
        g: 114,
        b: 178,
        a: 255,
    },
    Color {
        r: 230,
        g: 159,
        b: 0,
        a: 255,
    },
    Color {
        r: 0,
        g: 158,
        b: 115,
        a: 255,
    },
    Color {
        r: 204,
        g: 121,
        b: 167,
        a: 255,
    },
    Color {
        r: 86,
        g: 180,
        b: 233,
        a: 255,
    },
    Color {
        r: 213,
        g: 94,
        b: 0,
        a: 255,
    },
    Color {
        r: 240,
        g: 228,
        b: 66,
        a: 255,
    },
    Color {
        r: 0,
        g: 0,
        b: 0,
        a: 255,
    },
];

const TRITANOPIA_PALETTE: &[Color] = &[
    Color {
        r: 230,
        g: 159,
        b: 0,
        a: 255,
    },
    Color {
        r: 86,
        g: 180,
        b: 233,
        a: 255,
    },
    Color {
        r: 0,
        g: 158,
        b: 115,
        a: 255,
    },
    Color {
        r: 204,
        g: 121,
        b: 167,
        a: 255,
    },
    Color {
        r: 213,
        g: 94,
        b: 0,
        a: 255,
    },
    Color {
        r: 240,
        g: 228,
        b: 66,
        a: 255,
    },
    Color {
        r: 0,
        g: 114,
        b: 178,
        a: 255,
    },
    Color {
        r: 0,
        g: 0,
        b: 0,
        a: 255,
    },
];

fn palette_for(mode: GraphPalette) -> &'static [Color] {
    match mode {
        GraphPalette::Default => BRANCH_PALETTE,
        GraphPalette::DeuteranopiaSafe => DEUTERANOPIA_PALETTE,
        GraphPalette::ProtanopiaSafe => PROTANOPIA_PALETTE,
        GraphPalette::TritanopiaSafe => TRITANOPIA_PALETTE,
    }
}

pub struct GraphCalculator {
    commits: Vec<CommitInfo>,
    refs: HashMap<Oid, Vec<Ref>>,
    stashes: Vec<StashEntry>,
    options: GraphOptions,
}

struct CommitGraphData {
    row: usize,
    column: usize,
    is_merge: bool,
    color: Color,
}

impl GraphCalculator {
    pub fn new(
        commits: Vec<CommitInfo>,
        refs: HashMap<Oid, Vec<Ref>>,
        stashes: Vec<StashEntry>,
        options: GraphOptions,
    ) -> Self {
        Self {
            commits,
            refs,
            stashes,
            options,
        }
    }

    pub fn calculate_layout(&self) -> GraphLayout {
        let commits = if self.options.hide_merges {
            self.filter_merges()
        } else {
            self.commits.clone()
        };

        if commits.is_empty() {
            return self.empty_layout();
        }

        let children = Self::build_children_map(&commits);
        let row_assignments = Self::temporal_topological_sort(&commits, &children);

        let mut graph_data: HashMap<Oid, CommitGraphData> = HashMap::new();
        let palette = palette_for(self.options.palette);
        for c in &commits {
            let row = *row_assignments.get(&c.oid).unwrap_or(&0);
            graph_data.insert(
                c.oid,
                CommitGraphData {
                    row,
                    column: 0,
                    is_merge: c.parent_oids.len() > 1,
                    color: palette[0],
                },
            );
        }

        let mut sorted: Vec<&CommitInfo> = commits.iter().collect();
        sorted.sort_by_key(|c| graph_data[&c.oid].row);

        let oid_index: HashMap<Oid, usize> = commits
            .iter()
            .enumerate()
            .map(|(i, c)| (c.oid, i))
            .collect();

        let mut lanes: Vec<Option<Oid>> = Vec::new();
        let mut last_occupied_row: Vec<usize> = Vec::new();

        for c in &sorted {
            let c_row = graph_data[&c.oid].row;

            let branch_children: Vec<(usize, usize)> = children
                .get(&c.oid)
                .map(|chs| {
                    chs.iter()
                        .filter_map(|ch| {
                            oid_index.get(ch).and_then(|&chi| {
                                if commits[chi].parent_oids.first() == Some(&c.oid) {
                                    Some((chi, graph_data[ch].column))
                                } else {
                                    None
                                }
                            })
                        })
                        .collect()
                })
                .unwrap_or_default();

            let merge_children: Vec<(usize, usize)> = children
                .get(&c.oid)
                .map(|chs| {
                    chs.iter()
                        .filter_map(|ch| {
                            oid_index.get(ch).and_then(|&chi| {
                                if commits[chi].parent_oids.first() != Some(&c.oid) {
                                    Some((chi, graph_data[ch].column))
                                } else {
                                    None
                                }
                            })
                        })
                        .collect()
                })
                .unwrap_or_default();

            let i_min = merge_children
                .iter()
                .map(|&(mi, _)| graph_data[&commits[mi].oid].row)
                .min()
                .unwrap_or(usize::MAX);

            let forbidden: HashSet<usize> = last_occupied_row
                .iter()
                .enumerate()
                .filter(|(_, lor)| **lor >= i_min)
                .map(|(j, _)| j)
                .collect();

            let mut inherited_lane: Option<usize> = None;
            for &(_, child_col) in &branch_children {
                if !forbidden.contains(&child_col) {
                    inherited_lane = Some(child_col);
                    break;
                }
            }

            let assigned_lane = match inherited_lane {
                Some(lane) => {
                    lanes[lane] = Some(c.oid);
                    last_occupied_row[lane] = c_row;
                    lane
                }
                None => {
                    let nil_slot = lanes.iter().position(|l| l.is_none());
                    let lane = nil_slot.unwrap_or(lanes.len());
                    if lane == lanes.len() {
                        lanes.push(Some(c.oid));
                        last_occupied_row.push(c_row);
                    } else {
                        lanes[lane] = Some(c.oid);
                        last_occupied_row[lane] = c_row;
                    }
                    lane
                }
            };

            graph_data.get_mut(&c.oid).unwrap().column = assigned_lane;

            for &(_, child_col) in &branch_children {
                if child_col != assigned_lane && child_col < lanes.len() {
                    lanes[child_col] = None;
                }
            }

            for &(_, child_col) in &merge_children {
                if child_col < lanes.len() {
                    lanes[child_col] = None;
                }
            }
        }

        let total_columns = if lanes.is_empty() { 0 } else { lanes.len() };
        let total_rows = sorted.len();

        let mut stash_markers = self.place_stash_markers(&graph_data);

        self.assign_colors_to_nodes(&commits, &mut graph_data);
        let mut nodes = self.rebuild_nodes_with_colors(&sorted, &graph_data);
        let mut edges = self.rebuild_edges_with_colors(&sorted, &graph_data);

        if self.options.orientation == GraphOrientation::BottomToTop {
            let max_row = total_rows.saturating_sub(1);
            for node in &mut nodes {
                node.row = max_row - node.row;
            }
            for edge in &mut edges {
                let new_from = max_row - edge.from_row;
                let new_to = max_row - edge.to_row;
                edge.from_row = new_from;
                edge.to_row = new_to;
            }
            for marker in &mut stash_markers {
                marker.row = max_row - marker.row;
            }
        }

        GraphLayout {
            nodes,
            stash_markers,
            edges,
            total_columns,
            orientation: self.options.orientation,
            total_rows,
        }
    }

    pub fn apply_dimming(
        layout: &mut GraphLayout,
        selected_oid: Option<Oid>,
        matching_oids: Option<&HashSet<Oid>>,
    ) {
        match matching_oids {
            Some(matches) => {
                for node in &mut layout.nodes {
                    node.is_dimmed = !matches.contains(&node.oid);
                    node.is_highlighted = matches.contains(&node.oid);
                }
                for edge in &mut layout.edges {
                    edge.is_dimmed = true;
                }
            }
            None => {
                if let Some(oid) = selected_oid {
                    for node in &mut layout.nodes {
                        node.is_dimmed = node.oid != oid;
                        node.is_highlighted = node.oid == oid;
                    }
                    for edge in &mut layout.edges {
                        edge.is_dimmed = true;
                    }
                } else {
                    for node in &mut layout.nodes {
                        node.is_dimmed = false;
                        node.is_highlighted = false;
                    }
                    for edge in &mut layout.edges {
                        edge.is_dimmed = false;
                    }
                }
            }
        }
    }

    pub fn get_ancestor_oids(&self, oid: &Oid) -> HashSet<Oid> {
        let mut ancestors = HashSet::new();
        let mut stack = vec![*oid];
        while let Some(current) = stack.pop() {
            if !ancestors.insert(current) {
                continue;
            }
            if let Some(commit) = self.commits.iter().find(|c| c.oid == current) {
                for parent in &commit.parent_oids {
                    stack.push(*parent);
                }
            }
        }
        ancestors
    }

    fn filter_merges(&self) -> Vec<CommitInfo> {
        let merge_oids: HashSet<Oid> = self
            .commits
            .iter()
            .filter(|c| c.parent_oids.len() > 1)
            .map(|c| c.oid)
            .collect();

        let mut resolve: HashMap<Oid, Oid> = HashMap::new();
        for commit in &self.commits {
            if commit.parent_oids.len() <= 1 {
                continue;
            }
            let first_parent = match commit.parent_oids.first() {
                Some(p) => *p,
                None => continue,
            };
            let resolved = Self::resolve_to_non_merge(&first_parent, &merge_oids, &resolve);
            resolve.insert(commit.oid, resolved);
        }

        let mut result: Vec<CommitInfo> = self
            .commits
            .iter()
            .filter(|c| c.parent_oids.len() <= 1)
            .cloned()
            .collect();

        for c in &mut result {
            for p in &mut c.parent_oids {
                if let Some(&replacement) = resolve.get(p) {
                    *p = replacement;
                }
            }
        }

        result
    }

    fn resolve_to_non_merge(
        oid: &Oid,
        merge_oids: &HashSet<Oid>,
        resolve: &HashMap<Oid, Oid>,
    ) -> Oid {
        let mut current = *oid;
        let mut visited = HashSet::new();
        while merge_oids.contains(&current) {
            if !visited.insert(current) {
                break;
            }
            if let Some(&resolved) = resolve.get(&current) {
                current = resolved;
            } else {
                break;
            }
        }
        current
    }

    fn build_children_map(commits: &[CommitInfo]) -> HashMap<Oid, Vec<Oid>> {
        let mut children: HashMap<Oid, Vec<Oid>> = HashMap::new();
        for c in commits {
            for &p in &c.parent_oids {
                children.entry(p).or_default().push(c.oid);
            }
        }
        children
    }

    fn temporal_topological_sort(
        commits: &[CommitInfo],
        children: &HashMap<Oid, Vec<Oid>>,
    ) -> HashMap<Oid, usize> {
        let mut sorted_indices: Vec<usize> = (0..commits.len()).collect();
        sorted_indices.sort_by(|&a, &b| {
            commits[b]
                .commit_time
                .cmp(&commits[a].commit_time)
                .then_with(|| commits[b].oid.to_hex().cmp(&commits[a].oid.to_hex()))
        });

        let oid_to_idx: HashMap<Oid, usize> = commits
            .iter()
            .enumerate()
            .map(|(i, c)| (c.oid, i))
            .collect();

        let mut visited = vec![false; commits.len()];
        let mut rows = vec![0usize; commits.len()];
        let mut row_counter = 0usize;

        for &start in &sorted_indices {
            if visited[start] {
                continue;
            }
            let mut stack: Vec<(usize, bool)> = vec![(start, false)];
            while let Some((idx, is_post)) = stack.pop() {
                if is_post {
                    rows[idx] = row_counter;
                    row_counter += 1;
                    continue;
                }
                if visited[idx] {
                    continue;
                }
                visited[idx] = true;
                stack.push((idx, true));
                if let Some(child_oids) = children.get(&commits[idx].oid) {
                    let mut child_indices: Vec<usize> = child_oids
                        .iter()
                        .filter_map(|ch| oid_to_idx.get(ch).copied())
                        .filter(|&chi| !visited[chi])
                        .collect();
                    child_indices.sort_by(|&a, &b| {
                        commits[b]
                            .commit_time
                            .cmp(&commits[a].commit_time)
                            .then_with(|| commits[b].oid.to_hex().cmp(&commits[a].oid.to_hex()))
                    });
                    for &chi in child_indices.iter().rev() {
                        stack.push((chi, false));
                    }
                }
            }
        }

        commits
            .iter()
            .enumerate()
            .map(|(i, c)| (c.oid, rows[i]))
            .collect()
    }

    fn place_stash_markers(&self, graph_data: &HashMap<Oid, CommitGraphData>) -> Vec<StashMarker> {
        self.stashes
            .iter()
            .enumerate()
            .filter_map(|(idx, stash)| {
                graph_data.get(&stash.parent_oid).map(|gd| StashMarker {
                    row: gd.row,
                    column: gd.column,
                    stash_index: idx,
                    stash_oid: stash.oid,
                    parent_oid: stash.parent_oid,
                    message: stash.message.clone(),
                })
            })
            .collect()
    }

    fn assign_colors_to_nodes(
        &self,
        commits: &[CommitInfo],
        graph_data: &mut HashMap<Oid, CommitGraphData>,
    ) {
        let palette = palette_for(self.options.palette);
        let mut color_idx = 0usize;

        if self.options.color_mode == GraphColorMode::ByBranch {
            let mut lane_colors: HashMap<usize, Color> = HashMap::new();
            let mut ref_colors: HashMap<String, Color> = HashMap::new();
            for c in commits {
                let col = graph_data.get(&c.oid).map(|gd| gd.column).unwrap_or(0);
                let color = *lane_colors.entry(col).or_insert_with(|| {
                    let clr = palette[color_idx % palette.len()];
                    color_idx += 1;
                    clr
                });
                if let Some(refs) = self.refs.get(&c.oid) {
                    for r in refs {
                        let name = match r {
                            Ref::Branch(b) => b.name.clone(),
                            Ref::Tag(t) => t.name.clone(),
                            _ => continue,
                        };
                        ref_colors.entry(name).or_insert(color);
                    }
                }
                if let Some(gd) = graph_data.get_mut(&c.oid) {
                    gd.color = color;
                }
            }
        } else {
            let mut author_colors: HashMap<String, Color> = HashMap::new();
            for c in commits {
                let author_key = format!("{} <{}>", c.author.name, c.author.email);
                let color = *author_colors.entry(author_key).or_insert_with(|| {
                    let clr = palette[color_idx % palette.len()];
                    color_idx += 1;
                    clr
                });
                if let Some(gd) = graph_data.get_mut(&c.oid) {
                    gd.color = color;
                }
            }
        }
    }

    fn rebuild_nodes_with_colors(
        &self,
        sorted: &[&CommitInfo],
        graph_data: &HashMap<Oid, CommitGraphData>,
    ) -> Vec<NodePosition> {
        sorted
            .iter()
            .map(|c| {
                let gd = &graph_data[&c.oid];
                NodePosition {
                    oid: c.oid,
                    row: gd.row,
                    column: gd.column,
                    is_merge: gd.is_merge,
                    color: gd.color,
                    is_dimmed: false,
                    is_highlighted: false,
                }
            })
            .collect()
    }

    fn rebuild_edges_with_colors(
        &self,
        sorted: &[&CommitInfo],
        graph_data: &HashMap<Oid, CommitGraphData>,
    ) -> Vec<Edge> {
        let mut edges = Vec::new();
        for c in sorted {
            let c_row = graph_data[&c.oid].row;
            let c_col = graph_data[&c.oid].column;
            let c_color = graph_data[&c.oid].color;
            for (pi, &p_oid) in c.parent_oids.iter().enumerate() {
                if let Some(p_gd) = graph_data.get(&p_oid) {
                    let edge_type = if pi == 0 {
                        if c_col == p_gd.column {
                            EdgeType::Straight
                        } else {
                            EdgeType::Branch
                        }
                    } else {
                        EdgeType::Merge
                    };
                    let edge_style = if self.options.palette == GraphPalette::Default {
                        EdgeStyle::Solid
                    } else {
                        match c_col % 3 {
                            0 => EdgeStyle::Solid,
                            1 => EdgeStyle::Dashed,
                            _ => EdgeStyle::Dotted,
                        }
                    };
                    edges.push(Edge {
                        from_row: c_row,
                        from_col: c_col,
                        to_row: p_gd.row,
                        to_col: p_gd.column,
                        edge_type,
                        color: c_color,
                        is_dimmed: false,
                        edge_style,
                    });
                }
            }
        }
        edges
    }

    fn empty_layout(&self) -> GraphLayout {
        GraphLayout {
            nodes: Vec::new(),
            stash_markers: Vec::new(),
            edges: Vec::new(),
            total_columns: 0,
            orientation: self.options.orientation,
            total_rows: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};

    fn make_oid(n: u8) -> Oid {
        let mut bytes = [0u8; 20];
        bytes[0] = n;
        Oid::from_bytes(bytes)
    }

    fn make_commit(oid: u8, parents: Vec<u8>, msg: &str) -> CommitInfo {
        CommitInfo {
            oid: make_oid(oid),
            short_oid: format!("{oid:02x}00000"),
            message: msg.to_string(),
            summary: msg.to_string(),
            author: Author {
                name: "Test".to_string(),
                email: "test@test.com".to_string(),
            },
            committer: Author {
                name: "Test".to_string(),
                email: "test@test.com".to_string(),
            },
            author_time: Utc.timestamp_opt(1000 + oid as i64, 0).single().unwrap(),
            commit_time: Utc.timestamp_opt(1000 + oid as i64, 0).single().unwrap(),
            parent_oids: parents.into_iter().map(make_oid).collect(),
            refs: Vec::new(),
        }
    }

    #[test]
    fn linear_graph_single_column() {
        let c1 = make_commit(1, vec![], "first");
        let c2 = make_commit(2, vec![1], "second");
        let c3 = make_commit(3, vec![2], "third");
        let commits = vec![c3, c2, c1];
        let calc =
            GraphCalculator::new(commits, HashMap::new(), Vec::new(), GraphOptions::default());
        let layout = calc.calculate_layout();
        assert_eq!(layout.nodes.len(), 3);
        assert_eq!(layout.total_columns, 1);
        for node in &layout.nodes {
            assert_eq!(node.column, 0);
        }
    }

    #[test]
    fn branch_uses_two_columns() {
        let c1 = make_commit(1, vec![], "initial");
        let c2 = make_commit(2, vec![1], "on main");
        let c3 = make_commit(3, vec![1], "on branch");
        let commits = vec![c2, c3, c1];
        let calc =
            GraphCalculator::new(commits, HashMap::new(), Vec::new(), GraphOptions::default());
        let layout = calc.calculate_layout();
        assert_eq!(layout.nodes.len(), 3);
        let columns: HashSet<usize> = layout.nodes.iter().map(|n| n.column).collect();
        assert!(
            columns.len() >= 2,
            "branching should use at least 2 columns"
        );
    }

    #[test]
    fn property_topological_order() {
        let c1 = make_commit(1, vec![], "root");
        let c2 = make_commit(2, vec![1], "second");
        let c3 = make_commit(3, vec![2], "third");
        let c4 = make_commit(4, vec![3], "fourth");
        let commits = vec![c4, c3, c2, c1];
        let calc =
            GraphCalculator::new(commits, HashMap::new(), Vec::new(), GraphOptions::default());
        let layout = calc.calculate_layout();
        for edge in &layout.edges {
            assert!(
                edge.from_row < edge.to_row,
                "parent (to_row={}) should have higher row than child (from_row={})",
                edge.to_row,
                edge.from_row
            );
        }
    }

    #[test]
    fn property_completeness() {
        let c1 = make_commit(1, vec![], "a");
        let c2 = make_commit(2, vec![1], "b");
        let c3 = make_commit(3, vec![1], "c");
        let c4 = make_commit(4, vec![2, 3], "merge");
        let commits = vec![c4, c3, c2, c1];
        let parent_count: usize = commits.iter().map(|c| c.parent_oids.len()).sum();
        let calc =
            GraphCalculator::new(commits, HashMap::new(), Vec::new(), GraphOptions::default());
        let layout = calc.calculate_layout();
        assert_eq!(layout.nodes.len(), 4, "one node per commit");
        assert_eq!(layout.edges.len(), parent_count, "one edge per parent link");
    }

    #[test]
    fn property_branch_continuity() {
        let c1 = make_commit(1, vec![], "root");
        let c2 = make_commit(2, vec![1], "a");
        let c3 = make_commit(3, vec![2], "b");
        let c4 = make_commit(4, vec![3], "c");
        let commits = vec![c4, c3, c2, c1];
        let calc =
            GraphCalculator::new(commits, HashMap::new(), Vec::new(), GraphOptions::default());
        let layout = calc.calculate_layout();
        let columns: Vec<usize> = layout.nodes.iter().map(|n| n.column).collect();
        assert!(
            columns.iter().all(|&c| c == columns[0]),
            "linear chain should stay in same column (branch continuity)"
        );
    }

    #[test]
    fn hide_merges_removes_merge_commits() {
        let c1 = make_commit(1, vec![], "root");
        let c2 = make_commit(2, vec![1], "branch a");
        let c3 = make_commit(3, vec![1], "branch b");
        let c4 = make_commit(4, vec![2, 3], "merge");
        let commits = vec![c4, c3, c2, c1];
        let options = GraphOptions {
            hide_merges: true,
            ..GraphOptions::default()
        };
        let calc = GraphCalculator::new(commits, HashMap::new(), Vec::new(), options);
        let layout = calc.calculate_layout();
        assert_eq!(layout.nodes.len(), 3);
        assert!(layout.nodes.iter().all(|n| !n.is_merge));
    }

    #[test]
    fn hide_merges_rewires_child_of_merge() {
        let c1 = make_commit(1, vec![], "root");
        let c2 = make_commit(2, vec![1], "branch a");
        let c3 = make_commit(3, vec![1], "branch b");
        let c4 = make_commit(4, vec![2, 3], "merge");
        let c5 = make_commit(5, vec![4], "after merge");
        let commits = vec![c5, c4, c3, c2, c1];
        let options = GraphOptions {
            hide_merges: true,
            ..GraphOptions::default()
        };
        let calc = GraphCalculator::new(commits, HashMap::new(), Vec::new(), options);
        let layout = calc.calculate_layout();
        assert_eq!(layout.nodes.len(), 4, "merge removed, 4 nodes remain");
        assert!(layout.nodes.iter().all(|n| !n.is_merge));
        let after_merge = layout.nodes.iter().find(|n| n.oid == make_oid(5)).unwrap();
        let edges_to_after: Vec<_> = layout
            .edges
            .iter()
            .filter(|e| e.to_row == after_merge.row || e.from_row == after_merge.row)
            .collect();
        assert!(
            edges_to_after.iter().any(|e| {
                let other_oid = layout.nodes.iter().find(|n| {
                    n.row
                        == if e.from_row == after_merge.row {
                            e.to_row
                        } else {
                            e.from_row
                        }
                });
                other_oid.map_or(false, |n| n.oid == make_oid(2))
            }),
            "child of merge should be connected to merge's first parent"
        );
    }

    #[test]
    fn bottom_to_top_reverses_rows() {
        let c1 = make_commit(1, vec![], "first");
        let c2 = make_commit(2, vec![1], "second");
        let c3 = make_commit(3, vec![2], "third");
        let commits = vec![c3, c2, c1];
        let options = GraphOptions {
            orientation: GraphOrientation::BottomToTop,
            ..GraphOptions::default()
        };
        let calc = GraphCalculator::new(commits, HashMap::new(), Vec::new(), options);
        let layout = calc.calculate_layout();
        let mut nodes_by_oid: HashMap<Oid, &NodePosition> = HashMap::new();
        for n in &layout.nodes {
            nodes_by_oid.insert(n.oid, n);
        }
        assert!(
            nodes_by_oid[&make_oid(3)].row > nodes_by_oid[&make_oid(1)].row,
            "in bottom-to-top, newest commit (3) should have higher row (at bottom)"
        );
    }

    #[test]
    fn viewport_filters_nodes_and_edges() {
        let c1 = make_commit(1, vec![], "a");
        let c2 = make_commit(2, vec![1], "b");
        let c3 = make_commit(3, vec![2], "c");
        let c4 = make_commit(4, vec![3], "d");
        let c5 = make_commit(5, vec![4], "e");
        let commits = vec![c5, c4, c3, c2, c1];
        let calc =
            GraphCalculator::new(commits, HashMap::new(), Vec::new(), GraphOptions::default());
        let layout = calc.calculate_layout();
        let vp = layout.from_visible_range(1, 3);
        assert!(vp.nodes.len() <= 2);
        assert_eq!(vp.rows, 1..3);
    }

    #[test]
    fn empty_commits_produce_empty_layout() {
        let calc = GraphCalculator::new(
            Vec::new(),
            HashMap::new(),
            Vec::new(),
            GraphOptions::default(),
        );
        let layout = calc.calculate_layout();
        assert!(layout.nodes.is_empty());
        assert!(layout.edges.is_empty());
        assert_eq!(layout.total_rows, 0);
    }

    #[test]
    fn apply_dimming_highlights_matching() {
        let c1 = make_commit(1, vec![], "a");
        let c2 = make_commit(2, vec![1], "b");
        let commits = vec![c2, c1];
        let calc =
            GraphCalculator::new(commits, HashMap::new(), Vec::new(), GraphOptions::default());
        let mut layout = calc.calculate_layout();
        let matching: HashSet<Oid> = [make_oid(2)].into_iter().collect();
        GraphCalculator::apply_dimming(&mut layout, None, Some(&matching));
        let n1 = layout.nodes.iter().find(|n| n.oid == make_oid(1)).unwrap();
        let n2 = layout.nodes.iter().find(|n| n.oid == make_oid(2)).unwrap();
        assert!(n1.is_dimmed);
        assert!(!n1.is_highlighted);
        assert!(!n2.is_dimmed);
        assert!(n2.is_highlighted);
    }

    #[test]
    fn color_by_author_mode() {
        let c1 = make_commit(1, vec![], "a");
        let c2 = make_commit(2, vec![1], "b");
        let commits = vec![c2, c1];
        let options = GraphOptions {
            color_mode: GraphColorMode::ByAuthor,
            ..GraphOptions::default()
        };
        let calc = GraphCalculator::new(commits, HashMap::new(), Vec::new(), options);
        let layout = calc.calculate_layout();
        let colors: std::collections::HashSet<Color> =
            layout.nodes.iter().map(|n| n.color).collect();
        assert!(
            colors.len() >= 1,
            "color-by-author should assign colors to nodes"
        );
    }
}
