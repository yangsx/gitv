import type { GraphLayout, NodePosition, Edge, WorkingChangesDiff, CommitInfo } from '$lib/bindings/types';
import { STAGED_OID, UNSTAGED_OID } from '$lib/constants';

export const UNSTAGED_EDGE_COLOR = { r: 251, g: 146, b: 60, a: 200 };
export const STAGED_EDGE_COLOR = { r: 74, g: 222, b: 128, a: 200 };
export const VIRTUAL_NODE_COLOR = { r: 255, g: 255, b: 255, a: 255 };

export function applyVirtualWorkingChanges(
	graphLayout: GraphLayout | null,
	workingChangesDiff: WorkingChangesDiff | null,
	headOid?: string | null
): GraphLayout | null {
	if (!graphLayout) return null;
	if (!workingChangesDiff) return graphLayout;
	const hasStaged = workingChangesDiff.staged.length > 0;
	const hasUnstaged = workingChangesDiff.unstaged.length > 0;
	if (!hasStaged && !hasUnstaged) return graphLayout;

	const virtualCount = (hasStaged ? 1 : 0) + (hasUnstaged ? 1 : 0);

	const virtualNodes: NodePosition[] = [];
	if (hasUnstaged) {
		virtualNodes.push({
			oid: UNSTAGED_OID,
			row: 0,
			column: 0,
			is_merge: false,
			color: VIRTUAL_NODE_COLOR,
			is_dimmed: false,
			is_highlighted: false,
			is_stash: false
		});
	}
	if (hasStaged) {
		virtualNodes.push({
			oid: STAGED_OID,
			row: hasUnstaged ? 1 : 0,
			column: 0,
			is_merge: false,
			color: VIRTUAL_NODE_COLOR,
			is_dimmed: false,
			is_highlighted: false,
			is_stash: false
		});
	}

	const virtualEdges: Edge[] = [];
	const headNode = headOid
		? graphLayout.nodes.find((n) => n.oid === headOid) ?? null
		: (graphLayout.nodes.length > 0 ? graphLayout.nodes[0] : null);
	if (headNode) {
		const headRow = headNode.row + virtualCount;
		const headCol = headNode.column;
		if (hasUnstaged) {
			virtualEdges.push({
				from_row: 0,
				from_col: 0,
				to_row: headRow,
				to_col: headCol,
				edge_type: 'Straight' as const,
				color: UNSTAGED_EDGE_COLOR,
				is_dimmed: false,
				edge_style: 'Solid' as const
			});
		}
		if (hasStaged) {
			virtualEdges.push({
				from_row: hasUnstaged ? 1 : 0,
				from_col: 0,
				to_row: headRow,
				to_col: headCol,
				edge_type: 'Straight' as const,
				color: STAGED_EDGE_COLOR,
				is_dimmed: false,
				edge_style: 'Solid' as const
			});
		}
	}

	return {
		...graphLayout,
		nodes: [
			...virtualNodes,
			...graphLayout.nodes.map((n) => ({ ...n, row: n.row + virtualCount }))
		],
		edges: [
			...virtualEdges,
			...graphLayout.edges.map((e) => ({
				...e,
				from_row: e.from_row + virtualCount,
				to_row: e.to_row + virtualCount
			}))
		],
		stash_markers: graphLayout.stash_markers.map((s) => ({ ...s, row: s.row + virtualCount })),
		total_rows: graphLayout.total_rows + virtualCount
	};
}

function makeVirtualCommit(oid: string, summary: string): CommitInfo {
	return {
		oid,
		short_oid: '',
		message: summary,
		summary,
		author: { name: '', email: '' },
		committer: { name: '', email: '' },
		author_time: '',
		commit_time: '',
		parent_oids: [],
		refs: []
	};
}

export function createVirtualCommitInfos(
	workingChangesDiff: WorkingChangesDiff | null,
	t: (key: string) => string
): CommitInfo[] {
	if (!workingChangesDiff) return [];
	const hasStaged = workingChangesDiff.staged.length > 0;
	const hasUnstaged = workingChangesDiff.unstaged.length > 0;
	if (!hasStaged && !hasUnstaged) return [];
	const virtuals: CommitInfo[] = [];
	if (hasUnstaged)
		virtuals.push(
			makeVirtualCommit(UNSTAGED_OID, t('page.unstaged'))
		);
	if (hasStaged)
		virtuals.push(
			makeVirtualCommit(STAGED_OID, t('page.staged'))
		);
	return virtuals;
}
