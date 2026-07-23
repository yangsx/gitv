export interface Author {
	name: string;
	email: string;
}

export interface CommitInfo {
	oid: string;
	short_oid: string;
	message: string;
	author: Author;
	committer: Author;
	author_time: string;
	commit_time: string;
	parent_oids: string[];
	refs: Ref[];
}

export function commitSummary(commit: CommitInfo): string {
	return commit.message.split('\n')[0] || '';
}

export type Ref = { Branch?: BranchRef; Tag?: TagRef; Remote?: RemoteRef };

export interface BranchRef {
	name: string;
	oid: string;
	is_head: boolean;
	is_remote: boolean;
	upstream: string | null;
	ahead: number;
	behind: number;
	is_merged: boolean;
}

export interface TagRef {
	name: string;
	oid: string;
	annotation: TagAnnotation | null;
}

export interface TagAnnotation {
	tagger: Author;
	message: string;
	time: string;
}

export interface RemoteRef {
	name: string;
	remote: string;
	oid: string;
}

export interface RepositoryInfo {
	path: string;
	head_branch: string | null;
	head_commit: string | null;
	is_bare: boolean;
}

export interface Color {
	r: number;
	g: number;
	b: number;
	a: number;
}

export interface NodePosition {
	oid: string;
	row: number;
	column: number;
	is_merge: boolean;
	is_stash: boolean;
	color: Color;
	is_dimmed: boolean;
	is_highlighted: boolean;
}

export type EdgeType = 'Straight' | 'Branch' | 'Merge';
export type EdgeStyle = 'Solid' | 'Dashed' | 'Dotted';

export interface Edge {
	from_row: number;
	from_col: number;
	to_row: number;
	to_col: number;
	edge_type: EdgeType;
	color: Color;
	is_dimmed: boolean;
	edge_style: EdgeStyle;
	/** [row, col] pairs at direction changes — render as connected segments */
	waypoints: [number, number][];
	/** When non-null, the thread was removed from the rowidlist.
	 * Contains [seg1_end_row, seg2_start_row] — the gap boundaries.
	 * The renderer draws two segments with arrowheads at these boundaries. */
	arrow_gap?: [number, number] | null;
}

export interface StashMarker {
	row: number;
	column: number;
	stash_index: number;
	stash_oid: string;
	parent_oid: string;
	message: string;
}

export interface GraphLayout {
	nodes: NodePosition[];
	stash_markers: StashMarker[];
	edges: Edge[];
	total_columns: number;
	orientation: 'TopToBottom' | 'BottomToTop';
	total_rows: number;
	stash_commits: CommitInfo[];
	row_max_column: number[];
}

export interface CommitBatch {
	commits: CommitInfo[];
	has_more: boolean;
}

export interface SearchQuery {
	text?: string;
	use_regex: boolean;
	search_patch: boolean;
	sha_prefix?: string;
	author?: string;
	date_range?: DateRange;
	file_path?: string;
	combine_mode: 'And' | 'Or';
}

export interface DateRange {
	from?: string;
	to?: string;
}

export type MatchType = 'Message' | 'Sha' | 'Author' | 'Patch';

export interface SearchResult {
	commit_oid: string;
	match_type: MatchType;
	highlights: Highlight[];
	patch_matches: PatchMatchLocation[];
}

export interface PatchMatchLocation {
	file_path: string;
	old_line: number | null;
	new_line: number | null;
	matched_text: string;
}

export interface SearchResponse {
	results: SearchResult[];
	patch_search_id: number | null;
	patch_search_total: number | null;
}

export interface PatchSearchProgress {
	search_id: number;
	checked: number;
	total: number;
	matches: SearchResult[];
}

export interface PatchSearchComplete {
	search_id: number;
	total_checked: number;
}

export interface PatchSearchError {
	search_id: number;
	message: string;
}

export interface Highlight {
	start: number;
	length: number;
}

export interface RecentRepository {
	path: string;
	name: string;
	last_opened: string;
}

export interface CommitDetails {
	info: CommitInfo;
	tree_oid: string;
	signature: string | null;
	changed_files: FileChange[];
	body: string | null;
}

export interface FileChange {
	path: string;
	old_path: string | null;
	change_type: ChangeType;
	additions: number;
	deletions: number;
	is_binary: boolean;
	is_submodule: boolean;
	/** For merge commits: hex OID of the parent to diff against, null for first parent */
	diff_parent?: string | null;
}

export interface FileLineStats {
	path: string;
	additions: number;
	deletions: number;
}

export type ChangeType =
	| 'Added'
	| 'Deleted'
	| 'Modified'
	| 'Renamed'
	| 'Copied'
	| 'SubmoduleUpdated';

export interface DiffSummary {
	files: FileDiffSummary[];
	stats: DiffStats;
}

export interface FileDiffSummary {
	path: string;
	old_path: string | null;
	change_type: ChangeType;
	additions: number;
	deletions: number;
	is_binary: boolean;
	is_submodule: boolean;
}

export interface DiffStats {
	files_changed: number;
	additions: number;
	deletions: number;
}

export interface FileDiff {
	path: string;
	old_path: string | null;
	hunks: Hunk[];
	is_binary: boolean;
	is_submodule: boolean;
	old_size: number | null;
	new_size: number | null;
	truncated_at: number | null;
}

export interface Hunk {
	old_start: number;
	old_count: number;
	new_start: number;
	new_count: number;
	lines: DiffLine[];
}

export type DiffLine =
	| {
			Context: {
				content: string;
				old_line: number;
				new_line: number;
				combined_prefix?: string | null;
			};
	  }
	| { Addition: { content: string; new_line: number; combined_prefix?: string | null } }
	| { Deletion: { content: string; old_line: number; combined_prefix?: string | null } }
	| {
			WordDiff: {
				content: string;
				old_line: number;
				new_line: number;
				segments: WordDiffSegment[];
			};
	  };

export interface WordDiffSegment {
	text: string;
	kind: WordDiffKind;
}

export type WordDiffKind = 'Unchanged' | 'Added' | 'Removed';

export interface FileTreeNode {
	name: string;
	path: string;
	node_type: FileNodeType;
	children: FileTreeNode[];
	size: number | null;
}

export type FileNodeType = 'File' | 'Directory' | 'Symlink' | 'Submodule';

export interface FileHistoryEntry {
	commit_oid: string;
	path: string;
	old_path: string | null;
	message: string;
	author: Author;
	time: string;
}

export interface ReflogEntry {
	oid: string;
	old_oid: string | null;
	ref_name: string;
	message: string;
	author: Author;
	time: string;
}

export interface StashEntry {
	index: number;
	oid: string;
	parent_oid: string;
	message: string;
	author: Author;
	time: string;
	file_summary: StashFileSummary[];
}

export interface StashFileSummary {
	path: string;
	change_type: StashChangeType;
}

export type StashChangeType = 'Added' | 'Modified' | 'Deleted';

export interface Blame {
	file_path: string;
	lines: BlameLine[];
}

export interface BlameLine {
	line_number: number;
	content: string;
	commit_oid: string;
	author: Author;
	time: string;
}

export interface SavedSearch {
	id: string;
	name: string;
	query: string;
	created_at: string;
}

export interface LoadTiming {
	load_commits_ms: number;
	graph_calc_ms: number;
	refs_ms: number;
	working_changes_ms: number;
	total_ms: number;
	arc_topo_sort_ms: number;
	arc_insert_ms: number;
	arc_update_rows_ms: number;
	arc_disporder_ms: number;
	arc_ordertoken_ms: number;
	arc_fix_reversal_calls: number;
	arc_renumber_arc_calls: number;
	arc_split_arc_calls: number;
	assign_columns_ms: number;
	optimize_rows_ms: number;
	rebuild_edges_ms: number;
	fix_edge_pass_ms: number;
	sibling_walk_total: number;
	sibling_walk_count: number;
}

export interface InitialData {
	repo_info: RepositoryInfo;
	commits: CommitInfo[];
	total_commit_count: number;
	graph_layout: GraphLayout;
	refs: Ref[];
	working_changes: WorkingChangesDiff | null;
	timing: LoadTiming;
	warnings: string[];
}

export interface WorkingChangesDiff {
	staged: FileChange[];
	unstaged: FileChange[];
}

export interface AppPreferences {
	graph_color_mode: 'by-branch' | 'by-author';
	graph_hide_merges: boolean;
	graph_orientation: 'top-to-bottom' | 'bottom-to-top';
	graph_palette: 'default' | 'deuteranopia' | 'protanopia' | 'tritanopia';
	diff_mode: 'normal' | 'word-diff' | 'stat-only';
	diff_whitespace: 'none' | 'ignore-space-change' | 'ignore-all-space' | 'ignore-blank-lines';
	diff_view_mode: 'unified' | 'side-by-side';
	theme: 'dark' | 'light' | 'auto';
	font_size: number;
	high_contrast: boolean;
	language: string;
	arrow_gap_threshold: number;
}

export interface PropertyCheckResult {
	name: string;
	violation_count: number;
	sample: string[];
}

export interface SelfTestResult {
	repo_path: string;
	repo_name: string;
	timing_ms: number;
	node_count: number;
	edge_count: number;
	total_columns: number;
	max_concurrent_threads: number;
	column_waste: number;
	total_waypoints: number;
	max_waypoints_per_edge: number;
	straight_edges: number;
	branch_edges: number;
	merge_edges: number;
	arrow_gap_count: number;
	column_shift_histogram: string;
	row_thread_histogram: string;
	total_commits: number;
	merge_count: number;
	branching_factor_histogram: number[];
	longest_chain: number;
	fork_point_count: number;
	property_checks: PropertyCheckResult[];
	hide_merges_property_checks: PropertyCheckResult[];
}
