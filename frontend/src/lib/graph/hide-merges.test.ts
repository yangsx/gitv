import { describe, it, expect } from 'vitest';
import { computeHideMergeLayout } from './hide-merges';
import type {
	GraphLayout,
	NodePosition,
	Edge,
	EdgeType,
	EdgeStyle,
	Color
} from '$lib/bindings/types';

const WHITE: Color = { r: 255, g: 255, b: 255, a: 255 };
const BLUE: Color = { r: 100, g: 100, b: 255, a: 255 };
const GREEN: Color = { r: 100, g: 255, b: 100, a: 255 };

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
	edgeType: EdgeType,
	opts: Partial<Edge> = {}
): Edge {
	return {
		from_row: fromRow,
		from_col: fromCol,
		to_row: toRow,
		to_col: toCol,
		edge_type: edgeType,
		color: BLUE,
		is_dimmed: false,
		edge_style: 'Solid' as EdgeStyle,
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

function incomingEdges(result: GraphLayout, nodeRow: number): Edge[] {
	return result.edges.filter((e) => e.to_row === nodeRow);
}

function outgoingEdges(result: GraphLayout, nodeRow: number): Edge[] {
	return result.edges.filter((e) => e.from_row === nodeRow);
}

function assertNoNewLeaves(input: GraphLayout, result: GraphLayout, label: string) {
	for (const inputNode of input.nodes) {
		const resultNode = result.nodes.find((n) => n.oid === inputNode.oid);
		if (!resultNode) continue;
		const hadOutgoing = input.edges.some((e) => e.from_row === inputNode.row);
		if (!hadOutgoing) continue;
		const hasOutgoing = result.edges.some((e) => e.from_row === resultNode.row);
		expect(hasOutgoing, `${label}: node ${inputNode.oid} became a leaf`).toBe(true);
	}
}

describe('computeHideMergeLayout', () => {
	it('returns null when layout is null', () => {
		expect(computeHideMergeLayout(null, true)).toBeNull();
	});

	it('returns layout unchanged when hideMerges is false', () => {
		const l = layout([node('a', 0, 0)], []);
		expect(computeHideMergeLayout(l, false)).toBe(l);
	});

	it('returns layout unchanged when no merge nodes exist', () => {
		const l = layout([node('a', 0, 0), node('b', 1, 0)], [edge(0, 0, 1, 0, 'Straight')]);
		expect(computeHideMergeLayout(l, true)).toBe(l);
	});

	it('removes a single merge node', () => {
		const l = layout(
			[node('a', 0, 0), node('merge', 1, 0, { is_merge: true }), node('b', 2, 0), node('c', 3, 0)],
			[
				edge(0, 0, 1, 0, 'Straight'),
				edge(1, 0, 2, 0, 'Straight'),
				edge(1, 1, 3, 0, 'Merge'),
				edge(2, 0, 3, 0, 'Straight')
			]
		);

		const result = computeHideMergeLayout(l, true)!;

		expect(result.nodes).toHaveLength(3);
		expect(result.nodes.every((n) => !n.is_merge)).toBe(true);
		expect(result.total_rows).toBe(3);
	});

	it('remaps rows after merge removal', () => {
		const l = layout(
			[node('a', 0, 0), node('merge', 1, 0, { is_merge: true }), node('b', 2, 0), node('c', 3, 0)],
			[
				edge(0, 0, 1, 0, 'Straight'),
				edge(1, 0, 3, 0, 'Straight'),
				edge(1, 1, 2, 0, 'Merge'),
				edge(2, 0, 3, 0, 'Straight')
			]
		);

		const result = computeHideMergeLayout(l, true)!;

		const oids = result.nodes.map((n) => n.oid);
		expect(oids).toEqual(['a', 'b', 'c']);
		expect(result.nodes[0].row).toBe(0);
		expect(result.nodes[1].row).toBe(1);
		expect(result.nodes[2].row).toBe(2);
	});

	it('rewires child to first-parent ancestor when children < ancestors', () => {
		const l = layout(
			[
				node('child', 0, 0),
				node('merge', 1, 0, { is_merge: true }),
				node('parent', 2, 0),
				node('branch', 3, 1)
			],
			[edge(0, 0, 1, 0, 'Straight'), edge(1, 0, 2, 0, 'Straight'), edge(1, 1, 3, 1, 'Merge')]
		);

		const result = computeHideMergeLayout(l, true)!;

		expect(result.nodes).toHaveLength(3);
		const parentIdx = result.nodes.findIndex((n) => n.oid === 'parent');
		const toParent = result.edges.find((e) => e.from_row === 0 && e.to_row === parentIdx);
		expect(toParent).toBeDefined();
		expect(outgoingEdges(result, 0)).toHaveLength(2);
	});

	it('preserves merge with 2+ children (merge+branch node)', () => {
		const l = layout(
			[
				node('c1', 0, 0),
				node('c2', 1, 1),
				node('merge', 2, 0, { is_merge: true }),
				node('parent', 3, 0),
				node('branch', 4, 1)
			],
			[
				edge(0, 0, 2, 0, 'Straight'),
				edge(1, 1, 2, 0, 'Straight'),
				edge(2, 0, 3, 0, 'Straight'),
				edge(2, 1, 4, 1, 'Merge')
			]
		);

		const result = computeHideMergeLayout(l, true)!;

		expect(result.nodes).toHaveLength(5);
		const mergeNode = result.nodes.find((n) => n.oid === 'merge');
		expect(mergeNode).toBeDefined();
		expect(mergeNode!.is_merge).toBe(true);

		const c1Row = result.nodes.findIndex((n) => n.oid === 'c1');
		const c2Row = result.nodes.findIndex((n) => n.oid === 'c2');
		const mergeRow = result.nodes.findIndex((n) => n.oid === 'merge');
		const parentRow = result.nodes.findIndex((n) => n.oid === 'parent');
		const branchRow = result.nodes.findIndex((n) => n.oid === 'branch');

		expect(result.edges.find((e) => e.from_row === c1Row && e.to_row === mergeRow)).toBeDefined();
		expect(result.edges.find((e) => e.from_row === c2Row && e.to_row === mergeRow)).toBeDefined();
		expect(
			result.edges.find((e) => e.from_row === mergeRow && e.to_row === parentRow)
		).toBeDefined();
		expect(
			result.edges.find((e) => e.from_row === mergeRow && e.to_row === branchRow)
		).toBeDefined();
	});

	it('handles consecutive merges', () => {
		const l = layout(
			[
				node('child', 0, 0),
				node('merge1', 1, 0, { is_merge: true }),
				node('merge2', 2, 0, { is_merge: true }),
				node('ancestor', 3, 0)
			],
			[edge(0, 0, 1, 0, 'Straight'), edge(1, 0, 2, 0, 'Straight'), edge(2, 0, 3, 0, 'Straight')]
		);

		const result = computeHideMergeLayout(l, true)!;

		expect(result.nodes).toHaveLength(2);
		expect(result.nodes.map((n) => n.oid)).toEqual(['child', 'ancestor']);
		expect(result.edges).toHaveLength(1);
		expect(result.edges[0].from_row).toBe(0);
		expect(result.edges[0].to_row).toBe(1);
	});

	it('handles branch tip that is also a merge being removed', () => {
		const l = layout(
			[
				node('child', 0, 0),
				node('merge1', 1, 0, { is_merge: true }),
				node('merge2', 2, 1, { is_merge: true }),
				node('ancestor1', 3, 0),
				node('ancestor2', 4, 1)
			],
			[
				edge(0, 0, 1, 0, 'Straight'),
				edge(1, 0, 3, 0, 'Straight'),
				edge(1, 1, 2, 1, 'Merge'),
				edge(2, 1, 4, 1, 'Straight')
			]
		);

		const result = computeHideMergeLayout(l, true)!;

		expect(result.nodes).toHaveLength(3);
		expect(result.nodes.map((n) => n.oid)).toEqual(['child', 'ancestor1', 'ancestor2']);
	});

	it('deduplicates edges to the same ancestor', () => {
		const l = layout(
			[node('child', 0, 0), node('merge', 1, 0, { is_merge: true }), node('target', 2, 1)],
			[edge(0, 0, 1, 0, 'Straight'), edge(1, 0, 2, 1, 'Straight'), edge(1, 1, 2, 1, 'Merge')]
		);

		const result = computeHideMergeLayout(l, true)!;

		const toTarget = result.edges.filter((e) => e.to_row === 1);
		expect(toTarget).toHaveLength(1);
	});

	it('preserves stash nodes even when is_merge is true', () => {
		const l = layout(
			[node('a', 0, 0), node('stash', 1, 0, { is_merge: true, is_stash: true }), node('b', 2, 0)],
			[edge(0, 0, 1, 0, 'Straight'), edge(1, 0, 2, 0, 'Straight')]
		);

		const result = computeHideMergeLayout(l, true)!;

		expect(result.nodes).toHaveLength(3);
		expect(result.nodes.find((n) => n.oid === 'stash')).toBeDefined();
	});

	it('removes stash_markers on merge rows', () => {
		const l = layout(
			[node('a', 0, 0), node('merge', 1, 0, { is_merge: true })],
			[edge(0, 0, 1, 0, 'Straight')],
			{
				stash_markers: [
					{
						row: 1,
						column: 0,
						stash_index: 0,
						stash_oid: 's1',
						parent_oid: 'a',
						message: 'stash'
					}
				]
			}
		);

		const result = computeHideMergeLayout(l, true)!;

		expect(result.stash_markers).toHaveLength(0);
	});

	it('preserves non-merge edges unchanged', () => {
		const l = layout(
			[node('a', 0, 0), node('merge', 1, 0, { is_merge: true }), node('b', 2, 0)],
			[
				edge(0, 0, 1, 0, 'Straight'),
				edge(1, 0, 2, 0, 'Straight'),
				edge(0, 0, 2, 0, 'Straight', { color: GREEN, is_dimmed: true, edge_style: 'Dashed' })
			]
		);

		const result = computeHideMergeLayout(l, true)!;

		const preserved = result.edges.find(
			(e) => e.color === GREEN && e.is_dimmed === true && e.edge_style === 'Dashed'
		);
		expect(preserved).toBeDefined();
		expect(preserved!.from_row).toBe(0);
		expect(preserved!.to_row).toBe(1);
	});

	it('handles multiple merges with interleaved branches', () => {
		const l = layout(
			[
				node('head', 0, 0),
				node('m1', 1, 0, { is_merge: true }),
				node('m2', 2, 0, { is_merge: true }),
				node('b1', 3, 1),
				node('b2', 4, 2),
				node('root', 5, 0)
			],
			[
				edge(0, 0, 1, 0, 'Straight'),
				edge(1, 0, 2, 0, 'Straight'),
				edge(1, 1, 3, 1, 'Merge'),
				edge(2, 0, 5, 0, 'Straight'),
				edge(2, 2, 4, 2, 'Merge'),
				edge(3, 1, 5, 0, 'Straight'),
				edge(4, 2, 5, 0, 'Straight')
			]
		);

		const result = computeHideMergeLayout(l, true)!;

		expect(result.nodes).toHaveLength(4);
		expect(result.nodes.map((n) => n.oid)).toEqual(['head', 'b1', 'b2', 'root']);
		expect(result.total_rows).toBe(4);
	});

	describe('invariant 1: incoming/outgoing counts do not increase', () => {
		it('node with incoming from merge and elsewhere preserves incoming count', () => {
			const l = layout(
				[
					node('c1', 0, 0),
					node('c2', 1, 1),
					node('merge', 2, 0, { is_merge: true }),
					node('target', 3, 0),
					node('other', 4, 1)
				],
				[
					edge(0, 0, 2, 0, 'Straight'),
					edge(1, 1, 2, 0, 'Straight'),
					edge(2, 0, 3, 0, 'Straight'),
					edge(2, 1, 4, 1, 'Merge'),
					edge(3, 0, 4, 1, 'Straight')
				]
			);

			const showIncoming = l.edges.filter((e) => e.to_row === 4).length;

			const result = computeHideMergeLayout(l, true)!;

			const targetRow = result.nodes.find((n) => n.oid === 'other')!.row;
			const hideIncoming = incomingEdges(result, targetRow).length;
			expect(hideIncoming).toBeLessThanOrEqual(showIncoming);
		});

		it('child preserves outgoing count (1 outgoing stays 1)', () => {
			const l = layout(
				[
					node('child', 0, 0),
					node('merge', 1, 0, { is_merge: true }),
					node('parent', 2, 0),
					node('branch', 3, 1)
				],
				[edge(0, 0, 1, 0, 'Straight'), edge(1, 0, 2, 0, 'Straight'), edge(1, 1, 3, 1, 'Merge')]
			);

			const result = computeHideMergeLayout(l, true)!;

			const childRow = result.nodes.find((n) => n.oid === 'child')!.row;
			const hideOutgoing = outgoingEdges(result, childRow).length;
			expect(hideOutgoing).toBeGreaterThan(0);
		});

		it('incoming pruning distributes among children', () => {
			const l = layout(
				[
					node('c1', 0, 0),
					node('c2', 1, 1),
					node('c3', 2, 2),
					node('merge', 3, 0, { is_merge: true }),
					node('ancestor', 4, 0)
				],
				[
					edge(0, 0, 3, 0, 'Straight'),
					edge(1, 1, 3, 0, 'Straight'),
					edge(2, 2, 3, 0, 'Straight'),
					edge(3, 0, 4, 0, 'Straight')
				]
			);

			const result = computeHideMergeLayout(l, true)!;

			const ancestorRow = result.nodes.find((n) => n.oid === 'ancestor')!.row;
			expect(incomingEdges(result, ancestorRow).length).toBeGreaterThan(0);
			for (const oid of ['c1', 'c2', 'c3']) {
				const r = result.nodes.find((n) => n.oid === oid)!.row;
				expect(outgoingEdges(result, r).length).toBeGreaterThan(0);
			}
		});

		it('first-parent ancestor keeps incoming count unchanged', () => {
			const l = layout(
				[
					node('child', 0, 0),
					node('merge', 1, 0, { is_merge: true }),
					node('parent', 2, 0),
					node('branch', 3, 1)
				],
				[edge(0, 0, 1, 0, 'Straight'), edge(1, 0, 2, 0, 'Straight'), edge(1, 1, 3, 1, 'Merge')]
			);

			const result = computeHideMergeLayout(l, true)!;

			const parentRow = result.nodes.find((n) => n.oid === 'parent')!.row;
			expect(incomingEdges(result, parentRow)).toHaveLength(1);
		});

		it('non-merge nodes outside merge subgraph keep exact edge counts', () => {
			const l = layout(
				[
					node('child', 0, 0),
					node('merge', 1, 0, { is_merge: true }),
					node('parent', 2, 0),
					node('sibling', 3, 2),
					node('nephew', 4, 2)
				],
				[
					edge(0, 0, 1, 0, 'Straight'),
					edge(1, 0, 2, 0, 'Straight'),
					edge(2, 0, 3, 2, 'Branch'),
					edge(3, 2, 4, 2, 'Straight')
				]
			);

			const showSiblingIncoming = l.edges.filter((e) => e.to_row === 3).length;
			const showSiblingOutgoing = l.edges.filter((e) => e.from_row === 3).length;

			const result = computeHideMergeLayout(l, true)!;

			const siblingRow = result.nodes.find((n) => n.oid === 'sibling')!.row;
			expect(incomingEdges(result, siblingRow)).toHaveLength(showSiblingIncoming);
			expect(outgoingEdges(result, siblingRow)).toHaveLength(showSiblingOutgoing);
		});

		it('deduplicates when child has direct edge to same ancestor as merge', () => {
			const l = layout(
				[node('child', 0, 0), node('merge', 1, 0, { is_merge: true }), node('ancestor', 2, 0)],
				[edge(0, 0, 1, 0, 'Straight'), edge(1, 0, 2, 0, 'Straight'), edge(0, 0, 2, 0, 'Straight')]
			);

			const result = computeHideMergeLayout(l, true)!;

			const childRow = result.nodes.find((n) => n.oid === 'child')!.row;
			const ancestorRow = result.nodes.find((n) => n.oid === 'ancestor')!.row;
			const edgesToAncestor = result.edges.filter(
				(e) => e.from_row === childRow && e.to_row === ancestorRow
			);
			expect(edgesToAncestor).toHaveLength(1);
		});
	});

	describe('invariant 2: linked from above stays linked from above', () => {
		it('first-parent retains incoming after single merge removal', () => {
			const l = layout(
				[node('child', 0, 0), node('merge', 1, 0, { is_merge: true }), node('parent', 2, 0)],
				[edge(0, 0, 1, 0, 'Straight'), edge(1, 0, 2, 0, 'Straight')]
			);

			const result = computeHideMergeLayout(l, true)!;

			const parentRow = result.nodes.find((n) => n.oid === 'parent')!.row;
			expect(incomingEdges(result, parentRow).length).toBeGreaterThan(0);
		});

		it('non-first-parent retains incoming with sufficient children', () => {
			const l = layout(
				[
					node('c1', 0, 0),
					node('c2', 1, 1),
					node('merge', 2, 0, { is_merge: true }),
					node('feature', 3, 1),
					node('parent', 4, 0),
					node('feature-parent', 5, 1)
				],
				[
					edge(0, 0, 2, 0, 'Straight'),
					edge(1, 1, 2, 0, 'Straight'),
					edge(2, 0, 4, 0, 'Straight'),
					edge(2, 1, 3, 1, 'Merge'),
					edge(3, 1, 5, 1, 'Straight')
				]
			);

			const result = computeHideMergeLayout(l, true)!;

			const featureRow = result.nodes.find((n) => n.oid === 'feature')!.row;
			expect(incomingEdges(result, featureRow).length).toBeGreaterThan(0);
			expect(outgoingEdges(result, featureRow)).toHaveLength(1);
		});

		it('consecutive merges preserve connectivity with sufficient children', () => {
			const l = layout(
				[
					node('c1', 0, 0),
					node('c2', 1, 1),
					node('m1', 2, 0, { is_merge: true }),
					node('m2', 3, 0, { is_merge: true }),
					node('deep-branch', 4, 1),
					node('root', 5, 0)
				],
				[
					edge(0, 0, 2, 0, 'Straight'),
					edge(1, 1, 2, 0, 'Straight'),
					edge(2, 0, 3, 0, 'Straight'),
					edge(3, 1, 4, 1, 'Merge'),
					edge(3, 0, 5, 0, 'Straight'),
					edge(4, 1, 5, 0, 'Straight')
				]
			);

			const result = computeHideMergeLayout(l, true)!;

			const deepRow = result.nodes.find((n) => n.oid === 'deep-branch')!.row;
			expect(incomingEdges(result, deepRow).length).toBeGreaterThan(0);
			expect(outgoingEdges(result, deepRow)).toHaveLength(1);
		});

		it('1 child, 2 ancestors: first-parent stays connected, second disconnects', () => {
			const l = layout(
				[
					node('child', 0, 0),
					node('merge', 1, 0, { is_merge: true }),
					node('parent', 2, 0),
					node('branch', 3, 1)
				],
				[edge(0, 0, 1, 0, 'Straight'), edge(1, 0, 2, 0, 'Straight'), edge(1, 1, 3, 1, 'Merge')]
			);

			const result = computeHideMergeLayout(l, true)!;

			const parentRow = result.nodes.find((n) => n.oid === 'parent')!.row;
			expect(incomingEdges(result, parentRow).length).toBeGreaterThan(0);
			const branchRow = result.nodes.find((n) => n.oid === 'branch')!.row;
			expect(incomingEdges(result, branchRow).length).toBeGreaterThan(0);
		});

		it('1 child, 3 ancestors (octopus): only first parent stays connected', () => {
			const l = layout(
				[
					node('child', 0, 0),
					node('merge', 1, 0, { is_merge: true }),
					node('p1', 2, 0),
					node('p2', 3, 1),
					node('p3', 4, 2)
				],
				[
					edge(0, 0, 1, 0, 'Straight'),
					edge(1, 0, 2, 0, 'Straight'),
					edge(1, 1, 3, 1, 'Merge'),
					edge(1, 2, 4, 2, 'Merge')
				]
			);

			const result = computeHideMergeLayout(l, true)!;

			const p1Row = result.nodes.find((n) => n.oid === 'p1')!.row;
			expect(incomingEdges(result, p1Row).length).toBeGreaterThan(0);
			const p2Row = result.nodes.find((n) => n.oid === 'p2')!.row;
			expect(incomingEdges(result, p2Row).length).toBeGreaterThan(0);
			const p3Row = result.nodes.find((n) => n.oid === 'p3')!.row;
			expect(incomingEdges(result, p3Row).length).toBeGreaterThan(0);
		});

		it('consecutive merges: nearer ancestor gets priority over farther first-parent', () => {
			const l = layout(
				[
					node('child', 0, 0),
					node('m1', 1, 0, { is_merge: true }),
					node('m2', 2, 0, { is_merge: true }),
					node('deep', 3, 1),
					node('root', 4, 0)
				],
				[
					edge(0, 0, 1, 0, 'Straight'),
					edge(1, 0, 2, 0, 'Straight'),
					edge(2, 1, 3, 1, 'Merge'),
					edge(2, 0, 4, 0, 'Straight')
				]
			);

			const result = computeHideMergeLayout(l, true)!;

			const deepRow = result.nodes.find((n) => n.oid === 'deep')!.row;
			expect(incomingEdges(result, deepRow).length).toBeGreaterThan(0);
		});

		it('allows extra outgoing when sole parent is a merge with non-merge ancestors', () => {
			const l = layout(
				[
					node('n0', 0, 1),
					node('m1', 1, 1, { is_merge: true }),
					node('n2', 2, 2),
					node('n3', 3, 2),
					node('n4', 4, 1),
					node('n5', 5, 1)
				],
				[
					edge(0, 1, 1, 1, 'Straight'),
					edge(1, 1, 2, 2, 'Branch'),
					edge(2, 2, 3, 2, 'Straight'),
					edge(3, 2, 5, 1, 'Branch'),
					edge(1, 1, 4, 1, 'Straight'),
					edge(4, 1, 5, 1, 'Straight')
				]
			);

			const result = computeHideMergeLayout(l, true)!;

			expect(result.nodes.map((n) => n.oid)).toEqual(['n0', 'n2', 'n3', 'n4', 'n5']);

			const n0Row = result.nodes.find((n) => n.oid === 'n0')!.row;
			const n2Row = result.nodes.find((n) => n.oid === 'n2')!.row;
			const n3Row = result.nodes.find((n) => n.oid === 'n3')!.row;
			const n4Row = result.nodes.find((n) => n.oid === 'n4')!.row;
			const n5Row = result.nodes.find((n) => n.oid === 'n5')!.row;

			expect(result.edges.find((e) => e.from_row === n0Row && e.to_row === n2Row)).toBeDefined();
			expect(result.edges.find((e) => e.from_row === n0Row && e.to_row === n4Row)).toBeDefined();
			expect(result.edges.find((e) => e.from_row === n2Row && e.to_row === n3Row)).toBeDefined();
			expect(result.edges.find((e) => e.from_row === n3Row && e.to_row === n5Row)).toBeDefined();
			expect(result.edges.find((e) => e.from_row === n4Row && e.to_row === n5Row)).toBeDefined();

			expect(outgoingEdges(result, n0Row)).toHaveLength(2);
		});

		it('preserves linear chain through merge with farther first-parent', () => {
			const l = layout(
				[
					node('n0', 0, 2),
					node('m1', 1, 2, { is_merge: true }),
					node('n2', 2, 1),
					node('m3', 3, 2, { is_merge: true }),
					node('n4', 4, 1),
					node('n5', 5, 1),
					node('n6', 6, 1),
					node('n7', 7, 1),
					node('n8', 8, 1),
					node('n9', 9, 2)
				],
				[
					edge(0, 2, 1, 2, 'Straight'),
					edge(1, 2, 2, 1, 'Branch'),
					edge(1, 2, 3, 2, 'Straight'),
					edge(2, 1, 3, 2, 'Branch'),
					edge(3, 2, 4, 1, 'Branch'),
					edge(3, 2, 9, 2, 'Merge'),
					edge(4, 1, 5, 1, 'Straight'),
					edge(5, 1, 6, 1, 'Straight'),
					edge(6, 1, 7, 1, 'Straight'),
					edge(7, 1, 8, 1, 'Straight'),
					edge(8, 1, 9, 2, 'Branch')
				]
			);

			const result = computeHideMergeLayout(l, true)!;

			expect(result.nodes.map((n) => n.oid)).toEqual([
				'n0',
				'n2',
				'n4',
				'n5',
				'n6',
				'n7',
				'n8',
				'n9'
			]);

			const n0Row = result.nodes.find((n) => n.oid === 'n0')!.row;
			const n2Row = result.nodes.find((n) => n.oid === 'n2')!.row;
			const n4Row = result.nodes.find((n) => n.oid === 'n4')!.row;

			expect(result.edges.find((e) => e.from_row === n0Row && e.to_row === n2Row)).toBeDefined();

			const n2ToN4 = result.edges.find((e) => e.from_row === n2Row && e.to_row === n4Row);
			expect(n2ToN4).toBeDefined();
			expect(n2ToN4!.edge_type).toBe('Straight');
		});

		it('merge tree: only first-parent chain stays connected', () => {
			const l = layout(
				[
					node('child', 0, 0),
					node('m1', 1, 0, { is_merge: true }),
					node('a1', 2, 0),
					node('m2', 3, 1, { is_merge: true }),
					node('a2', 4, 0),
					node('a3', 5, 1)
				],
				[
					edge(0, 0, 1, 0, 'Straight'),
					edge(1, 0, 2, 0, 'Straight'),
					edge(1, 1, 3, 1, 'Merge'),
					edge(3, 0, 4, 0, 'Straight'),
					edge(3, 1, 5, 1, 'Straight')
				]
			);

			const result = computeHideMergeLayout(l, true)!;

			const a1Row = result.nodes.find((n) => n.oid === 'a1')!.row;
			expect(incomingEdges(result, a1Row).length).toBeGreaterThan(0);
			const a2Row = result.nodes.find((n) => n.oid === 'a2')!.row;
			expect(incomingEdges(result, a2Row).length).toBeGreaterThan(0);
			const a3Row = result.nodes.find((n) => n.oid === 'a3')!.row;
			expect(incomingEdges(result, a3Row).length).toBeGreaterThan(0);
		});

		it('octopus merge with 2 children: 3rd parent disconnects', () => {
			const l = layout(
				[
					node('c1', 0, 0),
					node('c2', 1, 1),
					node('merge', 2, 0, { is_merge: true }),
					node('p1', 3, 0),
					node('p2', 4, 1),
					node('p3', 5, 2)
				],
				[
					edge(0, 0, 2, 0, 'Straight'),
					edge(1, 1, 2, 0, 'Straight'),
					edge(2, 0, 3, 0, 'Straight'),
					edge(2, 1, 4, 1, 'Merge'),
					edge(2, 2, 5, 2, 'Merge')
				]
			);

			const result = computeHideMergeLayout(l, true)!;

			const p1Row = result.nodes.find((n) => n.oid === 'p1')!.row;
			expect(incomingEdges(result, p1Row).length).toBeGreaterThan(0);
			const p2Row = result.nodes.find((n) => n.oid === 'p2')!.row;
			expect(incomingEdges(result, p2Row).length).toBeGreaterThan(0);
			const p3Row = result.nodes.find((n) => n.oid === 'p3')!.row;
			expect(incomingEdges(result, p3Row).length).toBeGreaterThan(0);
		});

		it('non-column-matching child with merge as sole parent stays connected via dual constraint', () => {
			const l = layout(
				[
					node('c1', 0, 2),
					node('c2', 1, 0),
					node('merge', 2, 0, { is_merge: true }),
					node('a', 3, 0)
				],
				[edge(0, 0, 2, 0, 'Straight'), edge(1, 1, 2, 0, 'Straight'), edge(2, 0, 3, 0, 'Straight')]
			);

			const result = computeHideMergeLayout(l, true)!;

			for (const oid of ['c1', 'c2']) {
				const r = result.nodes.find((n) => n.oid === oid)!.row;
				expect(outgoingEdges(result, r).length).toBeGreaterThan(0);
			}
			const aRow = result.nodes.find((n) => n.oid === 'a')!.row;
			expect(incomingEdges(result, aRow).length).toBeGreaterThan(0);
		});

		it('child with merge and non-merge parents does not exceed showOutgoing', () => {
			const l = layout(
				[
					node('c', 0, 0),
					node('merge', 1, 0, { is_merge: true }),
					node('x', 2, 0),
					node('a1', 3, 0),
					node('a2', 4, 1)
				],
				[
					edge(0, 0, 1, 0, 'Straight'),
					edge(0, 0, 2, 0, 'Straight'),
					edge(1, 0, 3, 0, 'Straight'),
					edge(1, 1, 4, 1, 'Merge')
				]
			);

			const result = computeHideMergeLayout(l, true)!;

			const cRow = result.nodes.find((n) => n.oid === 'c')!.row;
			expect(outgoingEdges(result, cRow).length).toBeLessThanOrEqual(2);
		});

		it('ancestor incoming limit is respected when multiple children share one ancestor', () => {
			const l = layout(
				[
					node('c1', 0, 0),
					node('c2', 1, 1),
					node('merge', 2, 0, { is_merge: true }),
					node('a', 3, 0)
				],
				[edge(0, 0, 2, 0, 'Straight'), edge(1, 1, 2, 0, 'Straight'), edge(2, 0, 3, 0, 'Straight')]
			);

			const result = computeHideMergeLayout(l, true)!;

			for (const oid of ['c1', 'c2']) {
				const r = result.nodes.find((n) => n.oid === oid)!.row;
				expect(outgoingEdges(result, r).length).toBeGreaterThan(0);
			}
			const aRow = result.nodes.find((n) => n.oid === 'a')!.row;
			expect(incomingEdges(result, aRow).length).toBeGreaterThan(0);
		});

		it('octopus merge children maintain connectivity per ancestor capacity', () => {
			const l = layout(
				[
					node('c1', 0, 0),
					node('c2', 1, 1),
					node('merge', 2, 0, { is_merge: true }),
					node('a1', 3, 0),
					node('a2', 4, 1),
					node('a3', 5, 2)
				],
				[
					edge(0, 0, 2, 0, 'Straight'),
					edge(1, 1, 2, 0, 'Straight'),
					edge(2, 0, 3, 0, 'Straight'),
					edge(2, 1, 4, 1, 'Merge'),
					edge(2, 2, 5, 2, 'Merge'),
					edge(3, 0, 4, 1, 'Branch')
				]
			);

			const result = computeHideMergeLayout(l, true)!;

			for (const oid of ['c1', 'c2']) {
				const row = result.nodes.find((n) => n.oid === oid)!.row;
				expect(outgoingEdges(result, row).length).toBeGreaterThan(0);
			}
			const a1Row = result.nodes.find((n) => n.oid === 'a1')!.row;
			expect(incomingEdges(result, a1Row).length).toBeLessThanOrEqual(2);
			const a2Row = result.nodes.find((n) => n.oid === 'a2')!.row;
			expect(incomingEdges(result, a2Row).length).toBeLessThanOrEqual(2);
			const a3Row = result.nodes.find((n) => n.oid === 'a3')!.row;
			expect(incomingEdges(result, a3Row).length).toBeLessThanOrEqual(1);
		});

		it('rewires child through consecutive merges where second merge has no non-merge ancestors', () => {
			const l = layout(
				[
					node('n0', 0, 1),
					node('m1', 1, 1, { is_merge: true }),
					node('n2', 2, 0),
					node('n3', 3, 0),
					node('m4', 4, 1, { is_merge: true })
				],
				[
					edge(0, 1, 1, 1, 'Straight'),
					edge(1, 1, 2, 0, 'Straight'),
					edge(2, 0, 3, 0, 'Straight'),
					edge(3, 0, 4, 1, 'Branch'),
					edge(1, 1, 4, 1, 'Merge')
				]
			);

			const result = computeHideMergeLayout(l, true)!;

			expect(result.nodes).toHaveLength(3);
			expect(result.nodes.map((n) => n.oid)).toEqual(['n0', 'n2', 'n3']);

			const n0Row = result.nodes.find((n) => n.oid === 'n0')!.row;
			const n2Row = result.nodes.find((n) => n.oid === 'n2')!.row;
			const n3Row = result.nodes.find((n) => n.oid === 'n3')!.row;

			const n0ToN2 = result.edges.find((e) => e.from_row === n0Row && e.to_row === n2Row);
			expect(n0ToN2).toBeDefined();
			expect(n0ToN2!.edge_type).toBe('Branch');

			const n2ToN3 = result.edges.find((e) => e.from_row === n2Row && e.to_row === n3Row);
			expect(n2ToN3).toBeDefined();
			expect(n2ToN3!.edge_type).toBe('Straight');
		});

		it('nested merge chain preserves child connectivity', () => {
			const l = layout(
				[
					node('c1', 0, 0),
					node('c2', 1, 1),
					node('m1', 2, 0, { is_merge: true }),
					node('m2', 3, 0, { is_merge: true }),
					node('a', 4, 0)
				],
				[
					edge(0, 0, 2, 0, 'Straight'),
					edge(1, 1, 2, 0, 'Straight'),
					edge(2, 0, 3, 0, 'Straight'),
					edge(3, 0, 4, 0, 'Straight')
				]
			);

			const result = computeHideMergeLayout(l, true)!;

			for (const oid of ['c1', 'c2']) {
				const r = result.nodes.find((n) => n.oid === oid)!.row;
				expect(outgoingEdges(result, r).length).toBeGreaterThan(0);
			}
			const aRow = result.nodes.find((n) => n.oid === 'a')!.row;
			expect(incomingEdges(result, aRow).length).toBeGreaterThan(0);
		});

		it('feature branch with descendants: branch disconnects from above', () => {
			const l = layout(
				[
					node('child', 0, 0),
					node('merge', 1, 0, { is_merge: true }),
					node('main', 2, 0),
					node('feature', 3, 1),
					node('feature-child', 4, 1),
					node('root', 5, 0)
				],
				[
					edge(0, 0, 1, 0, 'Straight'),
					edge(1, 0, 2, 0, 'Straight'),
					edge(1, 1, 3, 1, 'Merge'),
					edge(2, 0, 5, 0, 'Straight'),
					edge(3, 1, 4, 1, 'Straight'),
					edge(4, 1, 5, 0, 'Straight')
				]
			);

			const result = computeHideMergeLayout(l, true)!;

			const mainRow = result.nodes.find((n) => n.oid === 'main')!.row;
			expect(incomingEdges(result, mainRow).length).toBeGreaterThan(0);
			const featureRow = result.nodes.find((n) => n.oid === 'feature')!.row;
			expect(incomingEdges(result, featureRow).length).toBeGreaterThan(0);
		});
	});

	describe('invariant 3: not linked from above stays not linked from above', () => {
		it('head node gains no incoming edge', () => {
			const l = layout(
				[node('head', 0, 0), node('merge', 1, 0, { is_merge: true }), node('parent', 2, 0)],
				[edge(0, 0, 1, 0, 'Straight'), edge(1, 0, 2, 0, 'Straight')]
			);

			const result = computeHideMergeLayout(l, true)!;

			const headRow = result.nodes.find((n) => n.oid === 'head')!.row;
			expect(incomingEdges(result, headRow)).toHaveLength(0);
		});

		it('unrelated branch gains no extra incoming edge from merge rewiring', () => {
			const l = layout(
				[
					node('child', 0, 0),
					node('merge', 1, 0, { is_merge: true }),
					node('parent', 2, 0),
					node('unrelated', 3, 2)
				],
				[edge(0, 0, 1, 0, 'Straight'), edge(1, 0, 2, 0, 'Straight'), edge(2, 0, 3, 2, 'Branch')]
			);

			const result = computeHideMergeLayout(l, true)!;

			const unrelatedRow = result.nodes.find((n) => n.oid === 'unrelated')!.row;
			expect(incomingEdges(result, unrelatedRow)).toHaveLength(1);
		});

		it('isolated node outside merge subgraph gains no incoming edges', () => {
			const l = layout(
				[
					node('child', 0, 0),
					node('merge', 1, 0, { is_merge: true }),
					node('parent', 2, 0),
					node('island', 3, 2),
					node('island-child', 4, 2)
				],
				[edge(0, 0, 1, 0, 'Straight'), edge(1, 0, 2, 0, 'Straight'), edge(3, 2, 4, 2, 'Straight')]
			);

			const result = computeHideMergeLayout(l, true)!;

			const islandRow = result.nodes.find((n) => n.oid === 'island')!.row;
			expect(incomingEdges(result, islandRow)).toHaveLength(0);
		});

		it('node with zero incoming in show keeps zero incoming in hide', () => {
			const l = layout(
				[
					node('c1', 0, 0),
					node('c2', 1, 1),
					node('merge', 2, 0, { is_merge: true }),
					node('ancestor', 3, 0)
				],
				[edge(0, 0, 2, 0, 'Straight'), edge(1, 1, 2, 0, 'Straight'), edge(2, 0, 3, 0, 'Straight')]
			);

			const result = computeHideMergeLayout(l, true)!;

			const c2Row = result.nodes.find((n) => n.oid === 'c2')!.row;
			expect(incomingEdges(result, c2Row)).toHaveLength(0);
		});
	});

	describe('parent-preservation invariant', () => {
		function assertOutgoingPreserved(input: GraphLayout, result: GraphLayout, label: string) {
			for (const showNode of input.nodes) {
				if (showNode.is_merge || showNode.is_stash) continue;
				const hadOutgoing = input.edges.some(
					(e) => e.from_row === showNode.row && e.from_col === showNode.column
				);
				if (!hadOutgoing) continue;

				const hideNode = result.nodes.find((n) => n.oid === showNode.oid);
				expect(hideNode).toBeDefined();

				const hasOutgoing = result.edges.some(
					(e) => e.from_row === hideNode!.row && e.from_col === hideNode!.column
				);
				expect(hasOutgoing, `${label}: node ${showNode.oid} lost all outgoing edges`).toBe(true);
			}
		}

		it('simple merge with child', () => {
			const l = layout(
				[
					node('child', 0, 0),
					node('merge', 1, 0, { is_merge: true }),
					node('rootA', 2, 0),
					node('rootB', 3, 1)
				],
				[edge(0, 0, 1, 0, 'Straight'), edge(1, 0, 2, 0, 'Straight'), edge(1, 0, 3, 1, 'Merge')]
			);

			const result = computeHideMergeLayout(l, true)!;
			assertOutgoingPreserved(l, result, 'simple merge');
			assertNoNewLeaves(l, result, 'simple merge');
		});

		it('chained merges', () => {
			const l = layout(
				[
					node('child', 0, 0),
					node('outer', 1, 0, { is_merge: true }),
					node('inner', 2, 0, { is_merge: true }),
					node('rootC', 3, 1),
					node('rootA', 4, 0),
					node('rootB', 5, 1)
				],
				[
					edge(0, 0, 1, 0, 'Straight'),
					edge(1, 0, 2, 0, 'Straight'),
					edge(1, 0, 3, 1, 'Merge'),
					edge(2, 0, 4, 0, 'Straight'),
					edge(2, 0, 5, 1, 'Merge')
				]
			);

			const result = computeHideMergeLayout(l, true)!;
			assertOutgoingPreserved(l, result, 'chained merges');
			assertNoNewLeaves(l, result, 'chained merges');
		});

		it('multiple children of same merge', () => {
			const l = layout(
				[
					node('c1', 0, 0),
					node('c2', 1, 1),
					node('merge', 2, 0, { is_merge: true }),
					node('rootA', 3, 0),
					node('rootB', 4, 1)
				],
				[
					edge(0, 0, 2, 0, 'Straight'),
					edge(1, 1, 2, 0, 'Branch'),
					edge(2, 0, 3, 0, 'Straight'),
					edge(2, 0, 4, 1, 'Merge')
				]
			);

			const result = computeHideMergeLayout(l, true)!;
			assertOutgoingPreserved(l, result, 'multiple children');
			assertNoNewLeaves(l, result, 'multiple children');
		});

		it('child whose only parent is a merge', () => {
			const l = layout(
				[
					node('grandchild', 0, 0),
					node('child', 1, 0),
					node('merge', 2, 0, { is_merge: true }),
					node('rootA', 3, 0),
					node('rootB', 4, 1)
				],
				[
					edge(0, 0, 1, 0, 'Straight'),
					edge(1, 0, 2, 0, 'Straight'),
					edge(2, 0, 3, 0, 'Straight'),
					edge(2, 0, 4, 1, 'Merge')
				]
			);

			const result = computeHideMergeLayout(l, true)!;
			assertOutgoingPreserved(l, result, 'only merge parent');
			assertNoNewLeaves(l, result, 'only merge parent');
		});

		it('merge of merge (nested)', () => {
			const l = layout(
				[
					node('child', 0, 0),
					node('outer', 1, 0, { is_merge: true }),
					node('inner', 2, 0, { is_merge: true }),
					node('feature', 3, 1),
					node('main', 4, 0),
					node('branch', 5, 1),
					node('root', 6, 0)
				],
				[
					edge(0, 0, 1, 0, 'Straight'),
					edge(1, 0, 2, 0, 'Straight'),
					edge(1, 0, 3, 1, 'Merge'),
					edge(2, 0, 4, 0, 'Straight'),
					edge(2, 0, 5, 1, 'Merge'),
					edge(3, 1, 6, 0, 'Branch'),
					edge(4, 0, 6, 0, 'Straight'),
					edge(5, 1, 6, 0, 'Branch')
				]
			);

			const result = computeHideMergeLayout(l, true)!;
			assertOutgoingPreserved(l, result, 'merge of merge');
			assertNoNewLeaves(l, result, 'merge of merge');
		});

		it('two merges sharing the same ancestor', () => {
			const l = layout(
				[
					node('c1', 0, 0),
					node('m1', 1, 0, { is_merge: true }),
					node('c2', 2, 1),
					node('m2', 3, 0, { is_merge: true }),
					node('shared', 4, 0),
					node('other', 5, 1)
				],
				[
					edge(0, 0, 1, 0, 'Straight'),
					edge(1, 0, 4, 0, 'Straight'),
					edge(1, 0, 5, 1, 'Merge'),
					edge(2, 1, 3, 0, 'Branch'),
					edge(3, 0, 4, 0, 'Straight'),
					edge(3, 0, 5, 1, 'Merge')
				]
			);

			const result = computeHideMergeLayout(l, true)!;
			assertOutgoingPreserved(l, result, 'shared ancestor');
			assertNoNewLeaves(l, result, 'shared ancestor');
		});

		it('three siblings all with only-merge parent', () => {
			const l = layout(
				[
					node('s1', 0, 0),
					node('s2', 1, 1),
					node('s3', 2, 2),
					node('merge', 3, 0, { is_merge: true }),
					node('rootA', 4, 0),
					node('rootB', 5, 1)
				],
				[
					edge(0, 0, 3, 0, 'Straight'),
					edge(1, 1, 3, 0, 'Branch'),
					edge(2, 2, 3, 0, 'Branch'),
					edge(3, 0, 4, 0, 'Straight'),
					edge(3, 0, 5, 1, 'Merge')
				]
			);

			const result = computeHideMergeLayout(l, true)!;
			assertOutgoingPreserved(l, result, 'three siblings');
			assertNoNewLeaves(l, result, 'three siblings');
		});
	});

	describe('merge+branch preservation', () => {
		it('keeps merge with 2 direct children and 2 parents', () => {
			const l = layout(
				[
					node('d1', 0, 0),
					node('d2', 1, 1),
					node('merge', 2, 0, { is_merge: true }),
					node('p1', 3, 0),
					node('p2', 4, 1)
				],
				[
					edge(0, 0, 2, 0, 'Straight'),
					edge(1, 1, 2, 0, 'Straight'),
					edge(2, 0, 3, 0, 'Straight'),
					edge(2, 1, 4, 1, 'Merge')
				]
			);

			const result = computeHideMergeLayout(l, true)!;

			expect(result.nodes.find((n) => n.oid === 'merge')).toBeDefined();
			expect(result.nodes).toHaveLength(5);
			assertNoNewLeaves(l, result, 'merge with 2 children');
		});

		it('removes merge with 1 child (single trunk above)', () => {
			const l = layout(
				[
					node('child', 0, 0),
					node('merge', 1, 0, { is_merge: true }),
					node('p1', 2, 0),
					node('p2', 3, 1)
				],
				[edge(0, 0, 1, 0, 'Straight'), edge(1, 0, 2, 0, 'Straight'), edge(1, 1, 3, 1, 'Merge')]
			);

			const result = computeHideMergeLayout(l, true)!;

			expect(result.nodes.find((n) => n.oid === 'merge')).toBeUndefined();
			expect(result.nodes).toHaveLength(3);
		});

		it('kept child merge allows parent merge to be removed (inner rewires to ancestors)', () => {
			const l = layout(
				[
					node('c1', 0, 0),
					node('c2', 1, 1),
					node('inner', 2, 0, { is_merge: true }),
					node('outer', 3, 0, { is_merge: true }),
					node('root', 4, 0),
					node('side', 5, 1)
				],
				[
					edge(0, 0, 2, 0, 'Straight'),
					edge(1, 1, 2, 0, 'Straight'),
					edge(2, 0, 3, 0, 'Straight'),
					edge(3, 0, 4, 0, 'Straight'),
					edge(3, 1, 5, 1, 'Merge')
				]
			);

			const result = computeHideMergeLayout(l, true)!;

			expect(result.nodes.find((n) => n.oid === 'inner')).toBeDefined();
			expect(result.nodes.find((n) => n.oid === 'outer')).toBeUndefined();

			const innerRow = result.nodes.findIndex((n) => n.oid === 'inner');
			const rootRow = result.nodes.findIndex((n) => n.oid === 'root');
			const sideRow = result.nodes.findIndex((n) => n.oid === 'side');

			expect(
				result.edges.find((e) => e.from_row === innerRow && e.to_row === rootRow)
			).toBeDefined();
			expect(
				result.edges.find((e) => e.from_row === innerRow && e.to_row === sideRow)
			).toBeDefined();
			assertNoNewLeaves(l, result, 'kept child merge');
		});

		it('removes both merges when inner has only 1 child (single trunk)', () => {
			const l = layout(
				[
					node('head', 0, 0),
					node('inner', 1, 0, { is_merge: true }),
					node('outer', 2, 0, { is_merge: true }),
					node('p1', 3, 0),
					node('p2', 4, 1)
				],
				[
					edge(0, 0, 1, 0, 'Straight'),
					edge(1, 0, 2, 0, 'Straight'),
					edge(2, 0, 3, 0, 'Straight'),
					edge(2, 1, 4, 1, 'Merge')
				]
			);

			const result = computeHideMergeLayout(l, true)!;

			expect(result.nodes.find((n) => n.oid === 'inner')).toBeUndefined();
			expect(result.nodes.find((n) => n.oid === 'outer')).toBeUndefined();
			expect(result.nodes).toHaveLength(3);
		});

		it('kept merge with non-merge parent and removed-merge parent retains both edges', () => {
			const l = layout(
				[
					node('d1', 0, 0),
					node('d2', 1, 1),
					node('kept', 2, 0, { is_merge: true }),
					node('directParent', 3, 0),
					node('removedMerge', 4, 0, { is_merge: true }),
					node('deepAncestor', 5, 1)
				],
				[
					edge(0, 0, 2, 0, 'Straight'),
					edge(1, 1, 2, 0, 'Straight'),
					edge(2, 0, 3, 0, 'Straight'),
					edge(2, 1, 4, 0, 'Merge'),
					edge(4, 0, 5, 1, 'Merge')
				]
			);

			const result = computeHideMergeLayout(l, true)!;

			const keptNode = result.nodes.find((n) => n.oid === 'kept');
			expect(keptNode).toBeDefined();

			const directParentRow = result.nodes.findIndex((n) => n.oid === 'directParent');
			const deepAncestorRow = result.nodes.findIndex((n) => n.oid === 'deepAncestor');

			expect(
				result.edges.find((e) => e.from_row === keptNode!.row && e.to_row === directParentRow)
			).toBeDefined();
			expect(
				result.edges.find((e) => e.from_row === keptNode!.row && e.to_row === deepAncestorRow)
			).toBeDefined();

			assertNoNewLeaves(l, result, 'kept merge mixed parents');
		});
	});
});
