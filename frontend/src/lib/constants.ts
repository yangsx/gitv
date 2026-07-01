export const STAGED_OID = '__staged__';
export const UNSTAGED_OID = '__unstaged__';
export const VIRTUAL_OIDS = new Set([STAGED_OID, UNSTAGED_OID]);

export const CHANGE_COLORS: Record<string, string> = {
	Added: 'text-green-400',
	Deleted: 'text-red-400',
	Modified: 'text-yellow-400',
	Renamed: 'text-blue-400',
	Copied: 'text-purple-400',
	SubmoduleUpdated: 'text-orange-400'
};

export const CHANGE_LETTERS: Record<string, string> = {
	Added: 'A',
	Deleted: 'D',
	Modified: 'M',
	Renamed: 'R',
	Copied: 'C',
	SubmoduleUpdated: 'S'
};

export const GRAPH_PADDING_LEFT = 12;
export const GRAPH_LANE_WIDTH = 24;
export const GRAPH_MAX_VIEWPORT_RATIO = 0.5;
export const GRAPH_EDGE_HIT_TOLERANCE = 6;

// ── Column widths (flowing-text layout) ───────────────────────
export const HASH_COLUMN_WIDTH = 68;
export const AUTHOR_COLUMN_WIDTH = 120;
export const DATE_COLUMN_WIDTH = 150;
export const GRAPH_GAP = 12;
export const MIN_TEXT_WIDTH = 200;

// ── Diff loading ──────────────────────────────────────────────
export const DIFF_FILE_LIMIT = 100;
export const DIFF_CONCURRENCY = 4;

// ── Reflog display ────────────────────────────────────────────
export const REFLOG_DISPLAY_LIMIT = 100;
export const REFLOG_MSG_MAX_LEN = 50;
export const REFLOG_OID_DISPLAY_LEN = 7;
