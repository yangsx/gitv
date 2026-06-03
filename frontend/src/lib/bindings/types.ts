export interface Oid {
	hex: string;
}

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

export interface Ref {
	Branch?: BranchRef;
	Tag?: TagRef;
	Remote?: RemoteRef;
	Head?: boolean;
}

export interface BranchRef {
	name: string;
	is_head: boolean;
	is_remote: boolean;
	upstream: string | null;
	ahead: number;
	behind: number;
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
}

export interface RepositoryInfo {
	path: string;
	head_branch: string | null;
	head_commit: string | null;
	is_bare: boolean;
}

export interface NodePosition {
	row: number;
	column: number;
	oid: string;
	color: string;
	is_dimmed: boolean;
}

export type EdgeType = 'Straight' | 'Branch' | 'Merge';

export interface Edge {
	from_row: number;
	from_col: number;
	to_row: number;
	to_col: number;
	edge_type: EdgeType;
	color: string;
	is_dimmed: boolean;
}

export interface StashMarker {
	row: number;
	column: number;
	stash_index: number;
	message: string;
}

export interface GraphLayout {
	nodes: NodePosition[];
	edges: Edge[];
	stash_markers: StashMarker[];
	total_rows: number;
	total_columns: number;
	orientation: 'TopToBottom' | 'BottomToTop';
}

export interface GraphOptions {
	hide_merges: boolean;
	orientation: 'TopToBottom' | 'BottomToTop';
	color_mode: 'ByBranch' | 'ByAuthor';
}

export interface CommitBatch {
	commits: CommitInfo[];
	has_more: boolean;
}

export interface RecentRepository {
	path: string;
	last_opened: string;
	branch: string | null;
}
