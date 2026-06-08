<script lang="ts">
	import { t } from '$lib/stores/locale';
	import type {
		GraphLayout,
		Color,
		NodePosition,
		Edge,
		StashMarker,
		CommitInfo
	} from '$lib/bindings/types';
	import { showStashes } from '$lib/stores/preferences';
	import { updateGraphDrawTime } from '$lib/stores/debug';

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

	const PADDING_LEFT = 12;

	let commitMap = $derived(new Map(commits.map((c) => [c.oid, c])));

	function colorToCss(c: Color): string {
		return `rgba(${c.r},${c.g},${c.b},${(c.a / 255).toFixed(2)})`;
	}

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

		const colXfn = (col: number) => sPadding + col * sLaneWidth + sLaneWidth / 2;
		const rowYfn = (row: number) => (row - startRow) * rowHeight + rowHeight / 2;

		for (const edge of l.edges) {
			if (edge.from_row < startRow && edge.to_row < startRow) continue;
			if (edge.from_row > endRow && edge.to_row > endRow) continue;
			drawEdge(ctx, edge, startRow, colXfn, rowYfn);
		}
		for (const node of l.nodes) {
			if (node.row < startRow || node.row > endRow) continue;
			drawNode(ctx, node, startRow, colXfn, rowYfn, sNodeRadius);
		}
		if ($showStashes) {
			for (const stash of l.stash_markers) {
				if (stash.row < startRow || stash.row > endRow) continue;
				drawStashMarker(ctx, stash, startRow, colXfn, rowYfn, sNodeRadius, sc);
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
		const sPadding = PADDING_LEFT * sc;
		const x = e.clientX - rect.left;
		const y = e.clientY - rect.top;
		const row = Math.floor(y / rowHeight) + visibleStart;
		const colXfn = (col: number) => sPadding + col * sLaneWidth + sLaneWidth / 2;

		if (onStashSelect && $showStashes) {
			for (const stash of layout.stash_markers) {
				if (stash.row === row) {
					const markerX = colXfn(stash.column) + nodeRadius * sc + 4 * sc;
					if (Math.abs(x - markerX) < 20 * sc) {
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
		const colXfn = (col: number) => sPadding + col * sLaneWidth + sLaneWidth / 2;
		const hitRadius = sNodeRadius + 4 * sc;

		if ($showStashes) {
			for (const stash of layout.stash_markers) {
				if (stash.row === row) {
					const markerX = colXfn(stash.column) + sNodeRadius + 4 * sc;
					if (Math.abs(x - markerX) < hitRadius) {
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
				const nodeX = colXfn(node.column);
				const nodeY = (node.row - visibleStart) * rowHeight + rowHeight / 2;
				if (Math.abs(x - nodeX) < hitRadius && Math.abs(y - nodeY) < hitRadius) {
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
		startRow: number,
		colXfn: (_col: number) => number,
		rowYfn: (_row: number) => number,
		sNodeRadius: number
	) {
		const x = colXfn(node.column);
		const y = rowYfn(node.row);

		if (node.oid === selectedOid) {
			ctx.beginPath();
			ctx.arc(x, y, sNodeRadius + 3, 0, Math.PI * 2);
			ctx.strokeStyle = '#60a5fa';
			ctx.lineWidth = 2;
			ctx.stroke();
		} else if (node.oid === comparisonOid) {
			ctx.beginPath();
			ctx.arc(x, y, sNodeRadius + 3, 0, Math.PI * 2);
			ctx.strokeStyle = '#a78bfa';
			ctx.lineWidth = 2;
			ctx.setLineDash([3, 3]);
			ctx.stroke();
			ctx.setLineDash([]);
		}

		ctx.globalAlpha = node.is_dimmed ? 0.35 : 1.0;
		ctx.beginPath();
		ctx.arc(x, y, sNodeRadius, 0, Math.PI * 2);
		ctx.fillStyle = colorToCss(node.color);
		ctx.fill();
		ctx.globalAlpha = 1.0;
	}

	function drawEdge(
		ctx: CanvasRenderingContext2D,
		edge: Edge,
		startRow: number,
		colXfn: (_col: number) => number,
		rowYfn: (_row: number) => number
	) {
		const x1 = colXfn(edge.from_col);
		const y1 = rowYfn(edge.from_row);
		const x2 = colXfn(edge.to_col);
		const y2 = rowYfn(edge.to_row);

		ctx.beginPath();
		ctx.globalAlpha = edge.is_dimmed ? 0.35 : 0.8;
		ctx.strokeStyle = colorToCss(edge.color);
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
		startRow: number,
		colXfn: (_col: number) => number,
		rowYfn: (_row: number) => number,
		sNodeRadius: number,
		sc: number
	) {
		const x = colXfn(stash.column) + sNodeRadius + 4 * sc;
		const y = rowYfn(stash.row);

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
