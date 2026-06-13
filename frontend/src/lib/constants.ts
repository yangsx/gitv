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
export const GRAPH_EDGE_HIT_TOLERANCE = 6;
