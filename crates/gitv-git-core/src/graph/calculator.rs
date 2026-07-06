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
        r: 51,
        g: 34,
        b: 136,
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
        r: 51,
        g: 34,
        b: 136,
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
        r: 51,
        g: 34,
        b: 136,
        a: 255,
    },
];

/// Number of rows below a child that a thread stays visible before removal.
/// Matches gitk's `downarrowlen`.
const DOWNARROW_LEN: usize = 5;

/// Number of rows above a parent that a thread is pre-inserted.
/// Matches gitk's `uparrowlen`.
const UPARROW_LEN: usize = 5;

const STASH_COLOR: Color = Color {
    r: 245,
    g: 158,
    b: 11,
    a: 255,
};

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
    #[must_use]
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

    /// Consume the calculator and return the owned commits, avoiding a clone.
    #[must_use]
    pub fn into_commits(self) -> Vec<CommitInfo> {
        self.commits
    }

    #[must_use]
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

        // --- gitk-style column assignment ---

        // Build displayorder: commit index by row
        let mut displayorder_idx: Vec<usize> = vec![0; commits.len()];
        for (i, c) in commits.iter().enumerate() {
            let row = *row_assignments.get(&c.oid).unwrap_or(&0);
            displayorder_idx[row] = i;
        }

        // Compute first-parent chain from HEAD for ordertoken tiebreaking.
        // When fork-point children share the same timestamp, the child on the
        // first-parent chain is preferred as the "chosen" continuation (gets
        // the smaller ordertoken → col 0), matching gitk's semantics.
        let head_oid = commits[displayorder_idx[0]].oid;
        let first_parent_chain: HashSet<Oid> =
            Self::compute_first_parent_chain(head_oid, &commits, &oid_index);

        // Sort children with FPC preference then by row, so children[p][0]
        // is the mainline child matching gitk display-order.
        let children_sorted =
            Self::sort_children_by_row(&children, &row_assignments, &first_parent_chain);

        // Compute ordertokens for stable column ordering
        let ordertokens = Self::compute_ordertokens(
            &commits,
            &children_sorted,
            &displayorder_idx,
            &oid_index,
            &first_parent_chain,
        );

        // Assign columns using row-by-row active-thread tracking (gitk algorithm)
        let mingap_len = self.options.arrow_gap_threshold;
        let (_, mut rowidlist) = Self::assign_columns(
            &displayorder_idx,
            &commits,
            &ordertokens,
            &children_sorted,
            &row_assignments,
            mingap_len,
        );

        // Build displayorder OIDs for optimize_rows and extract_columns
        let displayorder: Vec<Oid> = displayorder_idx.iter().map(|&ci| commits[ci].oid).collect();

        // Optimize: insert padding to separate branches and fix jaggies
        optimize_rows(&mut rowidlist, &displayorder, &children_sorted);

        // Extract final columns from optimized rowidlist
        let columns = extract_columns(&rowidlist, &displayorder);
        for (oid, col) in &columns {
            if let Some(gd) = graph_data.get_mut(oid) {
                gd.column = *col;
            }
        }

        let total_columns = graph_data
            .values()
            .map(|gd| gd.column)
            .max()
            .map(|c| c + 1)
            .unwrap_or(0);

        self.assign_colors_to_nodes(&sorted, &children_sorted, &mut graph_data);
        let mut nodes = self.rebuild_nodes_with_colors(&sorted, &graph_data);
        let mut edges = self.rebuild_edges_with_colors(&sorted, &graph_data, &rowidlist);

        let mut row_max_column: Vec<usize> = rowidlist
            .iter()
            .map(|row| {
                row.iter()
                    .rposition(|e| e.is_some())
                    .map(|i| i + 1)
                    .unwrap_or(0)
            })
            .collect();

        let (mut stash_markers, extra_cols) =
            self.insert_stash_nodes(&mut nodes, &mut edges, &mut row_max_column);
        let total_rows = nodes
            .iter()
            .map(|n| n.row)
            .max()
            .map(|r| r + 1)
            .unwrap_or(0);
        let total_columns = total_columns + extra_cols;

        // Adjust row_max_column for edge horizontal segments (merge/branch fan-out).
        // rowidlist captures threads entering each row but misses the parent columns
        // that merge edges fan out to at the merge commit's own row.
        for edge in &edges {
            let max_col = edge.from_col.max(edge.to_col) + 1;
            if edge.from_row < row_max_column.len() {
                row_max_column[edge.from_row] = row_max_column[edge.from_row].max(max_col);
            }
            if edge.to_row < row_max_column.len() {
                row_max_column[edge.to_row] = row_max_column[edge.to_row].max(max_col);
            }
            for &(row, col) in &edge.waypoints {
                if row < row_max_column.len() {
                    row_max_column[row] = row_max_column[row].max(col + 1);
                }
            }
        }

        let stash_commits: Vec<CommitInfo> = stash_markers
            .iter()
            .map(|sm| {
                let stash = &self.stashes[sm.stash_index];
                CommitInfo {
                    oid: stash.oid,
                    short_oid: stash.oid.short_hex(),
                    message: stash.message.clone(),
                    summary: stash.message.clone(),
                    author: stash.author.clone(),
                    committer: stash.author.clone(),
                    author_time: stash.time,
                    commit_time: stash.time,
                    parent_oids: vec![stash.parent_oid],
                    refs: vec![],
                }
            })
            .collect();

        // Post-process edges to re-route any obstructed same-column segments
        Self::fix_edge_pass_throughs(&nodes, &mut edges);

        if self.options.orientation == GraphOrientation::BottomToTop {
            let max_row = nodes.iter().map(|n| n.row).max().unwrap_or(0);
            for node in &mut nodes {
                node.row = max_row - node.row;
            }
            for edge in &mut edges {
                let new_from = max_row - edge.from_row;
                let new_to = max_row - edge.to_row;
                edge.from_row = new_from;
                edge.to_row = new_to;
                for wp in &mut edge.waypoints {
                    wp.0 = max_row - wp.0;
                }
                if let Some(ref mut gap) = edge.arrow_gap {
                    gap.0 = max_row - gap.0;
                    gap.1 = max_row - gap.1;
                }
            }
            for marker in &mut stash_markers {
                marker.row = max_row - marker.row;
            }
            row_max_column.reverse();
        }

        GraphLayout {
            nodes,
            stash_markers,
            edges,
            total_columns,
            orientation: self.options.orientation,
            total_rows,
            stash_commits,
            row_max_column,
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

    #[must_use]
    pub fn get_ancestor_oids(&self, oid: &Oid) -> HashSet<Oid> {
        let oid_index: HashMap<Oid, usize> = self
            .commits
            .iter()
            .enumerate()
            .map(|(i, c)| (c.oid, i))
            .collect();
        let mut ancestors = HashSet::new();
        let mut stack = vec![*oid];
        while let Some(current) = stack.pop() {
            if !ancestors.insert(current) {
                continue;
            }
            if let Some(&ci) = oid_index.get(&current) {
                for parent in &self.commits[ci].parent_oids {
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
        let mut changed = true;
        while changed {
            changed = false;
            for commit in &self.commits {
                if commit.parent_oids.len() <= 1 {
                    continue;
                }
                let first_parent = match commit.parent_oids.first() {
                    Some(p) => *p,
                    None => continue,
                };
                let resolved = Self::resolve_to_non_merge(&first_parent, &merge_oids, &resolve);
                if resolve.get(&commit.oid) != Some(&resolved) {
                    resolve.insert(commit.oid, resolved);
                    changed = true;
                }
            }
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

    /// Compute display row order using Kahn's algorithm with a time-based
    /// priority queue (matching `git rev-list --topo-order` behavior).
    ///
    /// A commit is only assigned a row after ALL its children have been
    /// assigned rows (topological constraint). When multiple commits are
    /// available, the one with the highest commit_time (newest) is chosen.
    /// This keeps first-parent chains contiguous and avoids interleaving
    /// unrelated branches.
    fn temporal_topological_sort(
        commits: &[CommitInfo],
        children: &HashMap<Oid, Vec<Oid>>,
    ) -> HashMap<Oid, usize> {
        let n = commits.len();
        let oid_to_idx: HashMap<Oid, usize> = commits
            .iter()
            .enumerate()
            .map(|(i, c)| (c.oid, i))
            .collect();

        // Count in-degree (number of unvisited children) for each commit.
        // A commit becomes "available" when its in-degree reaches 0.
        let mut indegree: Vec<usize> = (0..n)
            .map(|i| {
                children
                    .get(&commits[i].oid)
                    .map(|kids| kids.iter().filter(|k| oid_to_idx.contains_key(*k)).count())
                    .unwrap_or(0)
            })
            .collect();

        // Max-heap by commit_time (newest first). Ties broken by oid bytes
        // (largest first — any consistent tiebreaker suffices). Raw bytes give
        // the same ordering as the hex string but avoid a per-commit allocation.
        let mut heap: std::collections::BinaryHeap<(i64, [u8; 20], usize)> =
            std::collections::BinaryHeap::new();

        for i in 0..n {
            if indegree[i] == 0 {
                heap.push((
                    commits[i].commit_time.timestamp(),
                    *commits[i].oid.as_bytes(),
                    i,
                ));
            }
        }

        let mut rows = vec![0usize; n];
        let mut row_counter = 0usize;

        while let Some((_, _, idx)) = heap.pop() {
            rows[idx] = row_counter;
            row_counter += 1;

            for parent_oid in &commits[idx].parent_oids {
                if let Some(&parent_idx) = oid_to_idx.get(parent_oid) {
                    indegree[parent_idx] -= 1;
                    if indegree[parent_idx] == 0 {
                        heap.push((
                            commits[parent_idx].commit_time.timestamp(),
                            *commits[parent_idx].oid.as_bytes(),
                            parent_idx,
                        ));
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

    /// Walk first-parent pointers from HEAD to build the first-parent chain set.
    /// Used for ordertoken tiebreaking at fork points.
    fn compute_first_parent_chain(
        head_oid: Oid,
        commits: &[CommitInfo],
        oid_index: &HashMap<Oid, usize>,
    ) -> HashSet<Oid> {
        let mut chain = HashSet::new();
        let mut oid = head_oid;
        loop {
            chain.insert(oid);
            let Some(&idx) = oid_index.get(&oid) else {
                break;
            };
            let c = &commits[idx];
            if c.parent_oids.is_empty() {
                break;
            }
            oid = c.parent_oids[0];
        }
        chain
    }

    /// Sort each commit's children list with first-parent-chain preference,
    /// then by row ascending.
    ///
    /// FPC children sort first so `children_sorted[p][0]` is always the
    /// mainline child (matching gitk's display-order where the FPC child
    /// appears first in `git rev-list --topo-order`). Within each group
    /// (FPC vs non-FPC), children are sorted by row ascending (youngest
    /// first), which matches gitk's reverse-topological display order.
    ///
    /// This ordering is used throughout the algorithm for ordertoken walk,
    /// fork-point differentiation, makeupline trigger, and optimize_rows
    /// arrow detection.
    fn sort_children_by_row(
        children: &HashMap<Oid, Vec<Oid>>,
        row_assignments: &HashMap<Oid, usize>,
        first_parent_chain: &HashSet<Oid>,
    ) -> HashMap<Oid, Vec<Oid>> {
        let mut sorted = children.clone();
        for kids in sorted.values_mut() {
            kids.sort_by(|a, b| {
                let a_fpc = first_parent_chain.contains(a);
                let b_fpc = first_parent_chain.contains(b);
                // FPC children first (false < true in sort)
                match (a_fpc, b_fpc) {
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                    _ => {
                        let a_row = row_assignments.get(a).copied().unwrap_or(usize::MAX);
                        let b_row = row_assignments.get(b).copied().unwrap_or(usize::MAX);
                        a_row.cmp(&b_row)
                    }
                }
            });
        }
        sorted
    }

    /// Compute ordertokens for all commits (gitk algorithm).
    ///
    /// Each commit gets a string token encoding its path from HEAD via the
    /// youngest-child chain. First parent appends `""`, second parent appends
    /// `"1"`, etc. Lexicographic comparison of tokens determines column order:
    /// the mainline (first-parent chain from HEAD) gets token `""` and sits
    /// leftmost; branches get longer tokens and go right.
    ///
    /// **Branch differentiation**: After the initial pass (which assigns `""`
    /// to all first-parent-only chains), a post-processing step differentiates
    /// branches at fork points — commits with multiple children. The first
    /// (youngest) child keeps the fork point's base token; subsequent children
    /// get their child index (`"1"`, `"2"`, …) appended to their entire
    /// descendant subtree. Without this, all non-merge branches would share
    /// token `""` and collapse into the same column.
    ///
    /// Commits are processed in row order (row 0 first) so that children
    /// always have their tokens computed before their parents.
    fn compute_ordertokens(
        commits: &[CommitInfo],
        children_sorted: &HashMap<Oid, Vec<Oid>>,
        displayorder_idx: &[usize],
        oid_index: &HashMap<Oid, usize>,
        first_parent_chain: &HashSet<Oid>,
    ) -> HashMap<Oid, String> {
        let mut ordertokens: HashMap<Oid, String> = HashMap::new();

        // Initial pass: assign base tokens via youngest-child chain.
        for &ci in displayorder_idx {
            let c = &commits[ci];
            match children_sorted.get(&c.oid).and_then(|kids| kids.first()) {
                Some(&first_child_oid) => {
                    let child_token = ordertokens
                        .get(&first_child_oid)
                        .cloned()
                        .unwrap_or_default();
                    let &child_ci = oid_index
                        .get(&first_child_oid)
                        .expect("first child oid must exist in oid_index");
                    let parent_idx = commits[child_ci]
                        .parent_oids
                        .iter()
                        .position(|&p| p == c.oid)
                        .unwrap_or(0);
                    let segment = if parent_idx == 0 {
                        String::new()
                    } else {
                        parent_idx.to_string()
                    };
                    ordertokens.insert(c.oid, child_token + &segment);
                }
                None => {
                    // No children — HEAD or orphan; gets base token
                    ordertokens.insert(c.oid, String::new());
                }
            }
        }

        // Post-process: differentiate branches at fork points.
        // At each fork point (commit with multiple children), the first
        // (youngest) child inherits the fork point's token. Other children
        // get a suffix (child index) appended to their entire descendant
        // subtree, ensuring each branch sorts into a distinct column.
        //
        // Propagation skips children on the first-parent chain (mainline) to
        // prevent side-branch suffixes from leaking through merge points
        // into the mainline. Other children (side-branch continuations after
        // a merge) continue to receive the suffix normally.
        for &ci in displayorder_idx {
            let oid = commits[ci].oid;
            let Some(kids) = children_sorted.get(&oid) else {
                continue;
            };
            if kids.len() <= 1 {
                continue;
            }
            for (child_idx, &child_oid) in kids.iter().enumerate() {
                if child_idx == 0 {
                    continue;
                }
                let suffix = child_idx.to_string();
                propagate_branch_token(
                    child_oid,
                    children_sorted,
                    &mut ordertokens,
                    &suffix,
                    first_parent_chain,
                );
            }
        }

        ordertokens
    }

    /// Find the insertion position for `id` in `idlist` using a hint
    /// (gitk's idcol algorithm). When the new element's ordertoken equals
    /// the token at the hint position, the new element is inserted AT the
    /// hint (pushing existing elements right), so the first parent inherits
    /// its child's column.
    fn idcol(idlist: &[Oid], id: Oid, ordertokens: &HashMap<Oid, String>, hint: usize) -> usize {
        if idlist.is_empty() {
            return 0;
        }

        let t = ordertokens.get(&id).map(String::as_str).unwrap_or("");

        if hint >= idlist.len() {
            for p in (0..idlist.len()).rev() {
                let token_p = ordertokens
                    .get(&idlist[p])
                    .map(String::as_str)
                    .unwrap_or("");
                if t >= token_p {
                    return p + 1;
                }
            }
            return 0;
        }

        let token_at_hint = ordertokens
            .get(&idlist[hint])
            .map(String::as_str)
            .unwrap_or("");

        if t < token_at_hint {
            for p in (0..hint).rev() {
                let token_p = ordertokens
                    .get(&idlist[p])
                    .map(String::as_str)
                    .unwrap_or("");
                if t >= token_p {
                    return p + 1;
                }
            }
            0
        } else if t > token_at_hint {
            let mut j = hint + 1;
            while j < idlist.len() {
                let token_j = ordertokens
                    .get(&idlist[j])
                    .map(String::as_str)
                    .unwrap_or("");
                if t >= token_j {
                    j += 1;
                } else {
                    break;
                }
            }
            j
        } else {
            hint
        }
    }

    /// Find the next row >= `from_row` where `oid` is referenced (by a child
    /// or the commit itself). Returns None if not found.
    /// Matches gitk's `nextuse` function.
    ///
    /// gitk iterates children in display-row order (ascending) and returns the
    /// first child with row > from_row. gitv's `children_sorted` is sorted by
    /// row ascending (matching gitk's display order), but we scan all children
    /// and return the minimum row that is > from_row for consistency.
    fn nextuse(
        oid: Oid,
        from_row: usize,
        children_sorted: &HashMap<Oid, Vec<Oid>>,
        row_assignments: &HashMap<Oid, usize>,
    ) -> Option<usize> {
        let mut nearest: Option<usize> = None;
        if let Some(kids) = children_sorted.get(&oid) {
            for &kid in kids {
                if let Some(&kid_row) = row_assignments.get(&kid)
                    && kid_row > from_row
                {
                    nearest = Some(match nearest {
                        Some(prev) => prev.min(kid_row),
                        None => kid_row,
                    });
                }
            }
        }
        // Fall back to the commit's own row if no child is further down
        nearest.or_else(|| row_assignments.get(&oid).copied())
    }

    /// Find the most recent row < `from_row` where `oid` is referenced by a
    /// child. Returns None if not found.
    /// Matches gitk's `prevuse` function.
    ///
    /// gitk iterates children in display-row order and accumulates the last
    /// one with row < from_row. gitv's `children_sorted` is sorted by row
    /// ascending (matching gitk's display order), but we scan all children
    /// and return the maximum row that is < from_row for consistency.
    fn prevuse(
        oid: Oid,
        from_row: usize,
        children_sorted: &HashMap<Oid, Vec<Oid>>,
        row_assignments: &HashMap<Oid, usize>,
    ) -> Option<usize> {
        let kids = children_sorted.get(&oid)?;
        let mut ret: Option<usize> = None;
        for &kid in kids {
            if let Some(&kid_row) = row_assignments.get(&kid)
                && kid_row < from_row
            {
                ret = Some(match ret {
                    Some(prev) => prev.max(kid_row),
                    None => kid_row,
                });
            }
        }
        ret
    }

    /// Rebuild the idlist from scratch for a cold-start row.
    ///
    /// Scans backward for recently-removed threads whose next use is near,
    /// active parent threads, and forward for upcoming commits and their
    /// parents. Returns a sorted Vec<Oid> ready for use as the idlist.
    ///
    /// Matches gitk's `make_idlist` function. Called when thread removal
    /// empties the idlist entirely.
    #[allow(clippy::too_many_arguments)]
    fn make_idlist(
        row: usize,
        displayorder_idx: &[usize],
        commits: &[CommitInfo],
        ordertokens: &HashMap<Oid, String>,
        children_sorted: &HashMap<Oid, Vec<Oid>>,
        row_assignments: &HashMap<Oid, usize>,
        mingap_len: usize,
    ) -> Vec<Oid> {
        let n = displayorder_idx.len();
        if n == 0 || row >= n {
            return Vec::new();
        }

        let r_start = row.saturating_sub(mingap_len + DOWNARROW_LEN + 1);
        let ra = row.saturating_sub(DOWNARROW_LEN);
        let rb = (row + UPARROW_LEN).min(n);

        let mut entries: Vec<(String, Oid)> = Vec::new();

        // Helper: get next commit oid at r+1 (for first-parent skip)
        let next_oid_at = |r: usize| -> Option<Oid> {
            if r + 1 < n {
                Some(commits[displayorder_idx[r + 1]].oid)
            } else {
                None
            }
        };

        // Phase 1: recently removed threads (r_start..ra) whose next use is near
        #[allow(clippy::needless_range_loop)]
        for r in r_start..ra.min(n) {
            let ci = displayorder_idx[r];
            let next_oid = next_oid_at(r);
            for &p_oid in &commits[ci].parent_oids {
                if Some(p_oid) == next_oid {
                    continue;
                }
                if !ordertokens.contains_key(&p_oid) {
                    continue;
                }
                if let Some(nr) = Self::nextuse(p_oid, r, children_sorted, row_assignments)
                    && nr >= row
                    && nr <= r + DOWNARROW_LEN + mingap_len + UPARROW_LEN
                {
                    entries.push((ordertokens[&p_oid].clone(), p_oid));
                }
            }
        }

        // Phase 2: active threads (ra..row) — parents still unresolved
        #[allow(clippy::needless_range_loop)]
        for r in ra..row.min(n) {
            let ci = displayorder_idx[r];
            let next_oid = next_oid_at(r);
            for &p_oid in &commits[ci].parent_oids {
                if Some(p_oid) == next_oid {
                    continue;
                }
                if !ordertokens.contains_key(&p_oid) {
                    continue;
                }
                let nr = Self::nextuse(p_oid, r, children_sorted, row_assignments);
                if nr.is_none() || nr.unwrap() >= row {
                    entries.push((ordertokens[&p_oid].clone(), p_oid));
                }
            }
        }

        // Phase 3: current commit
        let curr_oid = commits[displayorder_idx[row]].oid;
        if let Some(token) = ordertokens.get(&curr_oid) {
            entries.push((token.clone(), curr_oid));
        }

        // Phase 4: upcoming commits and their parents (row..rb) — pre-insertion.
        // gitk: for each r in row..rb, add parents of r whose first_child row < row,
        // then also add displayorder[r+1] if its first_child row < row.
        // "first_child" in gitk = children[p][0] in display order = youngest child.
        // gitv's children_sorted is sorted by row ascending (same semantics), but we
        // use the child with the minimum row to be safe.
        #[allow(clippy::needless_range_loop)]
        for r in row..rb {
            if r >= n {
                break;
            }
            let ci = displayorder_idx[r];
            for &p_oid in &commits[ci].parent_oids {
                if !ordertokens.contains_key(&p_oid) {
                    continue;
                }
                let should_add = children_sorted
                    .get(&p_oid)
                    .and_then(|kids| {
                        kids.iter()
                            .filter_map(|k| row_assignments.get(k).copied())
                            .min()
                    })
                    .is_some_and(|min_row| min_row < row);
                if should_add {
                    entries.push((ordertokens[&p_oid].clone(), p_oid));
                }
            }
            // gitk also checks displayorder[r+1] directly
            let next_r = r + 1;
            if next_r < n {
                let next_oid = commits[displayorder_idx[next_r]].oid;
                if ordertokens.contains_key(&next_oid) {
                    let should_add = children_sorted
                        .get(&next_oid)
                        .and_then(|kids| {
                            kids.iter()
                                .filter_map(|k| row_assignments.get(k).copied())
                                .min()
                        })
                        .is_some_and(|min_row| min_row < row);
                    if should_add {
                        entries.push((ordertokens[&next_oid].clone(), next_oid));
                    }
                }
            }
        }

        // Sort by ordertoken, deduplicate by oid, extract
        entries.sort_by(|a, b| a.0.cmp(&b.0));
        entries.dedup_by(|a, b| a.1 == b.1);
        entries.into_iter().map(|(_, oid)| oid).collect()
    }

    /// Assign columns using gitk's row-by-row active-thread tracking.
    ///
    /// Implements the full gitk thread lifecycle:
    /// - Thread removal: long threads are removed from the idlist, creating gaps
    /// - Thread pre-insertion: upcoming branch tips are inserted `uparrowlen` rows early
    ///
    /// Returns (columns, rowidlist). The rowidlist stores the full active-
    /// thread list at each row, needed by optimize_rows and thread tracing.
    fn assign_columns(
        displayorder_idx: &[usize],
        commits: &[CommitInfo],
        ordertokens: &HashMap<Oid, String>,
        children_sorted: &HashMap<Oid, Vec<Oid>>,
        row_assignments: &HashMap<Oid, usize>,
        mingap_len: usize,
    ) -> (HashMap<Oid, usize>, Vec<Vec<Option<Oid>>>) {
        let n = displayorder_idx.len();
        if n == 0 {
            return (HashMap::new(), Vec::new());
        }

        let mut idlist: Vec<Oid> = Vec::with_capacity(32);
        let mut columns: HashMap<Oid, usize> = HashMap::new();
        let mut rowidlist: Vec<Vec<Option<Oid>>> = Vec::with_capacity(n);

        // Row 0: just the HEAD commit
        let head_oid = commits[displayorder_idx[0]].oid;
        idlist.push(head_oid);
        columns.insert(head_oid, 0);
        rowidlist.push(idlist.iter().map(|&o| Some(o)).collect());

        for row in 1..n {
            let prev_ci = displayorder_idx[row - 1];
            let curr_ci = displayorder_idx[row];
            let prev = &commits[prev_ci];
            let curr = &commits[curr_ci];

            // Remove prev from idlist (it's been drawn)
            let removed_col = idlist.iter().position(|&x| x == prev.oid);
            if let Some(col) = removed_col {
                let _removed = idlist.remove(col);
            }

            // Add prev's parents using removed column as hint (gitk algorithm).
            let mut hint = removed_col.unwrap_or(0);
            for &p_oid in &prev.parent_oids {
                if ordertokens.contains_key(&p_oid) && !idlist.contains(&p_oid) {
                    let col = Self::idcol(&idlist, p_oid, ordertokens, hint);
                    idlist.insert(col, p_oid);
                    // If prev is not p_oid's first (youngest) child, the thread
                    // was likely removed earlier. Patch it back into prior rows
                    // so the edge has a proper downward arrow segment.
                    let is_first_child = children_sorted
                        .get(&p_oid)
                        .and_then(|kids| kids.first())
                        .is_some_and(|&fk| fk == prev.oid);
                    if !is_first_child {
                        makeupline(
                            p_oid,
                            row - 1,
                            row,
                            col,
                            &mut rowidlist,
                            children_sorted,
                            row_assignments,
                            ordertokens,
                            mingap_len,
                        );
                    }
                    hint = col;
                }
            }

            // Thread removal: remove threads whose next use is too far away.
            // Matches gitk's layoutrows thread removal at termrow = row - downarrowlen - 1.
            if row > DOWNARROW_LEN {
                let termrow = row - DOWNARROW_LEN - 1;
                let term_ci = displayorder_idx[termrow];
                let term_commit = &commits[term_ci];
                for &p_oid in &term_commit.parent_oids {
                    if let Some(i) = idlist.iter().position(|&x| x == p_oid) {
                        let nr = Self::nextuse(p_oid, termrow, children_sorted, row_assignments);
                        if nr.is_none() || nr.unwrap() >= row + mingap_len + UPARROW_LEN {
                            let _ = idlist.remove(i);
                        }
                    }
                }
            }

            // Cold-start: if thread removal emptied the idlist, rebuild from
            // scratch. Matches gitk's make_idlist cold-start path.
            if idlist.is_empty() {
                idlist = Self::make_idlist(
                    row,
                    displayorder_idx,
                    commits,
                    ordertokens,
                    children_sorted,
                    row_assignments,
                    mingap_len,
                );
                let col = idlist.iter().position(|&x| x == curr.oid).unwrap_or(0);
                columns.insert(curr.oid, col);
                rowidlist.push(idlist.iter().map(|&o| Some(o)).collect());
                continue;
            }

            // Ensure curr is in idlist. Use default hint 0 (matching gitk's
            // idcol default) so newly appearing branch tips go to the leftmost
            // matching position when ordertokens are equal.
            if !idlist.contains(&curr.oid) {
                let col = Self::idcol(&idlist, curr.oid, ordertokens, 0);
                idlist.insert(col, curr.oid);
                // If curr has children and was thread-removed, patch its
                // thread back into prior rows so edges from children have
                // a proper upward arrow segment near curr.
                let has_children = children_sorted
                    .get(&curr.oid)
                    .is_some_and(|kids| !kids.is_empty());
                if has_children {
                    makeupline(
                        curr.oid,
                        row - 1,
                        row,
                        col,
                        &mut rowidlist,
                        children_sorted,
                        row_assignments,
                        ordertokens,
                        mingap_len,
                    );
                }
            }

            // Thread pre-insertion: insert parents of upcoming commits so
            // their threads appear `uparrowlen` rows early.
            // Matches gitk's layoutrows pre-insertion logic.
            let pre_row = row + UPARROW_LEN - 1;
            if pre_row < n {
                let pre_ci = displayorder_idx[pre_row];
                let pre_commit = &commits[pre_ci];
                let curr_col = idlist.iter().position(|&x| x == curr.oid).unwrap_or(0);
                let mut hint = curr_col;
                for &p_oid in &pre_commit.parent_oids {
                    if !idlist.contains(&p_oid) {
                        // gitk checks children[p][0] (first/youngest child in display order) < row.
                        // gitv's children_sorted is sorted by row ascending (same semantics), but we
                        // use the child with the minimum row for safety.
                        let should_insert = children_sorted
                            .get(&p_oid)
                            .and_then(|kids| {
                                kids.iter()
                                    .filter_map(|k| row_assignments.get(k).copied())
                                    .min()
                            })
                            .is_some_and(|min_row| min_row < row);
                        if should_insert {
                            let col = Self::idcol(&idlist, p_oid, ordertokens, hint);
                            idlist.insert(col, p_oid);
                            hint = col;
                        }
                    }
                }
                // Also check the commit at pre_row + 1
                if pre_row + 1 < n {
                    let next_ci = displayorder_idx[pre_row + 1];
                    let next_oid = commits[next_ci].oid;
                    if !idlist.contains(&next_oid) {
                        // Same earliest-displayed-child check
                        let should_insert = children_sorted
                            .get(&next_oid)
                            .and_then(|kids| {
                                kids.iter()
                                    .filter_map(|k| row_assignments.get(k).copied())
                                    .min()
                            })
                            .is_some_and(|min_row| min_row < row);
                        if should_insert {
                            let col = Self::idcol(&idlist, next_oid, ordertokens, hint);
                            idlist.insert(col, next_oid);
                        }
                    }
                }
            }

            // Record column of curr
            let col = idlist
                .iter()
                .position(|&x| x == curr.oid)
                .expect("curr.oid must be in idlist after insertion");
            columns.insert(curr.oid, col);

            // Store full rowidlist for optimize_rows + thread tracing
            rowidlist.push(idlist.iter().map(|&o| Some(o)).collect());
        }

        (columns, rowidlist)
    }

    fn insert_stash_nodes(
        &self,
        nodes: &mut Vec<NodePosition>,
        edges: &mut Vec<Edge>,
        row_max_column: &mut Vec<usize>,
    ) -> (Vec<StashMarker>, usize) {
        let node_map: HashMap<Oid, &NodePosition> = nodes.iter().map(|n| (n.oid, n)).collect();
        let mut by_parent: Vec<(usize, &StashEntry, usize, usize)> = Vec::new();
        for (idx, stash) in self.stashes.iter().enumerate() {
            if let Some(parent) = node_map.get(&stash.parent_oid) {
                by_parent.push((idx, stash, parent.row, parent.column));
            }
        }
        by_parent.sort_by_key(|(_, _, row, _)| *row);

        let base_max_col = nodes.iter().map(|n| n.column).max().unwrap_or(0);
        let mut extra_cols: usize = 0;
        let mut parent_branch_col: HashMap<Oid, usize> = HashMap::new();

        let mut stash_markers: Vec<StashMarker> = Vec::new();
        for (idx, stash, parent_row, parent_col) in &by_parent {
            let after_parent = stash_markers
                .iter()
                .filter(|sm: &&StashMarker| sm.parent_oid == stash.parent_oid)
                .count();
            let insert_row = parent_row + after_parent;

            let branch_col = match parent_branch_col.entry(stash.parent_oid) {
                std::collections::hash_map::Entry::Occupied(e) => *e.get(),
                std::collections::hash_map::Entry::Vacant(e) => {
                    extra_cols += 1;
                    let col = base_max_col + extra_cols;
                    e.insert(col);
                    col
                }
            };

            for node in nodes.iter_mut() {
                if node.row >= insert_row {
                    node.row += 1;
                }
            }
            for edge in edges.iter_mut() {
                if edge.from_row >= insert_row {
                    edge.from_row += 1;
                }
                if edge.to_row >= insert_row {
                    edge.to_row += 1;
                }
                for (r, _) in &mut edge.waypoints {
                    if *r >= insert_row {
                        *r += 1;
                    }
                }
                if let Some((lo, hi)) = &mut edge.arrow_gap {
                    if *lo >= insert_row {
                        *lo += 1;
                    }
                    if *hi >= insert_row {
                        *hi += 1;
                    }
                }
            }

            while row_max_column.len() < insert_row {
                row_max_column.push(0);
            }
            let orig_threads = row_max_column.get(insert_row).copied().unwrap_or(0);
            row_max_column.insert(insert_row, orig_threads.max(branch_col + 1));

            let parent_new_row = nodes
                .iter()
                .find(|n| n.oid == stash.parent_oid)
                .map(|n| n.row)
                .unwrap_or(*parent_row + 1);

            nodes.push(NodePosition {
                oid: stash.oid,
                row: insert_row,
                column: branch_col,
                is_merge: false,
                is_stash: true,
                color: STASH_COLOR,
                is_dimmed: false,
                is_highlighted: false,
            });

            edges.push(Edge {
                from_row: insert_row,
                from_col: branch_col,
                to_row: parent_new_row,
                to_col: *parent_col,
                edge_type: EdgeType::Straight,
                color: STASH_COLOR,
                is_dimmed: false,
                edge_style: EdgeStyle::Dashed,
                waypoints: Vec::new(),
                arrow_gap: None,
            });

            stash_markers.push(StashMarker {
                row: insert_row,
                column: branch_col,
                stash_index: *idx,
                stash_oid: stash.oid,
                parent_oid: stash.parent_oid,
                message: stash.message.clone(),
            });
        }

        let node_row_map: HashMap<Oid, usize> = nodes.iter().map(|n| (n.oid, n.row)).collect();
        for marker in &mut stash_markers {
            if let Some(&row) = node_row_map.get(&marker.stash_oid) {
                marker.row = row;
            }
        }

        (stash_markers, extra_cols)
    }

    fn assign_colors_to_nodes(
        &self,
        sorted: &[&CommitInfo],
        children_sorted: &HashMap<Oid, Vec<Oid>>,
        graph_data: &mut HashMap<Oid, CommitGraphData>,
    ) {
        let palette = palette_for(self.options.palette);
        let mut color_idx = 0usize;

        let commit_map: HashMap<Oid, &CommitInfo> = sorted.iter().map(|c| (c.oid, *c)).collect();

        if self.options.color_mode == GraphColorMode::ByBranch {
            let mut lane_colors: HashMap<usize, Color> = HashMap::new();
            let mut ref_colors: HashMap<String, Color> = HashMap::new();
            for c in sorted {
                let col = graph_data.get(&c.oid).map(|gd| gd.column).unwrap_or(0);

                // Linear chain inheritance: if this commit has exactly one
                // child and that child has exactly one parent, inherit the
                // child's color. This keeps linear chains uniformly colored
                // even when optimize_rows shifts columns.
                let inherited = children_sorted.get(&c.oid).and_then(|kids| {
                    if kids.len() == 1 {
                        let child_oid = kids[0];
                        let child_one_parent = commit_map
                            .get(&child_oid)
                            .map(|ci| ci.parent_oids.len() == 1)
                            .unwrap_or(false);
                        if child_one_parent {
                            return graph_data.get(&child_oid).map(|gd| gd.color);
                        }
                    }
                    None
                });

                let color = inherited.unwrap_or_else(|| {
                    *lane_colors.entry(col).or_insert_with(|| {
                        let clr = palette[color_idx % palette.len()];
                        color_idx += 1;
                        clr
                    })
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
            for c in sorted {
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
                    is_stash: false,
                    color: gd.color,
                    is_dimmed: false,
                    is_highlighted: false,
                }
            })
            .collect()
    }

    /// Post-process edges to re-route any same-column segment that passes
    /// through a node. Scans every consecutive pair in each edge's full
    /// path (endpoints + waypoints). When an obstruction is found, inserts
    /// detour waypoints into the existing waypoint list — never removes.
    fn fix_edge_pass_throughs(nodes: &[NodePosition], edges: &mut [Edge]) {
        let mut occupancy: HashSet<(usize, usize)> = HashSet::with_capacity(nodes.len());
        for n in nodes {
            occupancy.insert((n.row, n.column));
        }
        let max_col = nodes.iter().map(|n| n.column).max().unwrap_or(4) + 2;

        for edge in edges.iter_mut() {
            // Build list of consecutive pairs: endpoints + waypoints
            let mut segments: Vec<((usize, usize), (usize, usize))> = Vec::new();
            let mut prev = (edge.from_row, edge.from_col);
            for &wp in &edge.waypoints {
                segments.push((prev, wp));
                prev = wp;
            }
            segments.push((prev, (edge.to_row, edge.to_col)));

            // Find obstructed same-col segments and schedule insertions
            // (collected as (insert_after_index, detour_start, detour_end, route_col)
            // and applied in reverse to preserve indices).
            struct Insertion {
                after: usize,
                ds: usize,
                de: usize,
                rc: usize,
            }
            let mut insertions: Vec<Insertion> = Vec::new();
            let mut waypoint_idx = 0usize; // index into edge.waypoints at the END of each segment

            for (si, &(p1, p2)) in segments.iter().enumerate() {
                let is_last = si == segments.len() - 1;
                if p1.1 != p2.1 || is_last {
                    // Cross-col or last segment — no pass-through worry here
                    if !is_last {
                        waypoint_idx += 1;
                    }
                    continue;
                }

                // Same-column segment
                let col = p1.1;
                let (lo, hi) = (p1.0.min(p2.0), p1.0.max(p2.0));
                let obstructed: Vec<usize> = (lo + 1..hi)
                    .filter(|&r| occupancy.contains(&(r, col)))
                    .collect();

                if obstructed.is_empty() {
                    waypoint_idx += 1;
                    continue;
                }

                // Obstruction found — compute detour
                let first_obs = obstructed[0];
                let last_obs = obstructed[obstructed.len() - 1];
                let ds = hi.min(first_obs.saturating_sub(1)).max(lo + 1);
                let de = lo.max(last_obs + 1).min(hi - 1);
                if ds < de {
                    let rc = (0..max_col)
                        .filter(|&c| c != col)
                        .min_by_key(|&c| (ds..=de).filter(|&r| occupancy.contains(&(r, c))).count())
                        .unwrap_or(col + 1);
                    insertions.push(Insertion {
                        after: waypoint_idx,
                        ds,
                        de,
                        rc,
                    });
                }
                waypoint_idx += 1;
            }

            // Apply insertions in reverse order
            if !insertions.is_empty() {
                for ins in insertions.into_iter().rev() {
                    if ins.rc != Self::col_of_point(edge, ins.ds) {
                        edge.waypoints.insert(ins.after, (ins.ds, ins.rc));
                    }
                    if ins.rc != Self::col_of_point(edge, ins.de) {
                        edge.waypoints.insert(ins.after, (ins.de, ins.rc));
                    }
                }
            }
        }
    }

    /// Helper: column at a given row along the edge path.
    /// Interpolates by scanning edge endpoints and waypoints.
    fn col_of_point(edge: &Edge, target_row: usize) -> usize {
        let mut prev = (edge.from_row, edge.from_col);
        if target_row == prev.0 {
            return prev.1;
        }
        for &wp in &edge.waypoints {
            if target_row == wp.0 {
                return wp.1;
            }
            if target_row > prev.0 && target_row < wp.0 || target_row < prev.0 && target_row > wp.0
            {
                return prev.1; // between prev and wp — same as prev's column
            }
            prev = wp;
        }
        if target_row == edge.to_row {
            return edge.to_col;
        }
        edge.to_col
    }

    fn rebuild_edges_with_colors(
        &self,
        sorted: &[&CommitInfo],
        graph_data: &HashMap<Oid, CommitGraphData>,
        rowidlist: &[Vec<Option<Oid>>],
    ) -> Vec<Edge> {
        let mut edges = Vec::new();
        // Precompute each oid's (row, col) positions once so per-edge thread
        // tracing is a range lookup instead of a linear scan over all rows.
        let thread_positions = build_thread_index(rowidlist);

        // Build occupancy grid: which (row, col) has a node, used to detect
        // pass-through conflicts for same-column edges.
        let mut occupancy: HashSet<(usize, usize)> = HashSet::with_capacity(sorted.len());
        for c in sorted {
            let gd = &graph_data[&c.oid];
            occupancy.insert((gd.row, gd.column));
        }

        for c in sorted {
            let c_row = graph_data[&c.oid].row;
            let c_col = graph_data[&c.oid].column;
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

                    // Trace the parent's thread through the rowidlist to
                    // find direction-change waypoints (gitk drawlineseg approach).
                    let (mut waypoints, arrow_gap) =
                        trace_thread(p_oid, c_row, p_gd.row, p_gd.column, &thread_positions);

                    // For same-column edges, check whether a node sits in
                    // this column between the endpoints. If not, a straight
                    // vertical line is safe. If so, insert smooth routing
                    // waypoints to bypass the obstruction.
                    //
                    // trace_thread's own waypoints zigzag (the parent thread
                    // moves right then immediately left due to pad insertion),
                    // so we replace them with a clean detour via an adjacent
                    // column — which avoids both pass-through AND zigzag.
                    if c_col == p_gd.column && arrow_gap.is_none() {
                        let (lo, hi) = (c_row.min(p_gd.row), c_row.max(p_gd.row));
                        let obstructed: Vec<usize> = (lo + 1..hi)
                            .filter(|&r| occupancy.contains(&(r, c_col)))
                            .collect();
                        if obstructed.is_empty() {
                            waypoints = Vec::new();
                        } else {
                            let first_obs = obstructed[0];
                            let last_obs = obstructed[obstructed.len() - 1];
                            let detour_start = hi.min(first_obs.saturating_sub(1)).max(lo + 1);
                            let detour_end = lo.max(last_obs + 1).min(hi - 1);
                            if detour_start < detour_end {
                                // Find the column with fewest nodes in the
                                // detour range (preferring zero obstructions,
                                // settling for minimal remaining errors).
                                let max_col =
                                    graph_data.values().map(|gd| gd.column).max().unwrap_or(4) + 2;
                                let best_col =
                                    (0..max_col).filter(|&c| c != c_col).min_by_key(|&c| {
                                        (detour_start..=detour_end)
                                            .filter(|&r| occupancy.contains(&(r, c)))
                                            .count()
                                    });
                                if let Some(route_col) = best_col {
                                    waypoints =
                                        vec![(detour_start, route_col), (detour_end, route_col)];
                                }
                            }
                        }
                    }

                    // For cross-column edges without a gap, waypoints at the
                    // parent column are redundant — the chamfer handles routing.
                    // Only keep waypoints when they span multiple columns.
                    if arrow_gap.is_none() && c_col != p_gd.column && !waypoints.is_empty() {
                        let all_at_parent_col = waypoints.iter().all(|(_, c)| *c == p_gd.column);
                        if all_at_parent_col {
                            waypoints = Vec::new();
                        }
                    }

                    edges.push(Edge {
                        from_row: c_row,
                        from_col: c_col,
                        to_row: p_gd.row,
                        to_col: p_gd.column,
                        edge_type,
                        color: p_gd.color,
                        is_dimmed: false,
                        edge_style,
                        waypoints,
                        arrow_gap,
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
            stash_commits: Vec::new(),
            row_max_column: Vec::new(),
        }
    }
}

/// Recursively append `suffix` to the ordertoken of `start_oid` and all its
/// descendants in the children map.
///
/// This is used during ordertoken post-processing: when a fork point has
/// children [A, B, C], B's subtree gets `"1"` and C's subtree gets `"2"`,
/// ensuring each branch sorts into a distinct column via `idcol`.
///
/// Propagation skips children on the first-parent chain (mainline) at each
/// visited commit, preventing side-branch suffixes from leaking through
/// merge points into the mainline. Other children (side-branch continuations
/// after a merge) continue to receive the suffix normally.
fn propagate_branch_token(
    start_oid: Oid,
    children_sorted: &HashMap<Oid, Vec<Oid>>,
    ordertokens: &mut HashMap<Oid, String>,
    suffix: &str,
    first_parent_chain: &HashSet<Oid>,
) {
    // Never modify ordertokens of commits on the first-parent chain.
    if first_parent_chain.contains(&start_oid) {
        return;
    }
    let mut stack = vec![start_oid];
    let mut visited = HashSet::new();
    while let Some(oid) = stack.pop() {
        if !visited.insert(oid) {
            continue;
        }
        // Also guard every visited node against FPC membership
        // (e.g. a merge with both FPC and non-FPC parents on its children).
        if first_parent_chain.contains(&oid) {
            continue;
        }
        if let Some(token) = ordertokens.get_mut(&oid) {
            token.push_str(suffix);
        }
        if let Some(kids) = children_sorted.get(&oid) {
            for &kid in kids.iter().rev() {
                if first_parent_chain.contains(&kid) {
                    continue;
                }
                stack.push(kid);
            }
        }
    }
}

/// Simplify a path by removing collinear intermediate points.
///
/// Uses cross-multiplication to compare slopes: three points
/// (r0,c0), (r1,c1), (r2,c2) are collinear iff
/// `(c1-c0)*(r2-r1) == (c2-c1)*(r1-r0)`.
/// This correctly handles non-consecutive rows (e.g. across undetected
/// gaps in the rowidlist) where row deltas differ between segments.
fn simplify_collinear(path: &[(usize, usize)]) -> Vec<(usize, usize)> {
    let mut waypoints: Vec<(usize, usize)> = Vec::new();
    for &wp in path {
        while waypoints.len() >= 2 {
            let n = waypoints.len();
            let (r0, c0) = waypoints[n - 2];
            let (r1, c1) = waypoints[n - 1];
            let (r2, c2) = wp;
            // Cross-multiply to avoid float division:
            // slope01 = (c1-c0)/(r1-r0), slope12 = (c2-c1)/(r2-r1)
            let lhs = (c1 as i64 - c0 as i64) * (r2 as i64 - r1 as i64);
            let rhs = (c2 as i64 - c1 as i64) * (r1 as i64 - r0 as i64);
            if lhs == rhs {
                waypoints.pop();
            } else {
                break;
            }
        }
        waypoints.push(wp);
    }
    waypoints
}

/// Patch a thread (`oid`) back into the rowidlist for rows between its last
/// use and the current row. This creates proper arrow segments at both ends
/// of a thread-removal gap.
///
/// Matches gitk's `makeupline` function. Called when:
/// - A non-first-child parent is inserted into the idlist (it was removed
///   earlier by thread removal, and now needs to be visible again)
/// - The current commit reappears in the idlist after being thread-removed
///
/// `prev_row`: row of the commit whose parents are being processed (= rm1 in gitk)
/// `curr_row`: the current row being processed (= rend in gitk)
/// `col_hint`: column where `oid` was just inserted in the idlist
#[allow(clippy::too_many_arguments)]
fn makeupline(
    oid: Oid,
    prev_row: usize,
    curr_row: usize,
    col_hint: usize,
    rowidlist: &mut [Vec<Option<Oid>>],
    children_sorted: &HashMap<Oid, Vec<Oid>>,
    row_assignments: &HashMap<Oid, usize>,
    ordertokens: &HashMap<Oid, String>,
    mingap_len: usize,
) {
    // Walk backward through prevuse calls (gitk's loop: `for {set r $rend} {1} {set r $rstart}`)
    // until we find a use strictly before prev_row, or exhaust all previous uses.
    let mut r = curr_row;
    let rstart;
    loop {
        match GraphCalculator::prevuse(oid, r, children_sorted, row_assignments) {
            None => return, // no child before r → nothing to patch
            Some(rs) if rs < prev_row => {
                rstart = rs;
                break;
            }
            Some(rs) => {
                // rstart >= prev_row — keep walking backward
                r = rs;
                if r == 0 {
                    return;
                }
            }
        }
    }

    // If the gap is very large, clamp rstart so we only patch uparrowlen rows
    if rstart + UPARROW_LEN + mingap_len + DOWNARROW_LEN < curr_row {
        let clamped = curr_row.saturating_sub(UPARROW_LEN + 1);
        patch_rows(oid, clamped, prev_row, col_hint, rowidlist, ordertokens);
    } else {
        patch_rows(oid, rstart, prev_row, col_hint, rowidlist, ordertokens);
    }
}

/// Insert `oid` into rows `rstart+1..=prev_row` of the rowidlist where not
/// already present.
fn patch_rows(
    oid: Oid,
    rstart: usize,
    prev_row: usize,
    col_hint: usize,
    rowidlist: &mut [Vec<Option<Oid>>],
    ordertokens: &HashMap<Oid, String>,
) {
    let mut col = col_hint;
    for r in (rstart + 1)..=prev_row {
        if r >= rowidlist.len() {
            break;
        }
        if rowidlist[r].contains(&Some(oid)) {
            continue;
        }
        let active: Vec<Oid> = rowidlist[r].iter().copied().flatten().collect();
        col = GraphCalculator::idcol(&active, oid, ordertokens, col.min(active.len()));
        rowidlist[r].insert(col, Some(oid));
    }
}

/// Precompute, for each `Oid` appearing anywhere in `rowidlist`, the sorted
/// list of `(row, column)` positions where it appears. Rows are scanned in
/// ascending order so each vector is automatically sorted by row, enabling
/// a binary-search range lookup in [`trace_thread`] instead of a per-edge
/// linear scan over every intermediate row.
///
/// Building this index costs O(total cells) once per layout; each subsequent
/// `trace_thread` call drops from O(span × columns_per_row) to
/// O(log₂(positions) + matching_positions).
fn build_thread_index(rowidlist: &[Vec<Option<Oid>>]) -> HashMap<Oid, Vec<(usize, usize)>> {
    let mut idx: HashMap<Oid, Vec<(usize, usize)>> = HashMap::new();
    for (row, idlist) in rowidlist.iter().enumerate() {
        for (col, slot) in idlist.iter().enumerate() {
            if let Some(oid) = slot {
                idx.entry(*oid).or_default().push((row, col));
            }
        }
    }
    idx
}

/// Trace a thread (parent_oid) through the rowidlist from child_row to
/// parent_row, recording (row, col) at each direction change.
///
/// This matches gitk's drawlineseg approach: walk consecutive rows where
/// the thread appears, recording coordinates only when the column delta
/// changes direction. When the thread has been removed from intermediate
/// rows (gitk thread lifecycle), gaps are detected.
///
/// `thread_positions` must be a precomputed index (see [`build_thread_index`])
/// giving, for each oid, its `(row, col)` positions in ascending row order.
///
/// All gaps are found (not just the first), and each contiguous sub-segment
/// is simplified independently to avoid creating incorrect long diagonals
/// across undetected gaps.
///
/// Returns `(waypoints, gap)` where gap is `Some((lower_end_row, upper_start_row))`
/// for the **primary** (largest) gap — the one the renderer should draw
/// arrowheads at. Smaller gaps are preserved as waypoints but without
/// arrowhead rendering.
#[allow(clippy::type_complexity)]
fn trace_thread(
    parent_oid: Oid,
    child_row: usize,
    parent_row: usize,
    _parent_col: usize,
    thread_positions: &HashMap<Oid, Vec<(usize, usize)>>,
) -> (Vec<(usize, usize)>, Option<(usize, usize)>) {
    if parent_row <= child_row + 1 {
        return (Vec::new(), None);
    }

    // Look up the parent's precomputed positions (sorted ascending by row).
    // Collect the slice strictly between child_row and parent_row.
    // Start at the first row > child_row so the first waypoint is below the
    // child. This allows the renderer to apply vertical-first chamfering
    // (vertical from child, then horizontal near parent) instead of forcing
    // a horizontal jog at the child's row.
    let mut path: Vec<(usize, usize)> = Vec::new();
    if let Some(positions) = thread_positions.get(&parent_oid) {
        let start = positions.partition_point(|(r, _)| *r <= child_row);
        for &(r, col) in &positions[start..] {
            if r >= parent_row {
                break;
            }
            path.push((r, col));
        }
    }

    if path.is_empty() {
        return (Vec::new(), None);
    }

    // Detect ALL gaps (row discontinuities) and split into contiguous
    // sub-segments. Each sub-segment is simplified independently so
    // that non-consecutive rows across gaps don't create false
    // collinear collapses.
    let mut gap_boundaries: Vec<(usize, usize)> = Vec::new();
    let mut seg_starts: Vec<usize> = vec![0]; // start index of each segment

    for i in 0..path.len().saturating_sub(1) {
        if path[i + 1].0 > path[i].0 + 1 {
            gap_boundaries.push((path[i].0, path[i + 1].0));
            seg_starts.push(i + 1);
        }
    }

    // Simplify each contiguous sub-segment independently
    let mut waypoints = Vec::new();
    for (si, &start) in seg_starts.iter().enumerate() {
        let end = if si + 1 < seg_starts.len() {
            seg_starts[si + 1]
        } else {
            path.len()
        };
        if start < end {
            waypoints.extend(simplify_collinear(&path[start..end]));
        }
    }

    // Return the largest gap as the primary arrow_gap (for frontend arrowheads)
    let primary_gap = gap_boundaries.into_iter().max_by_key(|(lo, hi)| hi - lo);

    (waypoints, primary_gap)
}

// --- optimize_rows (gitk port) ---

/// Insert `npad` padding slots (None) at position `col` in `rowidlist[row]`.
/// Tries to absorb one existing None from positions after `col` to avoid
/// unbounded list growth (matches gitk's insert_pad behavior).
fn insert_pad(rowidlist: &mut [Vec<Option<Oid>>], row: usize, col: usize, npad: usize) {
    let idlist = &mut rowidlist[row];

    // Try to absorb one existing None from positions > col
    let absorb_idx: Option<usize> = (col + 1..idlist.len()).find(|&i| idlist[i].is_none());

    let mut new_list = Vec::with_capacity(idlist.len() + npad);
    new_list.extend_from_slice(&idlist[..col]);
    new_list.extend(std::iter::repeat_n(None, npad));

    if let Some(ai) = absorb_idx {
        new_list.extend_from_slice(&idlist[col..ai]);
        new_list.extend_from_slice(&idlist[ai + 1..]);
    } else {
        new_list.extend_from_slice(&idlist[col..]);
    }

    *idlist = new_list;
}

/// Optimize the rowidlist by inserting padding to prevent lines from moving
/// more than 1 column between consecutive rows, and to fix zigzags.
///
/// This is a port of gitk's optimize_rows (lines 5871-5993). It fixes the
/// "branch collapse" caused by hint-based idcol — where two siblings share
/// the same column — by inserting padding to separate them.
fn optimize_rows(
    rowidlist: &mut Vec<Vec<Option<Oid>>>,
    displayorder: &[Oid],
    children: &HashMap<Oid, Vec<Oid>>,
) {
    let n = rowidlist.len();
    if n < 3 {
        return;
    }
    let mut rowisopt = vec![false; n];
    optimize_rows_impl(rowidlist, &mut rowisopt, displayorder, children, 1, n);
}

fn optimize_rows_impl(
    rowidlist: &mut Vec<Vec<Option<Oid>>>,
    rowisopt: &mut [bool],
    displayorder: &[Oid],
    children: &HashMap<Oid, Vec<Oid>>,
    start_row: usize,
    end_row: usize,
) {
    let mut row = start_row;
    let mut col;

    while row < end_row {
        if rowisopt[row] {
            row += 1;
            continue;
        }
        if row < 2 {
            rowisopt[row] = true;
            row += 1;
            continue;
        }

        let y0 = row - 1;
        let ym = row - 2;

        if rowidlist[row].is_empty() || rowidlist[y0].is_empty() || rowidlist[ym].is_empty() {
            rowisopt[row] = true;
            row += 1;
            continue;
        }

        let mut haspad = false;
        col = 0;

        while col < rowidlist[row].len() {
            let id_opt = rowidlist[row][col];

            // Line goes straight up?
            if col < rowidlist[y0].len() && rowidlist[y0][col] == id_opt {
                col += 1;
                continue;
            }

            let id = match id_opt {
                Some(o) => o,
                None => {
                    haspad = true;
                    col += 1;
                    continue;
                }
            };

            // Find id in previous row
            let mut x0 = match rowidlist[y0].iter().position(|&x| x == Some(id)) {
                Some(x) => x,
                None => {
                    col += 1;
                    continue;
                }
            };

            let mut z = x0 as i64 - col as i64;

            // Check isarrow
            let mut isarrow = false;
            let mut z0: Option<i64> = None;

            if let Some(xm) = rowidlist[ym].iter().position(|&x| x == Some(id)) {
                z0 = Some(xm as i64 - x0 as i64);
            }

            if z0.is_none() {
                // If commit at y0 is NOT the first child of id, it's an arrow
                let first_child = children.get(&id).and_then(|kids| kids.first());
                if first_child != Some(&displayorder[y0]) {
                    isarrow = true;
                }
            }

            if !isarrow
                && id != displayorder[row]
                && row + 1 < rowidlist.len()
                && !rowidlist[row + 1].contains(&Some(id))
            {
                isarrow = true;
            }

            // Fix lines going left too much
            if z < -1 || (z < 0 && isarrow) {
                let npad = (-1 - z + isarrow as i64) as usize;
                insert_pad(rowidlist, y0, x0, npad);
                if y0 > 0 {
                    rowisopt[y0] = false;
                    optimize_rows_impl(rowidlist, rowisopt, displayorder, children, y0, row);
                }
                // Re-read x0/z/z0 after padding reshuffled y0.
                // gitk does NOT continue here — it falls through so the
                // z0-fill and anti-jig checks below can still fire.
                x0 = match rowidlist[y0].iter().position(|&x| x == Some(id)) {
                    Some(v) => v,
                    None => {
                        col += 1;
                        continue;
                    }
                };
                z = x0 as i64 - col as i64;
                if z0.is_some() {
                    z0 = rowidlist[ym]
                        .iter()
                        .position(|&x| x == Some(id))
                        .map(|xm| xm as i64 - x0 as i64);
                }
            } else if z > 1 || (z > 0 && isarrow) {
                // Fix lines going right too much
                let npad = (z - 1 + isarrow as i64) as usize;
                insert_pad(rowidlist, row, col, npad);
                haspad = true;
                col += npad;
                col += 1;
                continue;
            }

            // When z0 is still unset and this is not an arrow, fill it from
            // the commit drawn at row ym (gitk: "this line links to its first
            // child on row $row-2").
            if z0.is_none() && !isarrow {
                let ym_commit = displayorder[ym];
                if let Some(xc) = rowidlist[ym].iter().position(|&x| x == Some(ym_commit)) {
                    z0 = Some(xc as i64 - x0 as i64);
                }
            }

            // Avoid jigging left then immediately right
            if let Some(z0v) = z0
                && z < 0
                && z0v > 0
            {
                insert_pad(rowidlist, y0, x0, 1);
                if y0 > 0 {
                    rowisopt[y0] = false;
                    optimize_rows_impl(rowidlist, rowisopt, displayorder, children, y0, row);
                }
            }

            col += 1;
        }

        // If no padding was added, insert one pad for visual clarity
        if !haspad {
            let idlist_len = rowidlist[row].len();

            // Find the first column (from right) that doesn't have a line going right.
            // gitk scans right-to-left: if id is not in previdlist, only substitute
            // the first-child's position when id's first child IS displayorder[y0].
            // The final insert only fires when x0 >= 0 (gitk: "if $x0 >= 0").
            let mut found_col: Option<usize> = None;
            let mut found_x0: i64 = -1;
            'scan: for c in (0..idlist_len).rev() {
                let Some(cid) = rowidlist[row][c] else {
                    // Existing pad slot — x0 treated as 0 (>= 0), stop here
                    found_col = Some(c);
                    found_x0 = 0;
                    break;
                };
                let x0_scan: i64 = match rowidlist[y0].iter().position(|&x| x == Some(cid)) {
                    Some(p) => p as i64,
                    None => {
                        // Not in previdlist; only resolve via first-child link
                        let kid = displayorder[y0];
                        let is_first_child = children
                            .get(&cid)
                            .and_then(|kids| kids.first())
                            .is_some_and(|&fc| fc == kid);
                        if is_first_child {
                            match rowidlist[y0].iter().position(|&x| x == Some(kid)) {
                                Some(kc) => kc as i64,
                                None => continue 'scan,
                            }
                        } else {
                            // x0 effectively -1 (< 0) — x0 <= c → break,
                            // but do NOT insert (x0 < 0 fails the x0 >= 0 guard)
                            found_col = Some(c);
                            found_x0 = -1;
                            break 'scan;
                        }
                    }
                };
                if x0_scan <= c as i64 {
                    found_col = Some(c);
                    found_x0 = x0_scan;
                    break 'scan;
                }
            }

            // gitk: if {$x0 >= 0 && [incr col] < [llength $idlist]}
            if found_x0 >= 0
                && let Some(c) = found_col
                && c + 1 < idlist_len
            {
                rowidlist[row].insert(c + 1, None);
            }
        }
        rowisopt[row] = true;
        row += 1;
    }
}

/// Extract each commit's final column from the optimized rowidlist.
fn extract_columns(rowidlist: &[Vec<Option<Oid>>], displayorder: &[Oid]) -> HashMap<Oid, usize> {
    let mut columns = HashMap::new();
    for (row, idlist) in rowidlist.iter().enumerate() {
        if row >= displayorder.len() {
            break;
        }
        let oid = displayorder[row];
        if let Some(col) = idlist.iter().position(|&x| x == Some(oid)) {
            columns.insert(oid, col);
        }
    }
    columns
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
        // optimize_rows should separate the two branches
        let columns: HashSet<usize> = layout.nodes.iter().map(|n| n.column).collect();
        assert!(
            columns.len() >= 2,
            "branching should use at least 2 columns after optimize_rows, got {:?}: {:?}",
            columns,
            layout
                .nodes
                .iter()
                .map(|n| (n.oid.short_hex(), n.column))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn row_max_column_length_matches_total_rows() {
        let c1 = make_commit(1, vec![], "a");
        let c2 = make_commit(2, vec![1], "b");
        let c3 = make_commit(3, vec![1], "c");
        let c4 = make_commit(4, vec![2, 3], "merge");
        let commits = vec![c4, c3, c2, c1];
        let calc =
            GraphCalculator::new(commits, HashMap::new(), Vec::new(), GraphOptions::default());
        let layout = calc.calculate_layout();
        assert_eq!(
            layout.row_max_column.len(),
            layout.total_rows,
            "row_max_column length should match total_rows"
        );
    }

    #[test]
    fn row_max_column_covers_node_columns() {
        let c1 = make_commit(1, vec![], "root");
        let c2 = make_commit(2, vec![1], "on main");
        let c3 = make_commit(3, vec![1], "on branch");
        let c4 = make_commit(4, vec![2, 3], "merge");
        let commits = vec![c4, c3, c2, c1];
        let calc =
            GraphCalculator::new(commits, HashMap::new(), Vec::new(), GraphOptions::default());
        let layout = calc.calculate_layout();
        for node in &layout.nodes {
            assert!(
                layout.row_max_column[node.row] > node.column,
                "row_max_column[{}] = {} should be > {} (node column)",
                node.row,
                layout.row_max_column[node.row],
                node.column
            );
        }
    }

    #[test]
    fn row_max_column_empty_for_empty_layout() {
        let calc = GraphCalculator::new(
            Vec::new(),
            HashMap::new(),
            Vec::new(),
            GraphOptions::default(),
        );
        let layout = calc.calculate_layout();
        assert!(layout.row_max_column.is_empty());
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
                other_oid.is_some_and(|n| n.oid == make_oid(2))
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
    fn viewport_retains_spanning_edges() {
        // Topology that produces a long-spanning edge:
        //   c1 (root, row 4)
        //   c2 (root, row 3)
        //   c3 (root, row 2)
        //   c4 (child of c1, row 1)
        //   c5 (merge of c2+c4, row 0)
        // Edge c5→c2 spans from row 0 to row 3 — both outside viewport [1,3).
        let c1 = make_commit(1, vec![], "root1");
        let c2 = make_commit(2, vec![], "root2");
        let c3 = make_commit(3, vec![], "root3");
        let c4 = make_commit(4, vec![1], "branch from c1");
        let c5 = make_commit(5, vec![2, 4], "merge c2+c4");
        let commits = vec![c5, c4, c3, c2, c1];
        let calc =
            GraphCalculator::new(commits, HashMap::new(), Vec::new(), GraphOptions::default());
        let layout = calc.calculate_layout();

        let vp = layout.from_visible_range(1, 3);
        assert!(
            vp.edges.iter().any(|e| e.from_row < 1 && e.to_row >= 3),
            "spanning edge (both endpoints outside viewport) should be retained"
        );
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
            !colors.is_empty(),
            "color-by-author should assign colors to nodes"
        );
    }

    #[test]
    fn merge_child_has_no_spurious_edge_to_merge_side() {
        // Topology from bug report: 5701a67 (parents: 59295b8, ed13a61),
        // 59295b8 (parents: 2f38767, 5cb5908).
        // 59295b8 must have exactly 2 out-edges (to its own parents),
        // not a spurious third edge to ed13a61.
        //
        // Layout:
        //   row 0: 5701a67 (merge of 59295b8 + ed13a61)
        //   row 1: 59295b8 (merge of 2f38767 + 5cb5908)
        //   row 2: ed13a61
        //   row 3: 5cb5908
        //   row 4: 2f38767
        //   row 5-7: roots
        let roots = vec![
            make_commit(1, vec![], "root for 2f38767"),
            make_commit(2, vec![], "root for 5cb5908"),
            make_commit(3, vec![], "root for ed13a61"),
        ];
        let c4 = make_commit(4, vec![1], "2f38767");
        let c5 = make_commit(5, vec![2], "5cb5908");
        let c6 = make_commit(6, vec![3], "ed13a61");
        let c7 = make_commit(7, vec![4, 5], "59295b8");
        let c8 = make_commit(8, vec![7, 6], "5701a67");
        let commits = vec![c8, c7, c6, c5, c4];
        let mut all = roots;
        all.extend(commits);
        let calc = GraphCalculator::new(all, HashMap::new(), Vec::new(), GraphOptions::default());
        let layout = calc.calculate_layout();

        let commit_59295b8 = make_oid(7);
        let commit_ed13a61 = make_oid(6);

        // Find 59295b8's node
        let n7 = layout
            .nodes
            .iter()
            .find(|n| n.oid == commit_59295b8)
            .expect("59295b8 must be in layout");

        // Find all edges from 59295b8
        let edges_from_59295b8: Vec<&Edge> = layout
            .edges
            .iter()
            .filter(|e| e.from_row == n7.row && e.from_col == n7.column)
            .collect();

        // 59295b8 must have exactly 2 out-edges (to 2f38767 and 5cb5908)
        assert_eq!(
            edges_from_59295b8.len(),
            2,
            "59295b8 must have exactly 2 out-edges, got {}",
            edges_from_59295b8.len()
        );

        // Verify no edge from 59295b8 to ed13a61
        for edge in &edges_from_59295b8 {
            let to_node = layout
                .nodes
                .iter()
                .find(|n| n.row == edge.to_row && n.column == edge.to_col);
            if let Some(to_node) = to_node {
                assert_ne!(
                    to_node.oid, commit_ed13a61,
                    "59295b8 must not have an edge to ed13a61"
                );
            }
        }

        // Verify 5701a67 has edges to both 59295b8 and ed13a61
        let n8 = layout
            .nodes
            .iter()
            .find(|n| n.oid == make_oid(8))
            .expect("5701a67 must be in layout");
        let edges_from_5701a67: Vec<&Edge> = layout
            .edges
            .iter()
            .filter(|e| e.from_row == n8.row && e.from_col == n8.column)
            .collect();
        assert_eq!(
            edges_from_5701a67.len(),
            2,
            "5701a67 must have exactly 2 out-edges"
        );

        // Verify 59295b8's two parents (2f38767, 5cb5908) have distinct columns
        let commit_2f38767 = make_oid(4);
        let commit_5cb5908 = make_oid(5);
        let n4 = layout
            .nodes
            .iter()
            .find(|n| n.oid == commit_2f38767)
            .expect("2f38767 must be in layout");
        let n5 = layout
            .nodes
            .iter()
            .find(|n| n.oid == commit_5cb5908)
            .expect("5cb5908 must be in layout");
        assert_ne!(
            n4.column, n5.column,
            "59295b8's two parents must be in different columns, got {} and {}",
            n4.column, n5.column
        );
    }

    #[test]
    fn property_merge_parents_have_distinct_columns() {
        let test_cases = vec![
            vec![
                make_commit(1, vec![], "root1"),
                make_commit(2, vec![], "root2"),
                make_commit(3, vec![], "root3"),
                make_commit(4, vec![1], "2f38767"),
                make_commit(5, vec![2], "5cb5908"),
                make_commit(6, vec![3], "ed13a61"),
                make_commit(7, vec![4, 5], "59295b8"),
                make_commit(8, vec![7, 6], "5701a67"),
            ],
            vec![
                make_commit(1, vec![], "root"),
                make_commit(2, vec![1], "main"),
                make_commit(3, vec![1], "branch"),
                make_commit(4, vec![2, 3], "merge"),
            ],
            vec![
                make_commit(1, vec![], "root"),
                make_commit(2, vec![1], "a"),
                make_commit(3, vec![1], "b"),
                make_commit(4, vec![2, 3], "merge1"),
                make_commit(5, vec![4], "c"),
                make_commit(6, vec![1], "d"),
                make_commit(7, vec![5, 6], "merge2"),
            ],
        ];

        for commits in &test_cases {
            let calc = GraphCalculator::new(
                commits.clone(),
                HashMap::new(),
                Vec::new(),
                GraphOptions::default(),
            );
            let layout = calc.calculate_layout();

            let node_by_oid: HashMap<Oid, &NodePosition> =
                layout.nodes.iter().map(|n| (n.oid, n)).collect();

            for commit in &layout.nodes {
                let orig = match commits.iter().find(|c| c.oid == commit.oid) {
                    Some(c) => c,
                    None => continue,
                };
                if orig.parent_oids.len() < 2 {
                    continue;
                }
                let first_parent = match node_by_oid.get(&orig.parent_oids[0]) {
                    Some(n) => n,
                    None => continue,
                };
                for parent_oid in orig.parent_oids[1..].iter() {
                    if let Some(other_parent) = node_by_oid.get(parent_oid) {
                        assert_ne!(
                            first_parent.column, other_parent.column,
                            "Merge commit {} has two parents in column {}",
                            orig.short_oid, first_parent.column,
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn property_column_count_reasonable_for_complex_merge() {
        let commits = vec![
            make_commit(1, vec![], "root1"),
            make_commit(2, vec![], "root2"),
            make_commit(3, vec![], "root3"),
            make_commit(4, vec![1], "2f38767"),
            make_commit(5, vec![2], "5cb5908"),
            make_commit(6, vec![3], "ed13a61"),
            make_commit(7, vec![4, 5], "59295b8"),
            make_commit(8, vec![7, 6], "5701a67"),
        ];
        let calc =
            GraphCalculator::new(commits, HashMap::new(), Vec::new(), GraphOptions::default());
        let layout = calc.calculate_layout();

        // This topology has 3 independent branches merging into one mainline.
        // gitk-style layout should use at most 3 columns (mainline + 2 branch lanes).
        assert!(
            layout.total_columns <= 3,
            "Complex merge should use at most 3 columns, got {}",
            layout.total_columns
        );
    }

    #[test]
    fn property_no_pass_through_with_parallel_branches() {
        // Three parallel branches fork from root and merge at the end.
        // Each branch has 3 commits, creating long-spanning lane edges.
        // The layout must route them through distinct columns / waypoints
        // so no same-column edge passes through an unrelated node.
        //
        // root(1) ─→ main(2) → main(5) → main(8) ─→ mergeAB(11) → mergeFinal(12)
        //         ├→ branchA(3) → branchA(6) → branchA(9) ─┘
        //         └→ branchB(4) → branchB(7) → branchB(10) ───────────────────┘
        let commits = vec![
            make_commit(1, vec![], "root"),
            make_commit(2, vec![1], "main1"),
            make_commit(3, vec![1], "branchA1"),
            make_commit(4, vec![1], "branchB1"),
            make_commit(5, vec![2], "main2"),
            make_commit(6, vec![3], "branchA2"),
            make_commit(7, vec![4], "branchB2"),
            make_commit(8, vec![5], "main3"),
            make_commit(9, vec![6], "branchA3"),
            make_commit(10, vec![7], "branchB3"),
            make_commit(11, vec![8, 9], "mergeAB"),
            make_commit(12, vec![11, 10], "mergeFinal"),
        ];
        let calc =
            GraphCalculator::new(commits, HashMap::new(), Vec::new(), GraphOptions::default());
        let layout = calc.calculate_layout();

        let errors = layout.verify();
        assert!(
            errors.is_empty(),
            "verify() found {} pass-through error(s):\n{}",
            errors.len(),
            errors
                .iter()
                .take(5)
                .cloned()
                .collect::<Vec<_>>()
                .join("\n")
        );
    }

    /// Verify the parent-preservation invariant: every non-merge commit that
    /// has ≥1 parent when merges are shown must still have ≥1 parent edge
    /// when merges are hidden.
    fn assert_parents_preserved(show_layout: &GraphLayout, hide_layout: &GraphLayout, label: &str) {
        let show_oid_to_node: HashMap<Oid, &NodePosition> =
            show_layout.nodes.iter().map(|n| (n.oid, n)).collect();

        for node in &hide_layout.nodes {
            let show_node = match show_oid_to_node.get(&node.oid) {
                Some(n) => *n,
                None => continue,
            };

            let had_parents = show_layout
                .edges
                .iter()
                .any(|e| e.from_row == show_node.row && e.from_col == show_node.column);

            if !had_parents {
                continue;
            }

            let has_parents = hide_layout
                .edges
                .iter()
                .any(|e| e.from_row == node.row && e.from_col == node.column);

            assert!(
                has_parents,
                "{label}: node {} had parents in show-layout but has none in hide-layout",
                node.oid.short_hex(),
            );
        }
    }

    #[test]
    fn property_hide_merges_preserves_parents_simple() {
        let c1 = make_commit(1, vec![], "root A");
        let c2 = make_commit(2, vec![], "root B");
        let m = make_commit(3, vec![1, 2], "merge");
        let c4 = make_commit(4, vec![3], "after merge");
        let commits = vec![c4, m, c2, c1];

        let show_calc = GraphCalculator::new(
            commits.clone(),
            HashMap::new(),
            Vec::new(),
            GraphOptions::default(),
        );
        let hide_calc = GraphCalculator::new(
            commits,
            HashMap::new(),
            Vec::new(),
            GraphOptions {
                hide_merges: true,
                ..GraphOptions::default()
            },
        );

        let show_layout = show_calc.calculate_layout();
        let hide_layout = hide_calc.calculate_layout();

        assert_parents_preserved(&show_layout, &hide_layout, "simple merge");
    }

    #[test]
    fn property_hide_merges_preserves_parents_chained() {
        let c1 = make_commit(1, vec![], "root A");
        let c2 = make_commit(2, vec![], "root B");
        let c3 = make_commit(3, vec![], "root C");
        let m1 = make_commit(4, vec![1, 2], "inner merge");
        let m2 = make_commit(5, vec![4, 3], "outer merge");
        let c6 = make_commit(6, vec![5], "after outer merge");
        let commits = vec![c6, m2, m1, c3, c2, c1];

        let show_calc = GraphCalculator::new(
            commits.clone(),
            HashMap::new(),
            Vec::new(),
            GraphOptions::default(),
        );
        let hide_calc = GraphCalculator::new(
            commits,
            HashMap::new(),
            Vec::new(),
            GraphOptions {
                hide_merges: true,
                ..GraphOptions::default()
            },
        );

        let show_layout = show_calc.calculate_layout();
        let hide_layout = hide_calc.calculate_layout();

        assert_parents_preserved(&show_layout, &hide_layout, "chained merges");
    }

    #[test]
    fn property_hide_merges_preserves_parents_multiple_children() {
        let c1 = make_commit(1, vec![], "root A");
        let c2 = make_commit(2, vec![], "root B");
        let m = make_commit(3, vec![1, 2], "merge");
        let c4 = make_commit(4, vec![3], "child 1");
        let c5 = make_commit(5, vec![3], "child 2");
        let commits = vec![c5, c4, m, c2, c1];

        let show_calc = GraphCalculator::new(
            commits.clone(),
            HashMap::new(),
            Vec::new(),
            GraphOptions::default(),
        );
        let hide_calc = GraphCalculator::new(
            commits,
            HashMap::new(),
            Vec::new(),
            GraphOptions {
                hide_merges: true,
                ..GraphOptions::default()
            },
        );

        let show_layout = show_calc.calculate_layout();
        let hide_layout = hide_calc.calculate_layout();

        assert_parents_preserved(&show_layout, &hide_layout, "multiple children");
    }

    #[test]
    fn property_hide_merges_preserves_parents_only_merge_parent() {
        let c1 = make_commit(1, vec![], "root A");
        let c2 = make_commit(2, vec![], "root B");
        let m = make_commit(3, vec![1, 2], "merge");
        let c4 = make_commit(4, vec![3], "only merge parent");
        let c5 = make_commit(5, vec![4], "grandchild");
        let commits = vec![c5, c4, m, c2, c1];

        let show_calc = GraphCalculator::new(
            commits.clone(),
            HashMap::new(),
            Vec::new(),
            GraphOptions::default(),
        );
        let hide_calc = GraphCalculator::new(
            commits,
            HashMap::new(),
            Vec::new(),
            GraphOptions {
                hide_merges: true,
                ..GraphOptions::default()
            },
        );

        let show_layout = show_calc.calculate_layout();
        let hide_layout = hide_calc.calculate_layout();

        assert_parents_preserved(&show_layout, &hide_layout, "only merge parent");
    }

    #[test]
    fn property_hide_merges_preserves_parents_merge_of_merge() {
        // c1 → c2 → m1 ← c3
        //              ↓
        //      m2 ← c4
        //       ↓
        //      c5
        // m1 is a merge, m2 is a merge whose first parent is m1
        let c1 = make_commit(1, vec![], "root");
        let c2 = make_commit(2, vec![1], "main line");
        let c3 = make_commit(3, vec![1], "branch");
        let m1 = make_commit(4, vec![2, 3], "inner merge");
        let c4 = make_commit(5, vec![1], "feature");
        let m2 = make_commit(6, vec![4, 5], "outer merge");
        let c7 = make_commit(7, vec![6], "after outer merge");
        let commits = vec![c7, m2, m1, c4, c3, c2, c1];

        let show_calc = GraphCalculator::new(
            commits.clone(),
            HashMap::new(),
            Vec::new(),
            GraphOptions::default(),
        );
        let hide_calc = GraphCalculator::new(
            commits,
            HashMap::new(),
            Vec::new(),
            GraphOptions {
                hide_merges: true,
                ..GraphOptions::default()
            },
        );

        let show_layout = show_calc.calculate_layout();
        let hide_layout = hide_calc.calculate_layout();

        assert_parents_preserved(&show_layout, &hide_layout, "merge of merge");
    }

    #[test]
    fn compaction_reduces_column_count() {
        // Two non-overlapping branches should share a column after compaction.
        // Topology: root → a1 → merge1 → b1 → merge2
        //                  f1 →┘         f2 →┘
        // f1 and f2 are active at different times (no row overlap).
        let c1 = make_commit(1, vec![], "root");
        let c2 = make_commit(2, vec![1], "a1");
        let c3 = make_commit(3, vec![1], "f1");
        let c4 = make_commit(4, vec![2, 3], "merge1");
        let c5 = make_commit(5, vec![4], "b1");
        let c6 = make_commit(6, vec![4], "f2");
        let c7 = make_commit(7, vec![5, 6], "merge2");
        let commits = vec![c7, c6, c5, c4, c3, c2, c1];

        let calc =
            GraphCalculator::new(commits, HashMap::new(), Vec::new(), GraphOptions::default());
        let layout = calc.calculate_layout();

        // With compaction, sequential feature branches should share a column.
        // Without compaction this would need 3 columns (main + 2 features).
        // With compaction, f1 and f2 can share since they don't overlap.
        assert!(
            layout.total_columns <= 2,
            "Sequential branches should compact to ≤2 columns (main + shared feature), got {}",
            layout.total_columns
        );
    }

    // --- Phase 1-4: gitk algorithm alignment tests ---

    #[test]
    fn prevuse_finds_most_recent_child() {
        let oid = make_oid(1);
        let child_a = make_oid(2);
        let child_b = make_oid(3);

        let mut children: HashMap<Oid, Vec<Oid>> = HashMap::new();
        children.insert(oid, vec![child_a, child_b]);

        let mut row_assignments: HashMap<Oid, usize> = HashMap::new();
        row_assignments.insert(child_a, 0);
        row_assignments.insert(child_b, 5);
        row_assignments.insert(oid, 10);

        assert_eq!(
            GraphCalculator::prevuse(oid, 6, &children, &row_assignments),
            Some(5),
            "prevuse(6) should find child_b at row 5"
        );
        assert_eq!(
            GraphCalculator::prevuse(oid, 5, &children, &row_assignments),
            Some(0),
            "prevuse(5) should find child_a at row 0"
        );
        assert_eq!(
            GraphCalculator::prevuse(oid, 0, &children, &row_assignments),
            None,
            "prevuse(0) should return None"
        );
    }

    #[test]
    fn thread_removal_creates_arrow_gap() {
        // Long branch spanning >110 rows triggers thread removal + arrow_gap.
        // root(1) → main(2) → ... → main(120) → merge(122)
        //    └→ branch(121) ──────────────────────┘
        // Edge branch(121)→root(1) spans ~120 rows.
        let mut commits = vec![make_commit(1, vec![], "root")];
        for i in 2..=120u8 {
            commits.push(make_commit(i, vec![i - 1], "main"));
        }
        commits.push(make_commit(121, vec![1], "branch"));
        commits.push(make_commit(122, vec![120, 121], "merge"));

        let calc =
            GraphCalculator::new(commits, HashMap::new(), Vec::new(), GraphOptions::default());
        let layout = calc.calculate_layout();

        let branch_node = layout
            .nodes
            .iter()
            .find(|n| n.oid == make_oid(121))
            .expect("branch must be in layout");
        let root_node = layout
            .nodes
            .iter()
            .find(|n| n.oid == make_oid(1))
            .expect("root must be in layout");

        let branch_edge = layout
            .edges
            .iter()
            .find(|e| e.from_row == branch_node.row && e.to_row == root_node.row)
            .expect("should have edge from branch to root");

        assert!(
            branch_edge.arrow_gap.is_some(),
            "long-spanning branch edge should have arrow_gap from thread removal"
        );

        let errors = layout.verify();
        assert!(errors.is_empty(), "verify() should find no errors");
    }

    #[test]
    fn short_edge_has_no_arrow_gap() {
        // Short branch (<110 rows) should NOT have arrow_gap.
        let c1 = make_commit(1, vec![], "root");
        let c2 = make_commit(2, vec![1], "main");
        let c3 = make_commit(3, vec![1], "branch");
        let c4 = make_commit(4, vec![2, 3], "merge");
        let commits = vec![c4, c3, c2, c1];

        let calc =
            GraphCalculator::new(commits, HashMap::new(), Vec::new(), GraphOptions::default());
        let layout = calc.calculate_layout();

        for edge in &layout.edges {
            assert!(
                edge.arrow_gap.is_none(),
                "short edges should not have arrow_gap"
            );
        }
    }

    #[test]
    fn linear_chain_inherits_color() {
        // root → a → b → merge
        //         └→ branch →┘
        // Linear chain root→a→b should share color (each has 1 child with 1 parent).
        let c1 = make_commit(1, vec![], "root");
        let c2 = make_commit(2, vec![1], "a");
        let c3 = make_commit(3, vec![2], "b");
        let c4 = make_commit(4, vec![1], "branch");
        let c5 = make_commit(5, vec![4], "branch2");
        let c6 = make_commit(6, vec![3, 5], "merge");
        let commits = vec![c6, c5, c4, c3, c2, c1];

        let calc =
            GraphCalculator::new(commits, HashMap::new(), Vec::new(), GraphOptions::default());
        let layout = calc.calculate_layout();

        let n2 = layout.nodes.iter().find(|n| n.oid == make_oid(2)).unwrap();
        let n3 = layout.nodes.iter().find(|n| n.oid == make_oid(3)).unwrap();

        assert_eq!(
            n2.color, n3.color,
            "linear chain (a→b, each has 1 child/parent) should share color"
        );
    }

    #[test]
    fn edge_color_uses_parent_color() {
        let c1 = make_commit(1, vec![], "root");
        let c2 = make_commit(2, vec![1], "child");
        let commits = vec![c2, c1];

        let calc =
            GraphCalculator::new(commits, HashMap::new(), Vec::new(), GraphOptions::default());
        let layout = calc.calculate_layout();

        let n1 = layout.nodes.iter().find(|n| n.oid == make_oid(1)).unwrap();
        let edge = layout
            .edges
            .iter()
            .find(|e| e.to_row == n1.row)
            .expect("should have edge to root");

        assert_eq!(
            edge.color, n1.color,
            "edge should use parent's color, not child's"
        );
    }

    #[test]
    fn arrow_gap_threshold_controls_removal() {
        // Same topology as thread_removal test but with high threshold.
        // With threshold=200, threads spanning 120 rows should NOT be removed.
        let mut commits = vec![make_commit(1, vec![], "root")];
        for i in 2..=120u8 {
            commits.push(make_commit(i, vec![i - 1], "main"));
        }
        commits.push(make_commit(121, vec![1], "branch"));
        commits.push(make_commit(122, vec![120, 121], "merge"));

        let options = GraphOptions {
            arrow_gap_threshold: 200,
            ..GraphOptions::default()
        };
        let calc = GraphCalculator::new(commits, HashMap::new(), Vec::new(), options);
        let layout = calc.calculate_layout();

        let branch_node = layout
            .nodes
            .iter()
            .find(|n| n.oid == make_oid(121))
            .unwrap();
        let root_node = layout.nodes.iter().find(|n| n.oid == make_oid(1)).unwrap();

        let branch_edge = layout
            .edges
            .iter()
            .find(|e| e.from_row == branch_node.row && e.to_row == root_node.row)
            .expect("should have edge from branch to root");

        assert!(
            branch_edge.arrow_gap.is_none(),
            "with threshold=200, 120-row edge should NOT be removed"
        );
    }

    // --- simplify_collinear regression tests ---

    #[test]
    fn simplify_collinear_consecutive_rows() {
        // Same column → all collapsed
        let path = vec![(0, 3), (1, 3), (2, 3), (3, 3)];
        assert_eq!(simplify_collinear(&path), vec![(0, 3), (3, 3)]);

        // Uniform slope 1 (consecutive rows) → collapsed to endpoints
        let path = vec![(0, 3), (1, 4), (2, 5), (3, 6)];
        assert_eq!(simplify_collinear(&path), vec![(0, 3), (3, 6)]);

        // Direction change → intermediate point preserved
        let path = vec![(0, 3), (1, 4), (2, 3)];
        assert_eq!(simplify_collinear(&path), vec![(0, 3), (1, 4), (2, 3)]);

        // Slope change (flat then diagonal)
        let path = vec![(0, 3), (1, 3), (2, 4), (3, 5)];
        assert_eq!(simplify_collinear(&path), vec![(0, 3), (1, 3), (3, 5)]);
    }

    #[test]
    fn simplify_collinear_nonconsecutive_rows() {
        // Non-consecutive rows with same column delta but DIFFERENT slopes.
        // Old buggy code compared only column deltas and would collapse (0,3)
        // and (5,8) incorrectly. Cross-multiplication preserves the turn.
        //
        // (0,3) → (1,4): slope = 1/1 = 1
        // (1,4) → (5,8): slope = 4/4 = 1  → same slope, collinear, collapse OK
        let path = vec![(0, 3), (1, 4), (5, 8)];
        assert_eq!(simplify_collinear(&path), vec![(0, 3), (5, 8)]);

        // (0,3) → (1,5): slope = 2/1 = 2
        // (1,5) → (5,6): slope = 1/4     → different slopes, preserve turn
        let path = vec![(0, 3), (1, 5), (5, 6)];
        assert_eq!(simplify_collinear(&path), vec![(0, 3), (1, 5), (5, 6)]);

        // (0,3) → (1,4): slope = 1
        // (1,4) → (5,5): slope = 1/4     → different, preserve
        let path = vec![(0, 3), (1, 4), (5, 5)];
        assert_eq!(simplify_collinear(&path), vec![(0, 3), (1, 4), (5, 5)]);
    }

    #[test]
    fn simplify_collinear_single_and_empty() {
        assert!(simplify_collinear(&[]).is_empty());
        assert_eq!(simplify_collinear(&[(5, 3)]), vec![(5, 3)]);
        assert_eq!(simplify_collinear(&[(5, 3), (6, 4)]), vec![(5, 3), (6, 4)]);
    }

    // --- trace_thread multi-gap regression test ---

    #[test]
    fn trace_thread_detects_largest_gap() {
        // Build a rowidlist where parent_oid appears in two blocks:
        //   rows 0-2 at column 0, rows 8-10 at column 0
        // Gaps: rows 3-7 (5-row gap)
        let parent = make_oid(1);
        let other = make_oid(2);

        let mut rowidlist: Vec<Vec<Option<Oid>>> = Vec::new();
        for r in 0..=10 {
            let mut row = vec![Some(parent)];
            if (3..=7).contains(&r) {
                // Gap region: parent is absent, other thread present
                row = vec![Some(other)];
            }
            rowidlist.push(row);
        }

        let idx = build_thread_index(&rowidlist);
        let (waypoints, gap) = trace_thread(parent, 0, 10, 0, &idx);

        assert!(gap.is_some(), "should detect the gap");
        let (lo, hi) = gap.unwrap();
        assert_eq!(lo, 2, "gap lower end is last row of segment 1");
        assert_eq!(hi, 8, "gap upper start is first row of segment 2");

        // Waypoints should include entries from both segments
        assert!(
            !waypoints.is_empty(),
            "should have waypoints from both segments"
        );
        let rows: Vec<usize> = waypoints.iter().map(|(r, _)| *r).collect();
        assert!(
            rows.iter().all(|&r| r <= 2 || r >= 8),
            "no waypoints in the gap region"
        );
    }

    #[test]
    fn trace_thread_multiple_gaps_simplifies_independently() {
        // rowidlist where parent appears in THREE blocks:
        //   rows 0-2: col 0
        //   rows 5-6: col 0   (small gap of 2)
        //   rows 9-10: col 0  (small gap of 2)
        // The largest gap (rows 3-4, size 2) should be returned.
        // Actually all gaps are the same size; first found wins.
        let parent = make_oid(1);
        let other = make_oid(2);

        let mut rowidlist: Vec<Vec<Option<Oid>>> = Vec::new();
        for r in 0..=10 {
            let present = r <= 2 || (5..=6).contains(&r) || r >= 9;
            if present {
                rowidlist.push(vec![Some(parent)]);
            } else {
                rowidlist.push(vec![Some(other)]);
            }
        }

        let idx = build_thread_index(&rowidlist);
        let (waypoints, gap) = trace_thread(parent, 0, 10, 0, &idx);

        assert!(gap.is_some(), "should detect at least one gap");

        // Waypoints should NOT include any rows in gap regions (3-4, 7-8)
        let gap_rows: Vec<usize> = waypoints
            .iter()
            .map(|(r, _)| *r)
            .filter(|&r| (3..=4).contains(&r) || (7..=8).contains(&r))
            .collect();
        assert!(
            gap_rows.is_empty(),
            "no waypoints in gap regions, got {:?}",
            gap_rows
        );
    }

    // --- Visual correctness: f421e8d multi-branch topology ---
    // Real topology from this repo: f421e8d has 4 children
    // (6ce557e=mainline, 4e14921, 7dae642, a344ea2).
    // Each branch child has a chain of commits above it.

    fn make_f421e8d_topology() -> Vec<CommitInfo> {
        // OID assignments (higher = newer = appears first in display order):
        // 14 = 6ce557e (mainline child of f421e8d)
        // 13-10 = 279ad6c → 4f335b4 → b54e36f → 52f6ce1 (chain above 4e14921)
        // 9  = 4e14921 (branch child of f421e8d)
        // 8-5 = ff76046 → f2e47d6 → 562c79b → 55e3003 (chain above 7dae642)
        // 4  = 7dae642 (branch child of f421e8d)
        // 3  = a344ea2 (branch child of f421e8d)
        // 2  = f421e8d (common parent)
        // 1  = root (f421e8d's parent)
        vec![
            make_commit(1, vec![], "root"),
            make_commit(2, vec![1], "f421e8d"),
            make_commit(3, vec![2], "a344ea2"),
            make_commit(4, vec![2], "7dae642"),
            make_commit(5, vec![4], "55e3003"),
            make_commit(6, vec![5], "562c79b"),
            make_commit(7, vec![6], "f2e47d6"),
            make_commit(8, vec![7], "ff76046"),
            make_commit(9, vec![2], "4e14921"),
            make_commit(10, vec![9], "52f6ce1"),
            make_commit(11, vec![10], "b54e36f"),
            make_commit(12, vec![11], "4f335b4"),
            make_commit(13, vec![12], "279ad6c"),
            make_commit(14, vec![2], "6ce557e"),
        ]
    }

    /// Build the full pixel-ish path of an edge: (from_row, from_col) →
    /// waypoints → (to_row, to_col). For edges with arrow_gap, only the
    /// non-gap segments are included.
    fn edge_full_path(edge: &Edge) -> Vec<Vec<(usize, usize)>> {
        if let Some((gap_lo, gap_hi)) = edge.arrow_gap {
            let mut seg1 = vec![(edge.from_row, edge.from_col)];
            for wp in &edge.waypoints {
                if wp.0 <= gap_lo {
                    seg1.push(*wp);
                }
            }
            let mut seg2: Vec<(usize, usize)> = Vec::new();
            for wp in &edge.waypoints {
                if wp.0 >= gap_hi {
                    seg2.push(*wp);
                }
            }
            seg2.push((edge.to_row, edge.to_col));
            vec![seg1, seg2]
        } else {
            let mut path = vec![(edge.from_row, edge.from_col)];
            path.extend(edge.waypoints.iter().copied());
            path.push((edge.to_row, edge.to_col));
            vec![path]
        }
    }

    /// Check if any edge's RENDERED path crosses an unrelated node.
    ///
    /// For edges WITH waypoints: traces the polyline through waypoints.
    /// For edges WITHOUT waypoints (cross-column): models the chamfer path
    /// (horizontal-first: horizontal at child's row, then vertical at
    /// parent's column). Checks that the vertical segment at parent_col
    /// doesn't pass through any node, and the horizontal segment at
    /// child's row doesn't pass through any node.
    fn find_rendered_crossings(layout: &GraphLayout) -> Vec<String> {
        let mut errors = Vec::new();
        let node_at: HashMap<(usize, usize), String> = layout
            .nodes
            .iter()
            .map(|n| ((n.row, n.column), n.oid.short_hex().to_string()))
            .collect();

        let efrom = |layout: &GraphLayout, edge: &Edge| {
            layout
                .nodes
                .iter()
                .find(|n| n.row == edge.from_row)
                .map(|n| n.oid.short_hex().to_string())
                .unwrap_or_default()
        };
        let eto = |layout: &GraphLayout, edge: &Edge| {
            layout
                .nodes
                .iter()
                .find(|n| n.row == edge.to_row)
                .map(|n| n.oid.short_hex().to_string())
                .unwrap_or_default()
        };

        for edge in &layout.edges {
            if edge.from_col == edge.to_col {
                // Same-column edge: check vertical pass-through
                let (min_r, max_r) = (
                    edge.from_row.min(edge.to_row),
                    edge.from_row.max(edge.to_row),
                );
                for nr in (min_r + 1)..max_r {
                    if let Some(name) = node_at.get(&(nr, edge.from_col)) {
                        errors.push(format!(
                            "edge {}→{} same-col passes through node {} at ({},{})",
                            efrom(layout, edge),
                            eto(layout, edge),
                            name,
                            nr,
                            edge.from_col
                        ));
                    }
                }
                continue;
            }

            // Cross-column edge
            if !edge.waypoints.is_empty() || edge.arrow_gap.is_some() {
                // Has waypoints: trace polyline, check diagonal segments
                for path in edge_full_path(edge) {
                    for window in path.windows(2) {
                        let (r1, c1) = window[0];
                        let (r2, c2) = window[1];
                        if r1 == r2 || c1 == c2 {
                            continue;
                        }
                        let dr = (r2 as i64 - r1 as i64).abs();
                        let dc = c2 as i64 - c1 as i64;
                        let r_lo = r1.min(r2);
                        let r_hi = r1.max(r2);
                        for nr in (r_lo + 1)..r_hi {
                            let frac = (nr as i64 - r1 as i64) as f64 / dr as f64;
                            let c_frac = c1 as f64 + dc as f64 * frac;
                            let c_round = c_frac.round() as usize;
                            if let Some(name) = node_at.get(&(nr, c_round))
                                && name != &efrom(layout, edge)
                                && name != &eto(layout, edge)
                            {
                                errors.push(format!(
                                    "edge {}→{} wp-segment ({},{})→({},{}) crosses node {} at ({},{})",
                                    efrom(layout, edge), eto(layout, edge),
                                    r1, c1, r2, c2, name, nr, c_round
                                ));
                            }
                        }
                    }
                }
            } else if edge.edge_type == EdgeType::Merge {
                // Merge edge with empty waypoints: frontend renders horizontal-first
                // chamfer (horizontal at merge row, then diagonal, then vertical at
                // parent column for non-neighboring). Check the vertical at parent
                // column doesn't cross unrelated nodes.
                let dc = edge.to_col.abs_diff(edge.from_col);
                if dc > 1 {
                    let p_col = edge.to_col;
                    let (min_r, max_r) = (
                        edge.from_row.min(edge.to_row) + 1,
                        edge.from_row.max(edge.to_row),
                    );
                    for nr in min_r..max_r {
                        if let Some(name) = node_at.get(&(nr, p_col))
                            && name != &efrom(layout, edge)
                            && name != &eto(layout, edge)
                        {
                            errors.push(format!(
                                "edge {}→{} merge chamfer vertical at parent col {} crosses node {} at ({},{})",
                                efrom(layout, edge), eto(layout, edge),
                                p_col, name, nr, p_col
                            ));
                        }
                    }
                }
            }
        }
        errors
    }

    /// For branch edges (child_col != parent_col), verify the column
    /// change does NOT happen at the child's row. The horizontal jog
    /// should be below the child (closer to the parent), meaning the
    /// edge first goes vertically for at least one row before changing
    /// column.
    fn find_branch_jogs_at_child_row(layout: &GraphLayout) -> Vec<String> {
        let mut errors = Vec::new();
        for edge in &layout.edges {
            if edge.edge_type != EdgeType::Branch {
                continue;
            }
            if edge.from_col == edge.to_col {
                continue; // Same column, no jog
            }
            // Check waypoints: no waypoint should be at from_row with a
            // different column. This would mean the jog is at child's row.
            for wp in &edge.waypoints {
                if wp.0 == edge.from_row && wp.1 != edge.from_col {
                    let e_from = layout
                        .nodes
                        .iter()
                        .find(|n| n.row == edge.from_row)
                        .map(|n| n.oid.short_hex().to_string())
                        .unwrap_or_default();
                    errors.push(format!(
                        "branch edge from {} has waypoint at child row {} with col {} (expected col {} or no waypoint at child row)",
                        e_from, edge.from_row, wp.1, edge.from_col
                    ));
                }
            }
        }
        errors
    }

    #[test]
    fn f421e8d_no_edge_crosses_unrelated_node() {
        let commits = make_f421e8d_topology();
        let calc =
            GraphCalculator::new(commits, HashMap::new(), Vec::new(), GraphOptions::default());
        let layout = calc.calculate_layout();

        // First check basic verify (same-column pass-through)
        let verify_errors = layout.verify();
        assert!(
            verify_errors.is_empty(),
            "verify() found {} same-column pass-through error(s):\n{}",
            verify_errors.len(),
            verify_errors
                .iter()
                .take(5)
                .cloned()
                .collect::<Vec<_>>()
                .join("\n")
        );

        // Check rendered crossings (chamfer-aware)
        let crossings = find_rendered_crossings(&layout);
        assert!(
            crossings.is_empty(),
            "Found {} rendered crossing(s):\n{}",
            crossings.len(),
            crossings
                .iter()
                .take(10)
                .cloned()
                .collect::<Vec<_>>()
                .join("\n")
        );
    }

    #[test]
    fn f421e8d_branch_edges_jog_below_child() {
        let commits = make_f421e8d_topology();
        let calc =
            GraphCalculator::new(commits, HashMap::new(), Vec::new(), GraphOptions::default());
        let layout = calc.calculate_layout();

        let jog_errors = find_branch_jogs_at_child_row(&layout);
        assert!(
            jog_errors.is_empty(),
            "Found {} branch jog(s) at child row:\n{}",
            jog_errors.len(),
            jog_errors
                .iter()
                .take(10)
                .cloned()
                .collect::<Vec<_>>()
                .join("\n")
        );
    }

    #[test]
    fn f421e8d_all_four_children_reachable() {
        let commits = make_f421e8d_topology();
        let calc =
            GraphCalculator::new(commits, HashMap::new(), Vec::new(), GraphOptions::default());
        let layout = calc.calculate_layout();

        // f421e8d is OID 2. Its 4 children are OIDs 14, 9, 4, 3.
        let parent_oid = make_oid(2);
        let child_oids = [make_oid(14), make_oid(9), make_oid(4), make_oid(3)];

        let parent_node = layout
            .nodes
            .iter()
            .find(|n| n.oid == parent_oid)
            .expect("f421e8d must be in layout");

        for &child_oid in &child_oids {
            let child_node = layout
                .nodes
                .iter()
                .find(|n| n.oid == child_oid)
                .unwrap_or_else(|| panic!("child {:?} must be in layout", child_oid));

            // There must be an edge from child to parent
            let has_edge = layout.edges.iter().any(|e| {
                e.from_row == child_node.row
                    && e.from_col == child_node.column
                    && e.to_row == parent_node.row
                    && e.to_col == parent_node.column
            });
            assert!(
                has_edge,
                "child {:?} (row {}, col {}) should have edge to parent (row {}, col {})",
                child_oid, child_node.row, child_node.column, parent_node.row, parent_node.column
            );
        }
    }

    #[test]
    fn f421e8d_branch_children_in_separate_columns() {
        let commits = make_f421e8d_topology();
        let calc =
            GraphCalculator::new(commits, HashMap::new(), Vec::new(), GraphOptions::default());
        let layout = calc.calculate_layout();

        // 4e14921, 7dae642, a344ea2 should be in columns > 0
        // (separate from the mainline column 0)
        for &oid in &[make_oid(9), make_oid(4), make_oid(3)] {
            let node = layout.nodes.iter().find(|n| n.oid == oid).unwrap();
            assert!(
                node.column > 0,
                "branch child {:?} should be in column > 0, got {}",
                oid,
                node.column
            );
        }

        // f421e8d (parent) should be in column 0 (mainline)
        let parent = layout.nodes.iter().find(|n| n.oid == make_oid(2)).unwrap();
        assert_eq!(parent.column, 0, "f421e8d should be in mainline column 0");
    }

    #[test]
    fn fork_point_prefers_first_parent_chain_child() {
        // Topology where branch tip sorts earlier than mainline child due to
        // equal commit_time tiebreak by Oid. Without the first_parent_chain
        // preference in sort_children_by_row, the branch gets col 0 instead
        // of the mainline.
        //
        // oid1(HEAD, t=10) → oid2(t=9) → oid3(t=9) → oid5(merge, t=8)
        //                                           └→ oid7(branch, t=9)
        // oid7 has hex "0700..." > "0300..." (oid3) → oid7 gets row 1, oid3 row 3
        // Without fix: children_sorted[oid5] = [oid7, oid3] (row order)
        //   → oid7 gets token "" (col 0), oid3 gets "1" (col 1) — WRONG
        // With fix: first_parent_chain = {oid1,oid2,oid3,oid5,...}
        //   → children_sorted[oid5] = [oid3(fpc), oid7(not)] → oid3 gets col 0 — CORRECT
        let make_commit_at = |oid: u8, parents: Vec<u8>, time: i64| -> CommitInfo {
            CommitInfo {
                oid: make_oid(oid),
                short_oid: format!("{oid:02x}00000"),
                message: String::new(),
                summary: String::new(),
                author: Author {
                    name: "Test".to_string(),
                    email: "test@test.com".to_string(),
                },
                committer: Author {
                    name: "Test".to_string(),
                    email: "test@test.com".to_string(),
                },
                author_time: Utc.timestamp_opt(time, 0).single().unwrap(),
                commit_time: Utc.timestamp_opt(time, 0).single().unwrap(),
                parent_oids: parents.into_iter().map(make_oid).collect(),
                refs: Vec::new(),
            }
        };

        let commits = vec![
            make_commit_at(1, vec![2], 10),   // HEAD
            make_commit_at(7, vec![5], 9),    // branch tip
            make_commit_at(2, vec![3], 9),    // mainline
            make_commit_at(3, vec![5], 9),    // mainline child of merge
            make_commit_at(5, vec![4, 6], 8), // merge/fork point
            make_commit_at(4, vec![], 7),     // parent 1 of merge
            make_commit_at(6, vec![], 6),     // parent 2 of merge
        ];
        let calc =
            GraphCalculator::new(commits, HashMap::new(), Vec::new(), GraphOptions::default());
        let layout = calc.calculate_layout();

        // oid3 (mainline child of merge) should be at col 0
        let oid3 = layout.nodes.iter().find(|n| n.oid == make_oid(3)).unwrap();
        assert_eq!(
            oid3.column, 0,
            "mainline child oid3 should be at column 0, got {}",
            oid3.column
        );

        // oid7 (branch tip) should be at col > 0
        let oid7 = layout.nodes.iter().find(|n| n.oid == make_oid(7)).unwrap();
        assert!(
            oid7.column > 0,
            "branch tip oid7 should be at column > 0, got {}",
            oid7.column
        );
    }

    #[test]
    fn visual_grit_repo_fork_has_mainline_at_col_0() {
        // Exact topology of the grit repo fork point, using commit_time
        // values that force equal-timestamp tiebreak. Verifies the visual
        // layout a user would see:
        //
        //   oid4(row 0, HEAD, t=10)     oid7(row 1, branch tip, t=9)
        //        │                            │
        //   oid2(row 2, t=9)                  │
        //        │                            │
        //   oid3(row 3, child of merge, t=9)  │
        //        │                           /
        //      oid5(merge, t=8)
        //       /         \
        //   oid4(p1, t=7)  oid6(p2, t=6)
        //   (main below)   (branch below)
        //
        // oid7 has hex "07...", oid3 has hex "03..." → oid7 sorts first in heap
        // Without fix: oid7 becomes first child → col 0, oid3 → col 1
        // With fix: oid3 is on first-parent chain → col 0, oid7 → col 1
        let make = |oid: u8, parents: Vec<u8>, time: i64| -> CommitInfo {
            CommitInfo {
                oid: make_oid(oid),
                short_oid: format!("{oid:02x}00000"),
                message: String::new(),
                summary: String::new(),
                author: Author {
                    name: "Test".to_string(),
                    email: "test@test.com".to_string(),
                },
                committer: Author {
                    name: "Test".to_string(),
                    email: "test@test.com".to_string(),
                },
                author_time: Utc.timestamp_opt(time, 0).single().unwrap(),
                commit_time: Utc.timestamp_opt(time, 0).single().unwrap(),
                parent_oids: parents.into_iter().map(make_oid).collect(),
                refs: Vec::new(),
            }
        };

        let commits = vec![
            make(1, vec![2], 10),   // HEAD — mainline chain
            make(7, vec![5], 9),    // branch tip (non-fpc)
            make(2, vec![3], 9),    // mainline
            make(3, vec![5], 9),    // mainline child of merge (fpc)
            make(5, vec![4, 6], 8), // merge/fork point
            make(4, vec![], 7),     // parent 1 — mainline below merge
            make(6, vec![], 6),     // parent 2 — branch below merge
        ];
        let calc =
            GraphCalculator::new(commits, HashMap::new(), Vec::new(), GraphOptions::default());
        let layout = calc.calculate_layout();

        // MAINLINE chain (above and below merge) should be at col 0
        for &(oid, label) in &[
            (1, "HEAD"),
            (2, "mainline"),
            (3, "mainline child of merge"),
            (4, "parent 1/mainline below merge"),
        ] {
            let node = layout
                .nodes
                .iter()
                .find(|n| n.oid == make_oid(oid))
                .unwrap();
            assert_eq!(
                node.column, 0,
                "{label} should be at column 0, got {}",
                node.column
            );
        }

        // BRANCH chain (above and below merge) should be at col > 0
        for &(oid, label) in &[(7, "branch tip"), (6, "parent 2/branch below merge")] {
            let node = layout
                .nodes
                .iter()
                .find(|n| n.oid == make_oid(oid))
                .unwrap();
            assert!(
                node.column > 0,
                "{label} should be at column > 0, got {}",
                node.column
            );
        }

        // Verify the layout is structurally valid
        let errors = layout.verify();
        assert!(
            errors.is_empty(),
            "verify() should find no errors: {:?}",
            errors
        );
    }

    #[test]
    fn visual_three_way_fork_all_branches_right_of_mainline() {
        // Root with 4 children splitting at the same timestamp. The three
        // non-fpc children sit to the right of the mainline. Branches may
        // share columns if their row ranges don't overlap (column compaction).
        //
        //   oid6(HEAD, t=12)
        //        │
        //   oid2(mainline, t=11)   oid3 oid4 oid5(branches, t=11)
        //        │                    \    |    /
        //       oid1(root, t=10)
        //
        // children_sorted[oid1] = [oid2(fpc), oid3, oid4, oid5]
        // oid2 → col 0; oid3/oid4/oid5 → col > 0
        let make = |oid: u8, parents: Vec<u8>, time: i64| -> CommitInfo {
            CommitInfo {
                oid: make_oid(oid),
                short_oid: format!("{oid:02x}00000"),
                message: String::new(),
                summary: String::new(),
                author: Author {
                    name: "Test".to_string(),
                    email: "test@test.com".to_string(),
                },
                committer: Author {
                    name: "Test".to_string(),
                    email: "test@test.com".to_string(),
                },
                author_time: Utc.timestamp_opt(time, 0).single().unwrap(),
                commit_time: Utc.timestamp_opt(time, 0).single().unwrap(),
                parent_oids: parents.into_iter().map(make_oid).collect(),
                refs: Vec::new(),
            }
        };

        let commits = vec![
            make(6, vec![2], 12), // HEAD (child of mainline)
            make(2, vec![1], 11), // mainline child of root (fpc)
            make(3, vec![1], 11), // branch 1
            make(4, vec![1], 11), // branch 2
            make(5, vec![1], 11), // branch 3
            make(1, vec![], 10),  // root
        ];
        let calc =
            GraphCalculator::new(commits, HashMap::new(), Vec::new(), GraphOptions::default());
        let layout = calc.calculate_layout();

        // Mainline at col 0
        let mainline = layout.nodes.iter().find(|n| n.oid == make_oid(2)).unwrap();
        assert_eq!(
            mainline.column, 0,
            "mainline should be at column 0, got {}",
            mainline.column
        );

        // All branches at col > 0
        for &oid in &[3u8, 4, 5] {
            let node = layout
                .nodes
                .iter()
                .find(|n| n.oid == make_oid(oid))
                .unwrap();
            assert!(
                node.column > 0,
                "branch oid{oid} should be at column > 0, got {}",
                node.column
            );
        }

        let errors = layout.verify();
        assert!(
            errors.is_empty(),
            "verify() should find no errors: {:?}",
            errors
        );
    }

    #[test]
    fn visual_pre_insertion_triggers_on_any_child() {
        // Root has two children: a non-fpc branch tip at row 1, and an fpc
        // mainline child at row 120. The branch tip triggers pre-insertion
        // of root's thread at row ~2 (even though children_sorted[0] is the
        // fpc child at row 120). Without the pre-insertion fix, root enters
        // the idlist only at row 120 → no arrow_gap on the branch edge.
        //
        //   oid120(HEAD, t=130) → oid119 → ... → oid2 → oid1(root, t=1)
        //   oid121(branch, t=129) ────────────────────────────┘
        //
        // oid1 has children [oid2(fpc, row ~119), oid121(branch, row 1)].
        // oid121 is NOT on the first-parent chain from HEAD.
        let mut commits = vec![];
        // Mainline chain: oid120 → oid119 → ... → oid2 → oid1(root)
        for i in (2..=120u8).rev() {
            let parent = i - 1; // i → i-1 (e.g. oid120 → oid119, oid2 → oid1)
            let time = i as i64 + 100;
            commits.push(CommitInfo {
                oid: make_oid(i),
                short_oid: format!("{i:02x}00000"),
                message: format!("main {i}"),
                summary: format!("main {i}"),
                author: Author {
                    name: "T".into(),
                    email: "t@t.com".into(),
                },
                committer: Author {
                    name: "T".into(),
                    email: "t@t.com".into(),
                },
                author_time: Utc.timestamp_opt(time, 0).single().unwrap(),
                commit_time: Utc.timestamp_opt(time, 0).single().unwrap(),
                parent_oids: vec![make_oid(parent)],
                refs: Vec::new(),
            });
        }
        // root (oid1)
        let root_time = 101i64;
        commits.push(CommitInfo {
            oid: make_oid(1),
            short_oid: "0100000".into(),
            message: "root".into(),
            summary: "root".into(),
            author: Author {
                name: "T".into(),
                email: "t@t.com".into(),
            },
            committer: Author {
                name: "T".into(),
                email: "t@t.com".into(),
            },
            author_time: Utc.timestamp_opt(root_time, 0).single().unwrap(),
            commit_time: Utc.timestamp_opt(root_time, 0).single().unwrap(),
            parent_oids: vec![],
            refs: Vec::new(),
        });
        // branch tip (oid121). Parents = [oid1 (root)]
        commits.push(CommitInfo {
            oid: make_oid(121),
            short_oid: "12100000".into(),
            message: "branch".into(),
            summary: "branch".into(),
            author: Author {
                name: "T".into(),
                email: "t@t.com".into(),
            },
            committer: Author {
                name: "T".into(),
                email: "t@t.com".into(),
            },
            author_time: Utc.timestamp_opt(229, 0).single().unwrap(),
            commit_time: Utc.timestamp_opt(229, 0).single().unwrap(),
            parent_oids: vec![make_oid(1)],
            refs: Vec::new(),
        });
        // HEAD merge (oid120's child = oid122)
        // Actually, rework: HEAD = oid122 = merge of oid120 (main) and oid121 (branch)
        commits.push(CommitInfo {
            oid: make_oid(122),
            short_oid: "12200000".into(),
            message: "merge".into(),
            summary: "merge".into(),
            author: Author {
                name: "T".into(),
                email: "t@t.com".into(),
            },
            committer: Author {
                name: "T".into(),
                email: "t@t.com".into(),
            },
            author_time: Utc.timestamp_opt(230, 0).single().unwrap(),
            commit_time: Utc.timestamp_opt(230, 0).single().unwrap(),
            parent_oids: vec![make_oid(120), make_oid(121)],
            refs: Vec::new(),
        });

        let calc =
            GraphCalculator::new(commits, HashMap::new(), Vec::new(), GraphOptions::default());
        let layout = calc.calculate_layout();

        let branch_node = layout
            .nodes
            .iter()
            .find(|n| n.oid == make_oid(121))
            .unwrap();
        let root_node = layout.nodes.iter().find(|n| n.oid == make_oid(1)).unwrap();

        let branch_edge = layout
            .edges
            .iter()
            .find(|e| e.from_row == branch_node.row && e.to_row == root_node.row)
            .expect("should have edge from branch to root");

        // Branch→root spans ~120 rows; thread should be removed and
        // re-inserted, producing an arrow_gap. Without the pre-insertion
        // fix (any-child check), the root wouldn't enter the idlist early
        // enough to be removed → no arrow_gap.
        assert!(
            branch_edge.arrow_gap.is_some(),
            "long-spanning branch→root edge should have arrow_gap from pre-insertion + thread removal"
        );

        let errors = layout.verify();
        assert!(
            errors.is_empty(),
            "verify() should find no errors: {:?}",
            errors
        );
    }

    // -----------------------------------------------------------------------
    // New property tests derived from gitk algorithm invariants
    // -----------------------------------------------------------------------

    /// Helper: build a layout and return its rowidlist alongside the layout.
    /// Re-runs assign_columns internally so we can inspect the raw rowidlist.
    fn layout_with_rowidlist(commits: Vec<CommitInfo>) -> GraphLayout {
        let calc =
            GraphCalculator::new(commits, HashMap::new(), Vec::new(), GraphOptions::default());
        calc.calculate_layout()
    }

    // --- 1. Unique node positions -------------------------------------------

    /// gitk places exactly one commit per (row, column) cell.
    /// No two nodes may share the same (row, col) pair.
    #[test]
    fn prop_unique_node_positions() {
        let topologies: Vec<Vec<CommitInfo>> = vec![
            // linear
            vec![
                make_commit(1, vec![], "root"),
                make_commit(2, vec![1], "a"),
                make_commit(3, vec![2], "b"),
            ],
            // simple fork
            vec![
                make_commit(1, vec![], "root"),
                make_commit(2, vec![1], "main"),
                make_commit(3, vec![1], "branch"),
                make_commit(4, vec![2, 3], "merge"),
            ],
            // 4-way fan-out
            vec![
                make_commit(1, vec![], "root"),
                make_commit(2, vec![1], "a"),
                make_commit(3, vec![1], "b"),
                make_commit(4, vec![1], "c"),
                make_commit(5, vec![1], "d"),
            ],
            // diamond chain
            vec![
                make_commit(1, vec![], "root"),
                make_commit(2, vec![1], "left"),
                make_commit(3, vec![1], "right"),
                make_commit(4, vec![2], "left2"),
                make_commit(5, vec![3], "right2"),
                make_commit(6, vec![4, 5], "merge"),
            ],
        ];
        for commits in topologies {
            let layout = layout_with_rowidlist(commits);
            let mut positions: HashSet<(usize, usize)> = HashSet::new();
            for node in &layout.nodes {
                assert!(
                    positions.insert((node.row, node.column)),
                    "duplicate node position ({}, {}) for oid {}",
                    node.row,
                    node.column,
                    node.oid.short_hex()
                );
            }
        }
    }

    // --- 2. optimize_rows max-step-1 ----------------------------------------

    /// gitk's optimize_rows guarantees a thread moves at most 1 column between
    /// consecutive rows (unless it has an arrow gap).  Formally: for every edge,
    /// consecutive waypoints (including the endpoints) differ by at most 1 in
    /// column.  Edges with arrow_gap are exempt for the gap region.
    #[test]
    fn prop_thread_moves_at_most_one_column_per_row() {
        let commits = vec![
            make_commit(1, vec![], "root"),
            make_commit(2, vec![1], "main1"),
            make_commit(3, vec![1], "brA1"),
            make_commit(4, vec![1], "brB1"),
            make_commit(5, vec![2], "main2"),
            make_commit(6, vec![3], "brA2"),
            make_commit(7, vec![4], "brB2"),
            make_commit(8, vec![5], "main3"),
            make_commit(9, vec![6], "brA3"),
            make_commit(10, vec![7], "brB3"),
            make_commit(11, vec![8, 9], "mergeAB"),
            make_commit(12, vec![11, 10], "final"),
        ];
        let layout = layout_with_rowidlist(commits);

        for edge in &layout.edges {
            // Build the full ordered point sequence for this edge,
            // skipping the gap region when arrow_gap is present.
            let mut segments: Vec<Vec<(usize, usize)>> = Vec::new();
            if let Some((gap_lo, gap_hi)) = edge.arrow_gap {
                let mut seg1 = vec![(edge.from_row, edge.from_col)];
                for &wp in &edge.waypoints {
                    if wp.0 <= gap_lo {
                        seg1.push(wp);
                    }
                }
                segments.push(seg1);
                let mut seg2: Vec<(usize, usize)> = edge
                    .waypoints
                    .iter()
                    .filter(|wp| wp.0 >= gap_hi)
                    .copied()
                    .collect();
                seg2.push((edge.to_row, edge.to_col));
                segments.push(seg2);
            } else {
                let mut path = vec![(edge.from_row, edge.from_col)];
                path.extend_from_slice(&edge.waypoints);
                path.push((edge.to_row, edge.to_col));
                segments.push(path);
            }
            for seg in &segments {
                for w in seg.windows(2) {
                    let (r1, c1) = w[0];
                    let (r2, c2) = w[1];
                    let col_delta = (c2 as i64 - c1 as i64).unsigned_abs() as usize;
                    // Column may shift during the waypoint span, but
                    // each individual waypoint step (row delta ≥ 1) should
                    // shift by at most (row_delta) columns — in practice
                    // optimize_rows limits it to 1 per row.
                    let row_delta = (r2 as i64 - r1 as i64).unsigned_abs() as usize;
                    assert!(
                        col_delta <= row_delta,
                        "edge ({},{})→({},{}) segment ({},{})→({},{}) moves {} cols over {} rows (max 1/row)",
                        edge.from_row,
                        edge.from_col,
                        edge.to_row,
                        edge.to_col,
                        r1,
                        c1,
                        r2,
                        c2,
                        col_delta,
                        row_delta
                    );
                }
            }
        }
    }

    // --- 3. Anti-jig: no left-then-right zigzag in consecutive rows -----------

    /// gitk's optimize_rows inserts padding to prevent a thread going left
    /// in one row and immediately right in the next (and vice-versa).
    /// For each edge's waypoint sequence, three consecutive points must not
    /// form a "V" or "inverted-V" shape (col decreases then increases, or
    /// increases then decreases) across consecutive rows.
    #[test]
    fn prop_no_consecutive_zigzag_in_waypoints() {
        let commits = vec![
            make_commit(1, vec![], "root"),
            make_commit(2, vec![1], "main"),
            make_commit(3, vec![1], "branch"),
            make_commit(4, vec![2], "main2"),
            make_commit(5, vec![3], "branch2"),
            make_commit(6, vec![4, 5], "merge"),
        ];
        let layout = layout_with_rowidlist(commits);

        for edge in &layout.edges {
            if edge.arrow_gap.is_some() {
                continue; // gaps break continuity; exempt
            }
            let mut path = vec![(edge.from_row, edge.from_col)];
            path.extend_from_slice(&edge.waypoints);
            path.push((edge.to_row, edge.to_col));

            for w in path.windows(3) {
                let (_r0, c0) = w[0];
                let (_r1, c1) = w[1];
                let (_r2, c2) = w[2];
                let d1 = c1 as i64 - c0 as i64; // direction into middle point
                let d2 = c2 as i64 - c1 as i64; // direction out of middle point
                // A zigzag is d1 < 0 && d2 > 0 (left then right) or
                // d1 > 0 && d2 < 0 (right then left).
                assert!(
                    !(d1 < 0 && d2 > 0 || d1 > 0 && d2 < 0),
                    "edge ({},{})→({},{}) zigzags at waypoints ({:?}→{:?}→{:?})",
                    edge.from_row,
                    edge.from_col,
                    edge.to_row,
                    edge.to_col,
                    w[0],
                    w[1],
                    w[2]
                );
            }
        }
    }

    // --- 4. Edge to_col matches parent node column --------------------------

    /// Every edge's to_col must equal the column of the parent node.
    /// Every edge's from_col must equal the column of the child node.
    /// This ensures the rowidlist column assignments are self-consistent.
    #[test]
    fn prop_edge_endpoints_match_node_columns() {
        let topologies: &[&[_]] = &[
            &[
                make_commit(1, vec![], "root"),
                make_commit(2, vec![1], "a"),
                make_commit(3, vec![1], "b"),
                make_commit(4, vec![2, 3], "merge"),
            ],
            &[
                make_commit(1, vec![], "r1"),
                make_commit(2, vec![], "r2"),
                make_commit(3, vec![1], "a"),
                make_commit(4, vec![2], "b"),
                make_commit(5, vec![3, 4], "merge"),
            ],
        ];
        for &commits in topologies {
            let layout = layout_with_rowidlist(commits.to_vec());
            let node_pos: HashMap<Oid, (usize, usize)> = layout
                .nodes
                .iter()
                .map(|n| (n.oid, (n.row, n.column)))
                .collect();
            // Build child-oid map: row → oid for nodes
            let row_to_oid: HashMap<usize, Oid> =
                layout.nodes.iter().map(|n| (n.row, n.oid)).collect();
            for edge in &layout.edges {
                if let Some(&(_, from_col)) = row_to_oid
                    .get(&edge.from_row)
                    .and_then(|oid| node_pos.get(oid))
                {
                    assert_eq!(
                        edge.from_col, from_col,
                        "edge from_col {} ≠ child node column {} at row {}",
                        edge.from_col, from_col, edge.from_row
                    );
                }
                if let Some(&(_, to_col)) = row_to_oid
                    .get(&edge.to_row)
                    .and_then(|oid| node_pos.get(oid))
                {
                    assert_eq!(
                        edge.to_col, to_col,
                        "edge to_col {} ≠ parent node column {} at row {}",
                        edge.to_col, to_col, edge.to_row
                    );
                }
            }
        }
    }

    // --- 5. verify() passes for diverse topologies --------------------------

    /// verify() must report zero pass-through errors for all of these.
    #[test]
    fn prop_verify_passes_for_diverse_topologies() {
        // (a) diamond
        let diamond = vec![
            make_commit(1, vec![], "root"),
            make_commit(2, vec![1], "left"),
            make_commit(3, vec![1], "right"),
            make_commit(4, vec![2, 3], "merge"),
        ];
        // (b) staircase merges: each step merges a new branch into main
        let staircase = vec![
            make_commit(1, vec![], "root"),
            make_commit(2, vec![1], "main1"),
            make_commit(3, vec![1], "br1"),
            make_commit(4, vec![2, 3], "merge1"),
            make_commit(5, vec![4], "main2"),
            make_commit(6, vec![4], "br2"),
            make_commit(7, vec![5, 6], "merge2"),
            make_commit(8, vec![7], "main3"),
            make_commit(9, vec![7], "br3"),
            make_commit(10, vec![8, 9], "merge3"),
        ];
        // (c) criss-cross: two chains swap parents (octopus-style)
        let criss = vec![
            make_commit(1, vec![], "r1"),
            make_commit(2, vec![], "r2"),
            make_commit(3, vec![1], "a1"),
            make_commit(4, vec![2], "b1"),
            make_commit(5, vec![3, 4], "merge_ab"),
            make_commit(6, vec![4, 3], "merge_ba"),
            make_commit(7, vec![5, 6], "top"),
        ];
        // (d) long single-parent chain (tests straight-line efficiency)
        let chain: Vec<CommitInfo> = (1u8..=20)
            .map(|i| make_commit(i, if i == 1 { vec![] } else { vec![i - 1] }, "chain"))
            .collect();
        // (e) three-way fan-out and fan-in
        let octopus = vec![
            make_commit(1, vec![], "root"),
            make_commit(2, vec![1], "a"),
            make_commit(3, vec![1], "b"),
            make_commit(4, vec![1], "c"),
            make_commit(5, vec![2, 3, 4], "octomerge"),
        ];
        for (label, commits) in &[
            ("diamond", diamond),
            ("staircase", staircase),
            ("criss-cross", criss),
            ("chain-20", chain),
            ("octopus", octopus),
        ] {
            let layout = layout_with_rowidlist(commits.clone());
            let errors = layout.verify();
            assert!(
                errors.is_empty(),
                "{label}: verify() found {} error(s): {}",
                errors.len(),
                errors
                    .iter()
                    .take(3)
                    .cloned()
                    .collect::<Vec<_>>()
                    .join("; ")
            );
        }
    }

    // --- 6. Linear column stability -----------------------------------------

    /// A single-parent chain keeps the same column throughout.
    #[test]
    fn prop_linear_chain_same_column() {
        for len in [3, 10, 20, 50] {
            let commits: Vec<CommitInfo> = (1u8..=len)
                .map(|i| make_commit(i, if i == 1 { vec![] } else { vec![i - 1] }, "linear"))
                .collect();
            let layout = layout_with_rowidlist(commits);
            for node in &layout.nodes {
                assert_eq!(
                    node.column,
                    0,
                    "linear chain node {} at row {} should be in column 0, got {}",
                    node.oid.short_hex(),
                    node.row,
                    node.column
                );
            }
            assert_eq!(
                layout.total_columns, 1,
                "linear chain of len {} should use 1 column",
                len
            );
        }
    }

    // --- 7. Anti-jig (comprehensive) ----------------------------------------

    /// A thread never goes left then immediately right (zigzag).
    /// Tested across a diverse set of topologies, with and without arrow gaps.
    #[test]
    fn prop_no_zigzag_across_topologies() {
        let topologies: Vec<Vec<CommitInfo>> = vec![
            // diamond
            vec![
                make_commit(1, vec![], "root"),
                make_commit(2, vec![1], "a"),
                make_commit(3, vec![1], "b"),
                make_commit(4, vec![2, 3], "merge"),
            ],
            // three-way fan-out octopus
            vec![
                make_commit(1, vec![], "root"),
                make_commit(2, vec![1], "a"),
                make_commit(3, vec![1], "b"),
                make_commit(4, vec![1], "c"),
                make_commit(5, vec![2, 3, 4], "octopus"),
            ],
            // staircase merges
            vec![
                make_commit(1, vec![], "root"),
                make_commit(2, vec![1], "m1"),
                make_commit(3, vec![1], "b1"),
                make_commit(4, vec![2, 3], "merge1"),
                make_commit(5, vec![4], "m2"),
                make_commit(6, vec![4], "b2"),
                make_commit(7, vec![5, 6], "merge2"),
            ],
            // criss-cross
            vec![
                make_commit(1, vec![], "r1"),
                make_commit(2, vec![], "r2"),
                make_commit(3, vec![1], "a"),
                make_commit(4, vec![2], "b"),
                make_commit(5, vec![3, 4], "m_ab"),
                make_commit(6, vec![4, 3], "m_ba"),
                make_commit(7, vec![5, 6], "top"),
            ],
            // chain with side branch
            vec![
                make_commit(1, vec![], "root"),
                make_commit(2, vec![1], "main"),
                make_commit(3, vec![2], "main2"),
                make_commit(4, vec![2], "branch"),
                make_commit(5, vec![3], "main3"),
                make_commit(6, vec![4], "branch2"),
                make_commit(7, vec![5, 6], "merge"),
            ],
        ];

        for topology in &topologies {
            let layout = layout_with_rowidlist(topology.clone());
            for edge in &layout.edges {
                if edge.arrow_gap.is_some() {
                    continue;
                }
                let mut path = vec![(edge.from_row, edge.from_col)];
                path.extend_from_slice(&edge.waypoints);
                path.push((edge.to_row, edge.to_col));

                for w in path.windows(3) {
                    let (_r0, c0) = w[0];
                    let (_r1, c1) = w[1];
                    let (_r2, c2) = w[2];
                    let d1 = c1 as i64 - c0 as i64;
                    let d2 = c2 as i64 - c1 as i64;
                    assert!(
                        !(d1 < 0 && d2 > 0 || d1 > 0 && d2 < 0),
                        "edge ({},{})→({},{}) zigzags at ({},{})→({},{})→({},{})",
                        edge.from_row,
                        edge.from_col,
                        edge.to_row,
                        edge.to_col,
                        w[0].0,
                        w[0].1,
                        w[1].0,
                        w[1].1,
                        w[2].0,
                        w[2].1
                    );
                }
            }
        }
    }

    // --- 8. Thread pre-insertion --------------------------------------------

    /// A parent that appears much later than its child should have its thread
    /// pre-inserted UPARROW_LEN rows before the parent's own row, producing
    /// an arrow_gap on the edge.
    ///
    /// Builds a long main chain plus a side branch from root with a timestamp
    /// that places the branch tip early in the display order, creating a gap
    /// between the branch commit and its root ancestor that exceeds the
    /// arrow_gap_threshold. The edge must have an arrow_gap.
    #[test]
    fn prop_thread_preinsertion_creates_arrow_gap() {
        //   root(1) → 2 → 3 → ... → 120
        //   branch(121) ──────────────┘
        // Edge branch(121)→root(1) spans ~120 rows.
        let mut commits: Vec<CommitInfo> = Vec::new();
        // Main chain: oid120 → ... → oid1(root) with increasing timestamps
        for i in (2..=120u8).rev() {
            let parent = i - 1;
            commits.push(make_commit(i, vec![parent], "main"));
        }
        commits.push(make_commit(1, vec![], "root"));
        // Branch tip from root — timestamp makes it appear early
        commits.push(make_commit(121, vec![1], "branch"));

        let layout = layout_with_rowidlist(commits);
        let root_node = layout.nodes.iter().find(|n| n.oid == make_oid(1)).unwrap();
        let branch_node = layout
            .nodes
            .iter()
            .find(|n| n.oid == make_oid(121))
            .unwrap();
        let branch_edge = layout
            .edges
            .iter()
            .find(|e| e.from_row == branch_node.row && e.to_row == root_node.row)
            .expect("branch→root edge should exist");

        assert!(
            branch_edge.arrow_gap.is_some(),
            "branch→root edge spanning > removal threshold should have arrow_gap (pre-insertion + thread removal)"
        );

        // The arrow_gap bounds should be within the expected range.
        let (gap_lo, gap_hi) = branch_edge.arrow_gap.unwrap();
        assert!(
            gap_lo > branch_edge.from_row,
            "arrow_gap lower bound {} should be > child row {}",
            gap_lo,
            branch_edge.from_row
        );
        assert!(
            gap_hi <= root_node.row,
            "arrow_gap upper bound {} should be ≤ parent row {}",
            gap_hi,
            root_node.row
        );
    }

    /// Short branch gaps should NOT produce arrow_gap — pre-insertion keeps
    /// the thread alive within the removal threshold.
    #[test]
    fn prop_short_gap_no_arrow_gap() {
        let commits = vec![
            make_commit(1, vec![], "root"),
            make_commit(2, vec![1], "a"),
            make_commit(3, vec![1], "branch"),
            make_commit(4, vec![2], "b"),
            make_commit(5, vec![3, 4], "merge"),
        ];
        let layout = layout_with_rowidlist(commits);
        for edge in &layout.edges {
            assert!(
                edge.arrow_gap.is_none(),
                "short edge ({},{})→({},{}) should not have arrow_gap",
                edge.from_row,
                edge.from_col,
                edge.to_row,
                edge.to_col
            );
        }
    }

    // --- 9. nextuse/prevuse correctness -------------------------------------

    /// nextuse finds the nearest child *after* a given row.
    /// prevuse finds the nearest child *before* a given row.
    /// Test with multiple children in non-monotonic row order.
    #[test]
    fn prop_nextuse_prevuse_correctness() {
        use std::collections::HashMap;

        // Children of 'parent' appear at rows 2, 5, 8 (in that order when sorted).
        // We build children_sorted and row_assignments to reflect this.
        let parent = make_oid(1);
        let child_a = make_oid(2); // row 2
        let child_b = make_oid(3); // row 5
        let child_c = make_oid(4); // row 8

        let mut children_sorted: HashMap<Oid, Vec<Oid>> = HashMap::new();
        // gitk-style order: first child in display order (could be any of them).
        // The fpc is child_b (row 5), but we list all children.
        children_sorted.insert(parent, vec![child_b, child_a, child_c]);

        let mut row_assignments: HashMap<Oid, usize> = HashMap::new();
        row_assignments.insert(child_a, 2);
        row_assignments.insert(child_b, 5);
        row_assignments.insert(child_c, 8);
        row_assignments.insert(parent, 0);

        // nextuse: nearest child >= from_row
        // from_row=0 → nearest is child_a at row 2
        assert_eq!(
            GraphCalculator::nextuse(parent, 0, &children_sorted, &row_assignments),
            Some(2),
            "nextuse(0) should find child_a at row 2"
        );
        // from_row=2 → nearest is child_a (fits > from_row? No, >2 excludes row 2)
        // Actually, nextuse checks > from_row, not >=
        // from_row=2: children > 2 are child_b (5) and child_c (8) → nearest = 5
        assert_eq!(
            GraphCalculator::nextuse(parent, 2, &children_sorted, &row_assignments),
            Some(5),
            "nextuse(2) should find child_b at row 5"
        );
        // from_row=6: only child_c (8) > 6
        assert_eq!(
            GraphCalculator::nextuse(parent, 6, &children_sorted, &row_assignments),
            Some(8),
            "nextuse(6) should find child_c at row 8"
        );
        // from_row=9: no child > 9 → fallback to parent's own row (0)
        assert_eq!(
            GraphCalculator::nextuse(parent, 9, &children_sorted, &row_assignments),
            Some(0),
            "nextuse(9) should fallback to parent's own row"
        );

        // prevuse: most recent child < from_row
        // from_row=3 → children < 3 are child_a (2) only
        assert_eq!(
            GraphCalculator::prevuse(parent, 3, &children_sorted, &row_assignments),
            Some(2),
            "prevuse(3) should find child_a at row 2"
        );
        // from_row=6 → children < 6 are child_a (2), child_b (5) → most recent = 5
        assert_eq!(
            GraphCalculator::prevuse(parent, 6, &children_sorted, &row_assignments),
            Some(5),
            "prevuse(6) should find child_b at row 5"
        );
        // from_row=9 → all children < 9, most recent = 8
        assert_eq!(
            GraphCalculator::prevuse(parent, 9, &children_sorted, &row_assignments),
            Some(8),
            "prevuse(9) should find child_c at row 8"
        );
        // from_row=0 → no child < 0 → None
        assert_eq!(
            GraphCalculator::prevuse(parent, 0, &children_sorted, &row_assignments),
            None,
            "prevuse(0) should return None"
        );

        // Edge case: no children
        let lone_oid = make_oid(99);
        let empty_children: HashMap<Oid, Vec<Oid>> = HashMap::new();
        assert_eq!(
            GraphCalculator::nextuse(lone_oid, 0, &empty_children, &row_assignments),
            None,
            "nextuse on childless oid should return None"
        );
        assert_eq!(
            GraphCalculator::prevuse(lone_oid, 0, &empty_children, &row_assignments),
            None,
            "prevuse on childless oid should return None"
        );
    }

    // --- 10. Column count compactness ---------------------------------------

    /// The total number of columns should never exceed the number of concurrent
    /// active branches at any point in the graph. For a given topology:
    ///   total_columns ≤ max over rows of (#rowidlist entries at that row)
    ///
    /// In simple terms, the column count should be bounded by the maximum
    /// number of simultaneously alive threads.
    #[test]
    fn prop_column_count_compactness() {
        let topologies: Vec<(&str, Vec<CommitInfo>, usize)> = vec![
            (
                "linear-10",
                {
                    (1u8..=10)
                        .map(|i| make_commit(i, if i == 1 { vec![] } else { vec![i - 1] }, "l"))
                        .collect()
                },
                1,
            ),
            (
                "diamond",
                vec![
                    make_commit(1, vec![], "root"),
                    make_commit(2, vec![1], "a"),
                    make_commit(3, vec![1], "b"),
                    make_commit(4, vec![2, 3], "merge"),
                ],
                2,
            ),
            (
                "three-way-fanout",
                vec![
                    make_commit(1, vec![], "root"),
                    make_commit(2, vec![1], "a"),
                    make_commit(3, vec![1], "b"),
                    make_commit(4, vec![1], "c"),
                ],
                3,
            ),
            (
                "octopus-merge",
                vec![
                    make_commit(1, vec![], "root"),
                    make_commit(2, vec![1], "a"),
                    make_commit(3, vec![1], "b"),
                    make_commit(4, vec![1], "c"),
                    make_commit(5, vec![2, 3, 4], "merge"),
                ],
                3,
            ),
            (
                "staircase-3",
                vec![
                    make_commit(1, vec![], "root"),
                    make_commit(2, vec![1], "m1"),
                    make_commit(3, vec![1], "b1"),
                    make_commit(4, vec![2, 3], "merge1"),
                    make_commit(5, vec![4], "m2"),
                    make_commit(6, vec![4], "b2"),
                    make_commit(7, vec![5, 6], "merge2"),
                    make_commit(8, vec![7], "m3"),
                    make_commit(9, vec![7], "b3"),
                    make_commit(10, vec![8, 9], "merge3"),
                ],
                2,
            ),
            (
                "criss-cross",
                vec![
                    make_commit(1, vec![], "r1"),
                    make_commit(2, vec![], "r2"),
                    make_commit(3, vec![1], "a"),
                    make_commit(4, vec![2], "b"),
                    make_commit(5, vec![3, 4], "m_ab"),
                    make_commit(6, vec![4, 3], "m_ba"),
                    make_commit(7, vec![5, 6], "top"),
                ],
                3,
            ),
        ];

        for (label, commits, expected_max_cols) in &topologies {
            let layout = layout_with_rowidlist(commits.clone());
            assert!(
                layout.total_columns <= *expected_max_cols + 1,
                "{label}: total_columns ({}) should be ≤ {} (expected max + 1)",
                layout.total_columns,
                expected_max_cols + 1
            );

            // Also verify that total_columns never exceeds the number of nodes
            assert!(
                layout.total_columns <= layout.nodes.len(),
                "{label}: total_columns ({}) should be ≤ node count ({})",
                layout.total_columns,
                layout.nodes.len()
            );

            // row_max_column may exceed total_columns because edge fan-out
            // adjustments happen after the base total_columns computation.
            // But each row's width should be ≤ total_columns * 2 as a sanity
            // check (a row should not blow up arbitrarily).
            for (row_idx, &rwc) in layout.row_max_column.iter().enumerate() {
                assert!(
                    rwc <= layout.total_columns * 2 + 1,
                    "{label}: row_max_column[{}]={} exceeds sanity bound ({})",
                    row_idx,
                    rwc,
                    layout.total_columns * 2 + 1
                );
            }
        }
    }

    // --- 8. Oblique edge routing: 45° max slant ------------------------------

    /// For any cross-column edge with waypoints (thread trace), every segment of
    /// the path must be either axis-aligned (vertical/horizontal) or an exactly
    /// 1×1 diagonal (|dr|=|dc|=1). This enforces the "half right angle" rule —
    /// no shallow diagonals at other angles.
    ///
    /// Edges spanning ≤1 in both row and column (1×1 grid square) are exempt —
    /// a direct diagonal at any angle is fine for such short connections.
    ///
    /// Edges with empty waypoints are skipped — the frontend chamfer renderer
    /// enforces the 45° constraint.
    #[test]
    fn prop_oblique_edge_routing() {
        let make = make_commit;
        let topologies: Vec<Vec<CommitInfo>> = vec![
            // diamond
            vec![
                make(1, vec![], "root"),
                make(2, vec![1], "a"),
                make(3, vec![1], "b"),
                make(4, vec![2, 3], "merge"),
            ],
            // 3-fanout
            vec![
                make(1, vec![], "root"),
                make(2, vec![1], "a"),
                make(3, vec![1], "b"),
                make(4, vec![1], "c"),
            ],
            // staircase-3 (from prop_column_count_compactness)
            vec![
                make(1, vec![], "root"),
                make(2, vec![1], "m1"),
                make(3, vec![1], "b1"),
                make(4, vec![2, 3], "merge1"),
                make(5, vec![4], "m2"),
                make(6, vec![4], "b2"),
                make(7, vec![5, 6], "merge2"),
                make(8, vec![7], "m3"),
                make(9, vec![7], "b3"),
                make(10, vec![8, 9], "merge3"),
            ],
            // criss-cross
            vec![
                make(1, vec![], "r1"),
                make(2, vec![], "r2"),
                make(3, vec![1], "a"),
                make(4, vec![2], "b"),
                make(5, vec![3, 4], "m_ab"),
                make(6, vec![4, 3], "m_ba"),
                make(7, vec![5, 6], "top"),
            ],
            // f421e8d topology (branch edges with waypoints/arrow_gaps)
            make_f421e8d_topology(),
            // gitv-repo topology
            vec![
                make(1, vec![], "7688f42"),
                make(2, vec![1], "3754661"),
                make(3, vec![2], "1ce4d2d"),
                make(4, vec![3], "4ab690a"),
                make(5, vec![4], "029ceed"),
                make(6, vec![1], "5319794"),
                make(7, vec![6], "2435ba9"),
                make(8, vec![5, 7], "973fefb"),
                make(9, vec![8], "HEAD"),
            ],
            // multiple branches and merges (from prop_thread_moves)
            vec![
                make(1, vec![], "root"),
                make(2, vec![1], "main1"),
                make(3, vec![1], "brA1"),
                make(4, vec![1], "brB1"),
                make(5, vec![2], "main2"),
                make(6, vec![3], "brA2"),
                make(7, vec![4], "brB2"),
                make(8, vec![5], "main3"),
                make(9, vec![6], "brA3"),
                make(10, vec![7], "brB3"),
                make(11, vec![8, 9], "mergeAB"),
                make(12, vec![11, 10], "final"),
            ],
        ];

        for commits in &topologies {
            let layout = layout_with_rowidlist(commits.clone());

            for edge in &layout.edges {
                if edge.from_col == edge.to_col {
                    continue; // same-column, not oblique
                }

                let dr = edge.to_row.abs_diff(edge.from_row);
                let dc = edge.to_col.abs_diff(edge.from_col);

                if dr <= 1 && dc <= 1 {
                    continue; // 1×1 exception: direct diagonal allowed
                }

                if edge.waypoints.is_empty() {
                    continue; // frontend chamfer handles it
                }

                // Build the full ordered path
                let mut path = vec![(edge.from_row, edge.from_col)];
                path.extend(edge.waypoints.iter().copied());
                path.push((edge.to_row, edge.to_col));

                for w in path.windows(2) {
                    let ddr = w[1].0.abs_diff(w[0].0);
                    let ddc = w[1].1.abs_diff(w[0].1);
                    assert!(
                        ddr == 0 || ddc == 0 || (ddr == 1 && ddc == 1),
                        "oblique edge ({},{})→({},{}) has segment ({},{})→({},{}) \
                         with angle not at 0°, 90°, or 45° (|dr|={}, |dc|={})",
                        edge.from_row,
                        edge.from_col,
                        edge.to_row,
                        edge.to_col,
                        w[0].0,
                        w[0].1,
                        w[1].0,
                        w[1].1,
                        ddr,
                        ddc
                    );
                }
            }
        }
    }

    // --- 7. Regression tests -------------------------------------------------

    /// Regression: edge from a side-branch child to a branching-point parent
    /// must NOT have its waypoints discarded by the trivial waypoint filter
    /// when the parent's thread is at a different column than the parent's
    /// final column. This reproduces the gitv repo topology (commits
    /// 5319794 → 7688f42) — discarding waypoints made the frontend fall
    /// back to a chamfer path whose vertical segment in parent_col overlaps
    /// unrelated mainline commits.
    #[test]
    fn repro_branch_edge_waypoints_preserved_when_thread_col_differs_from_parent_col() {
        // Topology matching the gitv repo:
        //   9 (HEAD)
        //   |
        //   8 (merge) ─┬─ 5 ─ 4 ─ 3 ─ 2
        //               └─ 7 ─ 6 ─┐
        //                       ┌─┘
        //                       1 (7688f42, branching point)
        //
        // OID 6 (5319794) is a side-branch child of OID 1 (7688f42).
        // Its parent-thread trace finds OID 1 at a column ≠ parent_col
        // for intermediate rows — the trivial filter must NOT drop these
        // waypoints.
        let commits = vec![
            make_commit(1, vec![], "7688f42"),
            make_commit(2, vec![1], "3754661"),
            make_commit(3, vec![2], "1ce4d2d"),
            make_commit(4, vec![3], "4ab690a"),
            make_commit(5, vec![4], "029ceed"),
            make_commit(6, vec![1], "5319794"),
            make_commit(7, vec![6], "2435ba9"),
            make_commit(8, vec![5, 7], "973fefb"),
            make_commit(9, vec![8], "HEAD"),
        ];
        let calc =
            GraphCalculator::new(commits, HashMap::new(), Vec::new(), GraphOptions::default());
        let layout = calc.calculate_layout();

        // Basic pass-through check
        let verify_errors = layout.verify();
        assert!(
            verify_errors.is_empty(),
            "verify() found {} pass-through error(s):\n{}",
            verify_errors.len(),
            verify_errors
                .iter()
                .take(10)
                .cloned()
                .collect::<Vec<_>>()
                .join("\n")
        );

        // Rendered crossing check (chamfer-aware)
        let crossings = find_rendered_crossings(&layout);
        assert!(
            crossings.is_empty(),
            "find_rendered_crossings() found {} error(s):\n{}",
            crossings.len(),
            crossings
                .iter()
                .take(10)
                .cloned()
                .collect::<Vec<_>>()
                .join("\n")
        );

        // Specifically check the edge from 5319794 (OID 6) to 7688f42 (OID 1):
        // it must have waypoints (the trivial filter must NOT have dropped them).
        let child_row = layout
            .nodes
            .iter()
            .find(|n| n.oid == make_oid(6))
            .unwrap()
            .row;
        let parent_row = layout
            .nodes
            .iter()
            .find(|n| n.oid == make_oid(1))
            .unwrap()
            .row;
        let edge_6_to_1 = layout
            .edges
            .iter()
            .find(|e| e.from_row == child_row && e.to_row == parent_row)
            .expect("edge 5319794→7688f42 must exist");

        assert!(
            !edge_6_to_1.waypoints.is_empty(),
            "edge 5319794→7688f42 must have waypoints (trivial filter preserves them \
             because thread col differs from parent col). \
             Edge: from_row={} from_col={} to_row={} to_col={}",
            edge_6_to_1.from_row,
            edge_6_to_1.from_col,
            edge_6_to_1.to_row,
            edge_6_to_1.to_col,
        );
    }

    // --- Regression: gitv repo fix-crash branch edge routing ---
    //
    // Reproduces the exact topology around f421e8d in the gitv repo:
    //
    //   f421e8d has 3 children:
    //    - 6ce557e (mainline, FPC from HEAD, col 0)
    //    - 4e14921 (fix-crash base, col 1)
    //    - a344ea2 (standalone branch, col 2)
    //
    //   The fix-crash chain (4e14921 → 52f6ce1 → b54e36f → 4f335b4 → 279ad6c)
    //   sits at rows 1-5. a344ea2 sits at row 6. f421e8d at row 7.
    //
    //   The edge from 4e14921 (row 5, col 1) to f421e8d (row 7, col 0) must
    //   NOT pass through a344ea2's node at (row 6, col 2). f421e8d's thread
    //   must be pre-inserted at column 0 at row 6 so the edge can route
    //   (5,1) → (6,0) → (7,0).

    #[test]
    fn regression_gitv_repo_fix_crash_edge_routing() {
        // Timestamps chosen to reproduce the real topological ordering:
        //   6ce557e (newest, processed first) > fix-crash chain > a344ea2 > f421e8d
        let make = |oid: u8, parents: Vec<u8>, time: i64| -> CommitInfo {
            CommitInfo {
                oid: make_oid(oid),
                short_oid: format!("{oid:02x}00000"),
                message: String::new(),
                summary: String::new(),
                author: Author {
                    name: "Test".into(),
                    email: "test@test.com".into(),
                },
                committer: Author {
                    name: "Test".into(),
                    email: "test@test.com".into(),
                },
                author_time: Utc.timestamp_opt(time, 0).single().unwrap(),
                commit_time: Utc.timestamp_opt(time, 0).single().unwrap(),
                parent_oids: parents.into_iter().map(make_oid).collect(),
                refs: Vec::new(),
            }
        };

        // OID assignments:
        //   oid1 = 3ad2005 (parent of f421e8d)
        //   oid2 = f421e8d (fork point)
        //   oid3 = 6ce557e (FPC child, mainline)
        //   oid4 = mainline chain intermediate (child of oid3)
        //   oid5 = HEAD (mainline tip, child of oid4)
        //   oid6 = 4e14921 (fix-crash base, child of f421e8d)
        //   oid7 = 52f6ce1 (fix-crash)
        //   oid8 = b54e36f (fix-crash)
        //   oid9 = 4f335b4 (fix-crash)
        //   oid10 = 279ad6c (fix-crash tip)
        //   oid11 = a344ea2 (standalone child of f421e8d)
        let commits = vec![
            // HEAD mainline (highest timestamps, processed first by topo sort)
            make(5, vec![4], 50), // HEAD
            make(4, vec![3], 45), // mainline intermediate
            // fix-crash chain (tip to base, timestamps decreasing)
            make(10, vec![9], 39), // fix-crash tip (like 279ad6c)
            make(9, vec![8], 38),  // (like 4f335b4)
            make(8, vec![7], 37),  // (like b54e36f)
            make(7, vec![6], 36),  // (like 52f6ce1)
            make(6, vec![2], 34),  // 4e14921 — fix-crash base, child of f421e8d
            make(11, vec![2], 33), // a344ea2 — standalone child of f421e8d
            // mainline chain continues
            make(3, vec![2], 40), // 6ce557e — FPC child of f421e8d
            // fork point and root
            make(2, vec![1], 32), // f421e8d
            make(1, vec![], 31),  // 3ad2005 — parent of f421e8d (root)
        ];

        let calc =
            GraphCalculator::new(commits, HashMap::new(), Vec::new(), GraphOptions::default());
        let layout = calc.calculate_layout();

        // Debug output
        println!("\n=== Nodes ===");
        for n in &layout.nodes {
            println!(
                "  oid{:02x} row={} col={}",
                n.oid.as_bytes()[0],
                n.row,
                n.column
            );
        }
        println!("=== Edges ===");
        for e in &layout.edges {
            let nfrom = layout.nodes.iter().find(|n| n.row == e.from_row).unwrap();
            let nto = layout.nodes.iter().find(|n| n.row == e.to_row).unwrap();
            println!(
                "  oid{:02x}→oid{:02x} ({},{}→{},{}) waypoints={:?} gap={:?}",
                nfrom.oid.as_bytes()[0],
                nto.oid.as_bytes()[0],
                e.from_row,
                e.from_col,
                e.to_row,
                e.to_col,
                e.waypoints,
                e.arrow_gap,
            );
        }

        // 1. No same-column pass-through
        let verify_errors = layout.verify();
        assert!(
            verify_errors.is_empty(),
            "verify() found {} error(s):\n{}",
            verify_errors.len(),
            verify_errors
                .iter()
                .take(10)
                .cloned()
                .collect::<Vec<_>>()
                .join("\n")
        );

        // 2. Mainline at col 0
        for &oid in &[
            make_oid(5),
            make_oid(4),
            make_oid(3),
            make_oid(2),
            make_oid(1),
        ] {
            let node = layout.nodes.iter().find(|n| n.oid == oid).unwrap();
            assert_eq!(
                node.column,
                0,
                "mainline oid{:02x} should be at column 0, got {}",
                oid.as_bytes()[0],
                node.column
            );
        }

        // 3. fix-crash chain at col > 0
        for &oid in &[
            make_oid(6),
            make_oid(7),
            make_oid(8),
            make_oid(9),
            make_oid(10),
        ] {
            let node = layout.nodes.iter().find(|n| n.oid == oid).unwrap();
            assert!(
                node.column > 0,
                "fix-crash chain oid{:02x} should be at column > 0, got {}",
                oid.as_bytes()[0],
                node.column
            );
        }

        // 4. a344ea2 at col > 0
        let a344ea2_node = layout.nodes.iter().find(|n| n.oid == make_oid(11)).unwrap();
        assert!(
            a344ea2_node.column > 0,
            "a344ea2 should be at column > 0, got {}",
            a344ea2_node.column
        );

        // 5. Edge from fix-crash base (oid6=4e14921) to f421e8d (oid2)
        // must route correctly WITHOUT overlapping a344ea2.
        // The horizontal-first chamfer handles this naturally:
        //   Seg 1: horizontal at row 7 from col 1 toward col 0 (no node at (7,0))
        //   Seg 2: diagonal (avoids a344ea2 at (8,1))
        //   Seg 3: vertical at col 0 to row 9
        let child_node = layout.nodes.iter().find(|n| n.oid == make_oid(6)).unwrap();
        let parent_node = layout.nodes.iter().find(|n| n.oid == make_oid(2)).unwrap();
        let edge_6_to_2 = layout
            .edges
            .iter()
            .find(|e| e.from_row == child_node.row && e.to_row == parent_node.row)
            .expect("edge 4e14921→f421e8d must exist");

        // Waypoints are dropped for non-gap cross-column edges — the chamfer handles routing
        assert!(
            edge_6_to_2.waypoints.is_empty(),
            "edge 4e14921→f421e8d should have no waypoints (chamfer handles routing)"
        );

        // 6. No rendered crossing (chamfer-aware check)
        let crossings = find_rendered_crossings(&layout);
        assert!(
            crossings.is_empty(),
            "find_rendered_crossings() found {} error(s):\n{}",
            crossings.len(),
            crossings
                .iter()
                .take(10)
                .cloned()
                .collect::<Vec<_>>()
                .join("\n")
        );
    }
}
