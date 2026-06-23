import { describe, it, expect } from 'vitest';
import {
	applyVirtualWorkingChanges,
	createVirtualCommitInfos,
	UNSTAGED_EDGE_COLOR,
	STAGED_EDGE_COLOR,
	VIRTUAL_NODE_COLOR
} from './virtual-working-changes';
import type {
	GraphLayout,
	NodePosition,
	Edge,
	WorkingChangesDiff,
	FileChange
} from '$lib/bindings/types';
import { STAGED_OID, UNSTAGED_OID } from '$lib/constants';

const WHITE: NodePosition['color'] = { r: 255, g: 255, b: 255, a: 255 };
const BLUE: Edge['color'] = { r: 100, g: 100, b: 255, a: 255 };

function node(
	oid: string,
	row: number,
	column: number,
	opts: Partial<NodePosition> = {}
): NodePosition {
	return {
		oid,
		row,
		column,
		is_merge: false,
		is_stash: false,
		color: WHITE,
		is_dimmed: false,
		is_highlighted: false,
		...opts
	};
}

function edge(
	fromRow: number,
	fromCol: number,
	toRow: number,
	toCol: number,
	opts: Partial<Edge> = {}
): Edge {
	return {
		from_row: fromRow,
		from_col: fromCol,
		to_row: toRow,
		to_col: toCol,
		edge_type: 'Straight',
		color: BLUE,
		is_dimmed: false,
		edge_style: 'Solid',
		...opts
	};
}

function layout(
	nodes: NodePosition[],
	edges: Edge[],
	extras: Partial<GraphLayout> = {}
): GraphLayout {
	return {
		nodes,
		edges,
		stash_markers: [],
		total_columns: 3,
		total_rows: nodes.length,
		orientation: 'TopToBottom',
		stash_commits: [],
		...extras
	};
}

function workingDiff(staged: number, unstaged: number): WorkingChangesDiff {
	const fakeFile = (n: number): FileChange => ({
		path: `file${n}.txt`,
		old_path: null,
		change_type: 'Modified',
		additions: 1,
		deletions: 0,
		is_binary: false,
		is_submodule: false
	});
	return {
		staged: Array.from({ length: staged }, (_, i) => fakeFile(i)),
		unstaged: Array.from({ length: unstaged }, (_, i) => fakeFile(i + staged))
	};
}

const emptyDiff: WorkingChangesDiff = { staged: [], unstaged: [] };

function findNode(l: GraphLayout, oid: string): NodePosition | undefined {
	return l.nodes.find((n) => n.oid === oid);
}

function apply(
	layout: GraphLayout | null,
	diff: WorkingChangesDiff | null,
	headOid?: string | null
): GraphLayout {
	const r = applyVirtualWorkingChanges(layout, diff, headOid);
	expect(r).not.toBeNull();
	return r as GraphLayout;
}

describe('applyVirtualWorkingChanges', () => {
	it('returns null when graphLayout is null', () => {
		expect(applyVirtualWorkingChanges(null, emptyDiff)).toBeNull();
	});

	it('returns layout unchanged when workingChangesDiff is null', () => {
		const l = layout([node('a', 0, 0)], []);
		expect(applyVirtualWorkingChanges(l, null)).toBe(l);
	});

	it('returns layout unchanged when no staged or unstaged changes', () => {
		const l = layout([node('a', 0, 0)], []);
		expect(applyVirtualWorkingChanges(l, emptyDiff)).toBe(l);
	});

	it('creates unstaged virtual node at row 0, staged at row 1 when both present', () => {
		const l = layout([node('head', 0, 0)], []);
		const result = apply(l, workingDiff(1, 1));

		const unstaged = findNode(result, UNSTAGED_OID);
		const staged = findNode(result, STAGED_OID);
		expect(unstaged).toBeDefined();
		expect(staged).toBeDefined();
		expect(unstaged!.row).toBe(0);
		expect(staged!.row).toBe(1);
		expect(unstaged!.column).toBe(0);
		expect(staged!.column).toBe(0);
		expect(unstaged!.color).toEqual(VIRTUAL_NODE_COLOR);
		expect(staged!.color).toEqual(VIRTUAL_NODE_COLOR);
	});

	it('creates only unstaged virtual node at row 0 when no staged changes', () => {
		const l = layout([node('head', 0, 0)], []);
		const result = apply(l, workingDiff(0, 1));

		expect(findNode(result, UNSTAGED_OID)).toBeDefined();
		expect(findNode(result, STAGED_OID)).toBeUndefined();
	});

	it('creates only staged virtual node at row 0 when no unstaged changes', () => {
		const l = layout([node('head', 0, 0)], []);
		const result = apply(l, workingDiff(1, 0));

		expect(findNode(result, STAGED_OID)).toBeDefined();
		expect(findNode(result, UNSTAGED_OID)).toBeUndefined();
	});

	it('connects virtual edges to HEAD commit by OID when headOid is provided', () => {
		const l = layout(
			[node('older', 0, 1), node('head', 1, 0), node('other-tip', 2, 2)],
			[edge(0, 1, 1, 0), edge(1, 0, 2, 2)]
		);
		const result = apply(l, workingDiff(1, 1), 'head');

		const unstagedEdge = result.edges.find((e) => e.from_row === 0);
		const stagedEdge = result.edges.find((e) => e.from_row === 1);
		expect(unstagedEdge).toBeDefined();
		expect(stagedEdge).toBeDefined();
		expect(unstagedEdge!.to_row).toBe(3);
		expect(unstagedEdge!.to_col).toBe(0);
		expect(stagedEdge!.to_row).toBe(3);
		expect(stagedEdge!.to_col).toBe(0);
	});

	it('connects to HEAD even when HEAD is not the first node in layout — regression test', () => {
		const l = layout(
			[node('newer-feature', 0, 0), node('head', 1, 0), node('older-main', 2, 0)],
			[edge(0, 0, 1, 0), edge(1, 0, 2, 0)]
		);
		const result = apply(l, workingDiff(1, 1), 'head');

		const unstagedEdge = result.edges.find((e) => e.from_row === 0 && e.from_col === 0);
		expect(unstagedEdge).toBeDefined();
		expect(unstagedEdge!.to_row).toBe(1 + 2);
		expect(unstagedEdge!.to_col).toBe(0);

		const headNode = result.nodes.find((n) => n.oid === 'head');
		expect(headNode!.row).toBe(1 + 2);
	});

	it('falls back to nodes[0] when headOid is null', () => {
		const l = layout([node('first', 0, 0), node('second', 1, 1)], [edge(0, 0, 1, 1)]);
		const result = apply(l, workingDiff(1, 0), null);

		const firstNodeNewRow = 0 + 1;
		const stagedEdge = result.edges.find((e) => e.to_row === firstNodeNewRow && e.to_col === 0);
		expect(stagedEdge).toBeDefined();
		expect(stagedEdge!.to_row).toBe(firstNodeNewRow);
		expect(stagedEdge!.to_col).toBe(0);
	});

	it('omits virtual edges when headOid is not found in layout', () => {
		const l = layout([node('a', 0, 0), node('b', 1, 0)], [edge(0, 0, 1, 0)]);
		const result = apply(l, workingDiff(0, 1), 'nonexistent');

		const virtualEdge = result.edges.find((e) => e.from_row < virtualNodes(result).length);
		expect(virtualEdge).toBeUndefined();
	});

	it('falls back to nodes[0] when headOid is undefined', () => {
		const l = layout([node('first', 0, 0)], []);
		const result = apply(l, workingDiff(1, 0));

		const firstNewRow = 0 + 1;
		const stagedEdge = result.edges.find((e) => e.to_row === firstNewRow && e.to_col === 0);
		expect(stagedEdge).toBeDefined();
	});

	it('uses orange edge for unstaged and green edge for staged', () => {
		const l = layout([node('head', 0, 0)], []);
		const result = apply(l, workingDiff(1, 1), 'head');

		const unstagedEdge = result.edges.find((e) => e.from_row === 0);
		const stagedEdge = result.edges.find((e) => e.from_row === 1);
		expect(unstagedEdge!.color).toEqual(UNSTAGED_EDGE_COLOR);
		expect(stagedEdge!.color).toEqual(STAGED_EDGE_COLOR);
	});

	it('offsets real node rows by virtual count', () => {
		const l = layout([node('head', 0, 0), node('parent', 1, 0)], [edge(0, 0, 1, 0)]);
		const result = apply(l, workingDiff(1, 1), 'head');

		expect(findNode(result, 'head')!.row).toBe(0 + 2);
		expect(findNode(result, 'parent')!.row).toBe(1 + 2);
	});

	it('offsets real edge rows by virtual count', () => {
		const l = layout([node('head', 0, 0), node('parent', 1, 0)], [edge(0, 0, 1, 0)]);
		const result = apply(l, workingDiff(1, 1), 'head');

		const parentEdge = result.edges.find((e) => e.from_row === 2);
		expect(parentEdge).toBeDefined();
		expect(parentEdge!.to_row).toBe(3);
	});

	it('offsets stash_marker rows by virtual count', () => {
		const l = layout([node('head', 0, 0)], [], {
			stash_markers: [
				{
					row: 0,
					column: 0,
					stash_index: 0,
					stash_oid: 'stash1',
					parent_oid: 'head',
					message: 'WIP'
				}
			]
		});
		const result = apply(l, workingDiff(1, 0), 'head');

		expect(result.stash_markers[0].row).toBe(0 + 1);
	});

	it('updates total_rows by virtual count', () => {
		const l = layout([node('head', 0, 0)], [], { total_rows: 1 });
		const result = apply(l, workingDiff(1, 0), 'head');

		expect(result.total_rows).toBe(1 + 1);
	});

	it('preserves existing layout properties', () => {
		const l = layout([node('head', 0, 0)], [], {
			total_columns: 5,
			orientation: 'BottomToTop' as const,
			stash_commits: [
				{
					oid: 's1',
					short_oid: '',
					message: '',
					summary: '',
					author: { name: '', email: '' },
					committer: { name: '', email: '' },
					author_time: '',
					commit_time: '',
					parent_oids: [],
					refs: []
				}
			]
		});
		const result = apply(l, workingDiff(1, 0), 'head');

		expect(result.total_columns).toBe(5);
		expect(result.orientation).toBe('BottomToTop');
		expect(result.stash_commits).toHaveLength(1);
	});

	it('handles empty nodes array in layout', () => {
		const l = layout([], []);
		const result = apply(l, workingDiff(1, 0));

		expect(findNode(result, STAGED_OID)).toBeDefined();
	});

	it('preserves node properties other than row when shifting', () => {
		const l = layout(
			[
				node('head', 0, 0, {
					is_merge: true,
					is_stash: false,
					color: { r: 1, g: 2, b: 3, a: 255 },
					is_dimmed: true,
					is_highlighted: true
				})
			],
			[]
		);
		const result = apply(l, workingDiff(1, 0), 'head');

		const headNode = findNode(result, 'head')!;
		expect(headNode.is_merge).toBe(true);
		expect(headNode.color).toEqual({ r: 1, g: 2, b: 3, a: 255 });
		expect(headNode.is_dimmed).toBe(true);
		expect(headNode.is_highlighted).toBe(true);
	});
});

describe('createVirtualCommitInfos', () => {
	const t = (key: string) => key;

	it('returns empty array when diff is null', () => {
		expect(createVirtualCommitInfos(null, t)).toEqual([]);
	});

	it('returns empty array when no staged or unstaged changes', () => {
		expect(createVirtualCommitInfos(emptyDiff, t)).toEqual([]);
	});

	it('creates unstaged CommitInfo when only unstaged changes exist', () => {
		const result = createVirtualCommitInfos(workingDiff(0, 1), t);

		expect(result).toHaveLength(1);
		expect(result[0].oid).toBe(UNSTAGED_OID);
		expect(result[0].summary).toBe('page.unstaged');
	});

	it('creates staged CommitInfo when only staged changes exist', () => {
		const result = createVirtualCommitInfos(workingDiff(1, 0), t);

		expect(result).toHaveLength(1);
		expect(result[0].oid).toBe(STAGED_OID);
		expect(result[0].summary).toBe('page.staged');
	});

	it('creates both virtual commits when both staged and unstaged exist', () => {
		const result = createVirtualCommitInfos(workingDiff(1, 1), t);

		expect(result).toHaveLength(2);
		expect(result[0].oid).toBe(UNSTAGED_OID);
		expect(result[1].oid).toBe(STAGED_OID);
	});

	it('calls translate function with correct keys', () => {
		const calls: string[] = [];
		const capture = (key: string) => {
			calls.push(key);
			return key;
		};
		createVirtualCommitInfos(workingDiff(1, 1), capture);

		expect(calls).toContain('page.unstaged');
		expect(calls).toContain('page.staged');
	});

	it('populates correct virtual commit fields', () => {
		const result = createVirtualCommitInfos(workingDiff(1, 0), t);

		expect(result[0].short_oid).toBe('');
		expect(result[0].message).toBe('page.staged');
		expect(result[0].message).toBe(result[0].summary);
		expect(result[0].author).toEqual({ name: '', email: '' });
		expect(result[0].author_time).toBe('');
		expect(result[0].commit_time).toBe('');
		expect(result[0].parent_oids).toEqual([]);
		expect(result[0].refs).toEqual([]);
	});
});

function virtualNodes(l: GraphLayout): NodePosition[] {
	return l.nodes.filter((n) => n.oid === STAGED_OID || n.oid === UNSTAGED_OID);
}
