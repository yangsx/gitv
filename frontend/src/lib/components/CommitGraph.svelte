<script lang="ts">
	import { t } from '$lib/stores/locale';
	import type {
		GraphLayout,
		NodePosition,
		Edge,
		StashMarker,
		CommitInfo
	} from '$lib/bindings/types';
	import { showStashes } from '$lib/stores/preferences';
	import { updateGraphDrawTime } from '$lib/stores/debug';
	import {
		colorToCSS,
		columnCenterX,
		nodeCenterY,
		isEdgeVisible,
		stashX,
		stashY,
		SELECTED_COLOR,
		COMPARISON_COLOR
	} from '$lib/graph/graph-math';

	const PADDING_LEFT = 12;

	interface Props {
		layout: GraphLayout;
		commits: CommitInfo[];
		rowHeight?: number;
		laneWidth?: number;
		nodeRadius?: number;
		visibleStart: number;
		visibleEnd: number;
		selectedOid?: string | null;
		comparisonOid?: string | null;
		onSelect?: (_oid: string, _ctrlKey: boolean) => void;
		onStashSelect?: (_stashIndex: number) => void;
	}

	let {
		layout,
		commits,
		rowHeight = 28,
		laneWidth = 24,
		nodeRadius = 4,
		visibleStart,
		visibleEnd,
		selectedOid,
		comparisonOid = null,
		onSelect,
		onStashSelect
	}: Props = $props();

	let canvas: HTMLCanvasElement;
	let tooltip = $state<{ x: number; y: number; text: string } | null>(null);
	let scale = $state(1.0);
	let prevCanvasW = 0;
	let prevCanvasH = 0;

	let commitMap = $derived(new Map(commits.map((c) => [c.oid, c])));

	function draw(l: GraphLayout) {
		if (!canvas) return;
		const ctx = canvas.getContext('2d');
		if (!ctx) return;
		const drawStart = performance.now();

		const sc = scale;
		const sLaneWidth = laneWidth * sc;
		const sNodeRadius = nodeRadius * sc;
		const sPadding = PADDING_LEFT * sc;

		const height = (visibleEnd - visibleStart) * rowHeight;
		const width = l.total_columns * sLaneWidth + sPadding * 2;
		if (width <= 0 || height <= 0) return;

		const cssW = `${width}px`;
		const cssH = `${height}px`;
		if (width !== prevCanvasW || height !== prevCanvasH) {
			canvas.width = width * devicePixelRatio;
			canvas.height = height * devicePixelRatio;
			canvas.style.width = cssW;
			canvas.style.height = cssH;
			prevCanvasW = width;
			prevCanvasH = height;
		}
		ctx.setTransform(devicePixelRatio, 0, 0, devicePixelRatio, 0, 0);
		ctx.clearRect(0, 0, width, height);

		const startRow = visibleStart;
		const endRow = visibleEnd;

		for (const edge of l.edges) {
			if (!isEdgeVisible(edge, startRow, endRow)) continue;
			drawEdge(ctx, edge, sLaneWidth, sPadding, startRow, rowHeight);
		}
		for (const node of l.nodes) {
			if (node.row < startRow || node.row > endRow) continue;
			drawNode(ctx, node, sLaneWidth, sNodeRadius, sPadding, startRow, rowHeight);
		}
		if ($showStashes) {
			for (const stash of l.stash_markers) {
				if (stash.row < startRow || stash.row > endRow) continue;
				drawStashMarker(ctx, stash, sLaneWidth, sNodeRadius, sPadding, startRow, rowHeight, sc);
			}
		}
		updateGraphDrawTime(performance.now() - drawStart);
	}

	$effect(() => {
		void visibleStart;
		void visibleEnd;
		draw(layout);
	});

	function handleClick(e: MouseEvent) {
		if (!onSelect && !onStashSelect) return;
		const rect = canvas.getBoundingClientRect();
		const sc = scale;
		const sLaneWidth = laneWidth * sc;
		const sNodeRadius = nodeRadius * sc;
		const sPadding = PADDING_LEFT * sc;
		const x = e.clientX - rect.left;
		const y = e.clientY - rect.top;
		const row = Math.floor(y / rowHeight) + visibleStart;

		if (onStashSelect && $showStashes) {
			for (const stash of layout.stash_markers) {
				if (stash.row === row) {
					const msx = stashX(stash.column, sLaneWidth, sPadding, sNodeRadius, sc);
					const msy = stashY(stash.row, visibleStart, rowHeight);
					if (Math.abs(Math.abs(x - msx)) < 20 * sc && Math.abs(y - msy) < 20 * sc) {
						onStashSelect(stash.stash_index);
						return;
					}
				}
			}
		}

		for (const node of layout.nodes) {
			if (node.row === row) {
				onSelect?.(node.oid, e.ctrlKey || e.metaKey);
				return;
			}
		}
	}

	function handleMouseMove(e: MouseEvent) {
		const rect = canvas.getBoundingClientRect();
		const sc = scale;
		const sLaneWidth = laneWidth * sc;
		const sNodeRadius = nodeRadius * sc;
		const sPadding = PADDING_LEFT * sc;
		const x = e.clientX - rect.left;
		const y = e.clientY - rect.top;
		const row = Math.floor(y / rowHeight) + visibleStart;
		const hitRadius = sNodeRadius + 4 * sc;

		if ($showStashes) {
			for (const stash of layout.stash_markers) {
				if (stash.row === row) {
					const msx = stashX(stash.column, sLaneWidth, sPadding, sNodeRadius, sc);
					const msy = stashY(stash.row, visibleStart, rowHeight);
					if (Math.abs(x - msx) < hitRadius && Math.abs(y - msy) < hitRadius) {
						tooltip = {
							x: e.clientX - rect.left + 12,
							y: e.clientY - rect.top - 8,
							text: stash.message
						};
						return;
					}
				}
			}
		}

		for (const node of layout.nodes) {
			if (node.row === row) {
				const nx = columnCenterX(node.column, sLaneWidth, sPadding);
				const ny = nodeCenterY(node.row, visibleStart, rowHeight);
				if (Math.abs(x - nx) < hitRadius && Math.abs(y - ny) < hitRadius) {
					const ci = commitMap.get(node.oid);
					if (ci) {
						tooltip = {
							x: e.clientX - rect.left + 12,
							y: e.clientY - rect.top - 8,
							text: `${ci.short_oid} ${ci.summary}`
						};
						return;
					}
				}
			}
		}

		tooltip = null;
	}

	function handleMouseLeave() {
		tooltip = null;
	}

	function handleWheel(e: WheelEvent) {
		if (!e.ctrlKey && !e.metaKey) return;
		e.preventDefault();
		const delta = e.deltaY > 0 ? -0.1 : 0.1;
		scale = Math.max(0.5, Math.min(2.0, scale + delta));
	}

	function drawNode(
		ctx: CanvasRenderingContext2D,
		node: NodePosition,
		sLaneWidth: number,
		sNodeRadius: number,
		sPadding: number,
		startRow: number,
		rh: number
	) {
		const x = columnCenterX(node.column, sLaneWidth, sPadding);
		const y = nodeCenterY(node.row, startRow, rh);

		if (node.oid === selectedOid) {
			ctx.beginPath();
			ctx.arc(x, y, sNodeRadius + 3, 0, Math.PI * 2);
			ctx.strokeStyle = SELECTED_COLOR;
			ctx.lineWidth = 2;
			ctx.stroke();
		} else if (node.oid === comparisonOid) {
			ctx.beginPath();
			ctx.arc(x, y, sNodeRadius + 3, 0, Math.PI * 2);
			ctx.strokeStyle = COMPARISON_COLOR;
			ctx.lineWidth = 2;
			ctx.setLineDash([3, 3]);
			ctx.stroke();
			ctx.setLineDash([]);
		}

		ctx.globalAlpha = node.is_dimmed ? 0.35 : 1.0;
		ctx.beginPath();
		ctx.arc(x, y, sNodeRadius, 0, Math.PI * 2);
		ctx.fillStyle = colorToCSS(node.color);
		ctx.fill();
		ctx.globalAlpha = 1.0;
	}

	function drawEdge(
		ctx: CanvasRenderingContext2D,
		edge: Edge,
		sLaneWidth: number,
		sPadding: number,
		startRow: number,
		rh: number
	) {
		const x1 = columnCenterX(edge.from_col, sLaneWidth, sPadding);
		const y1 = nodeCenterY(edge.from_row, startRow, rh);
		const x2 = columnCenterX(edge.to_col, sLaneWidth, sPadding);
		const y2 = nodeCenterY(edge.to_row, startRow, rh);

		ctx.beginPath();
		ctx.globalAlpha = edge.is_dimmed ? 0.35 : 0.8;
		ctx.strokeStyle = colorToCSS(edge.color);
		ctx.lineWidth = 1.5;

		if (edge.edge_style === 'Dashed') {
			ctx.setLineDash([6, 3]);
		} else if (edge.edge_style === 'Dotted') {
			ctx.setLineDash([2, 3]);
		} else {
			ctx.setLineDash([]);
		}

		if (edge.from_col === edge.to_col) {
			ctx.moveTo(x1, y1);
			ctx.lineTo(x2, y2);
		} else {
			const midY = (y1 + y2) / 2;
			ctx.moveTo(x1, y1);
			ctx.bezierCurveTo(x1, midY, x2, midY, x2, y2);
		}

		ctx.stroke();
		ctx.setLineDash([]);
		ctx.globalAlpha = 1.0;
	}

	function drawStashMarker(
		ctx: CanvasRenderingContext2D,
		stash: StashMarker,
		sLaneWidth: number,
		sNodeRadius: number,
		sPadding: number,
		startRow: number,
		rh: number,
		sc: number
	) {
		const x = stashX(stash.column, sLaneWidth, sPadding, sNodeRadius, sc);
		const y = stashY(stash.row, startRow, rh);

		ctx.font = `${10 * sc}px monospace`;
		ctx.fillStyle = '#f59e0b';
		ctx.fillText(`S${stash.stash_index}`, x, y + 3 * sc);
	}
</script>

<canvas
	bind:this={canvas}
	class="block cursor-pointer"
	aria-label={$t('commit_graph.aria')}
	onclick={handleClick}
	onmousemove={handleMouseMove}
	onmouseleave={handleMouseLeave}
	onwheel={handleWheel}
></canvas>

{#if tooltip}
	<div
		class="pointer-events-none absolute z-50 max-w-[250px] rounded bg-gray-800 px-2 py-1 text-xs text-gray-200 shadow-lg border border-gray-700"
		style="left: {tooltip.x}px; top: {tooltip.y}px;"
	>
		{tooltip.text}
	</div>
{/if}
