export interface Author {
	name: string;
	email: string;
}

export interface CommitInfo {
	oid: string;
	short_oid: string;
	message: string;
	summary: string;
	author: Author;
	committer: Author;
	author_time: string;
	commit_time: string;
	parent_oids: string[];
	refs: Ref[];
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
}

export interface CommitBatch {
	commits: CommitInfo[];
	has_more: boolean;
}

export interface SearchQuery {
	text?: string;
	use_regex: boolean;
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

export interface SearchResult {
	commit_oid: string;
	match_type: 'Message' | 'Sha' | 'Author';
	highlights: Highlight[];
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
	| { Context: { content: string; old_line: number; new_line: number } }
	| { Addition: { content: string; new_line: number } }
	| { Deletion: { content: string; old_line: number } }
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
	summary: string;
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

export interface StashSplitDiff {
	staged: FileDiff;
	unstaged: FileDiff;
}

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
	theme: 'dark' | 'light';
	font_size: number;
	high_contrast: boolean;
	language: 'en' | 'zh-cn';
}
