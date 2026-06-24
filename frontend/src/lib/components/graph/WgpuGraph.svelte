<script lang="ts">
	import { invoke } from '@tauri-apps/api/core';
	import { SvelteMap } from 'svelte/reactivity';
	import type {
		GraphLayout,
		RenderGraphInput,
		RenderNode,
		RenderEdge,
		CommitInfo
	} from '$lib/bindings/types';
	import {
		columnCenterX,
		nodeCenterY,
		isEdgeVisible,
		nodeHitTest,
		colorToCSS,
		SELECT_RGB,
		COMPARISON_RGB
	} from '$lib/graph/graph-math';
	import {
		computeVisibleEdgeCoords,
		edgeHitTest,
		edgeFarOid,
		drawEdgeHighlight,
		drawEdgeEndpoints
	} from '$lib/graph/edge-interaction';
	import {
		GRAPH_PADDING_LEFT as PADDING_LEFT,
		GRAPH_EDGE_HIT_TOLERANCE as EDGE_HIT_TOLERANCE
	} from '$lib/constants';

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
		onEdgeNavigate?: (_oid: string) => void;
	}

	let {
		layout,
		commits,
		rowHeight = 28,
		laneWidth = 24,
		nodeRadius = 4,
		visibleStart,
		visibleEnd,
		selectedOid = null,
		comparisonOid = null,
		onSelect,
		onEdgeNavigate
	}: Props = $props();

	let canvasEl: HTMLCanvasElement;
	let ctx: CanvasRenderingContext2D | null = $state(null);
	let dpr = $state(1);
	let baseImageData: ImageData | null = null;

	let containerWidth = $derived(Math.round(layout.total_columns * laneWidth + PADDING_LEFT + 10));
	let containerHeight = $derived(Math.round((visibleEnd - visibleStart) * rowHeight));

	let commitMap = $derived(new Map(commits.map((c) => [c.oid, c])));

	let hoveredEdgeIdx = $state<number | null>(null);
	let selectedEdgeIdx = $state<number | null>(null);

	let visibleEdgeData = $derived(
		computeVisibleEdgeCoords(layout, visibleStart, visibleEnd, rowHeight, laneWidth, PADDING_LEFT)
	);

	let visibleStashLabels = $derived.by(() => {
		const stashIdxMap = new Map(layout.stash_markers.map((s) => [s.stash_oid, s.stash_index]));
		return layout.nodes
			.filter((n) => n.is_stash && n.row >= visibleStart && n.row <= visibleEnd)
			.map((n) => ({
				index: stashIdxMap.get(n.oid) ?? 0,
				left: Math.round(columnCenterX(n.column, laneWidth, PADDING_LEFT) + nodeRadius + 2),
				top: Math.round(nodeCenterY(n.row, visibleStart, rowHeight))
			}));
	});

	let tooltip = $state<{ x: number; y: number; text: string } | null>(null);

	let nodeHitMap = new SvelteMap<string, { x: number; y: number; radius: number }>();

	function buildNodes(start: number, end: number): RenderNode[] {
		const result: RenderNode[] = [];
		nodeHitMap = new SvelteMap();
		for (const n of layout.nodes) {
			if (n.row < start || n.row > end) continue;
			const x = columnCenterX(n.column, laneWidth, PADDING_LEFT);
			const y = nodeCenterY(n.row, start, rowHeight);
			const [cr, cg, cb, ca] = [n.color.r, n.color.g, n.color.b, n.color.a];
			const [sr, sg, sb] = n.oid === comparisonOid ? COMPARISON_RGB : SELECT_RGB;
			result.push({
				row: n.row,
				column: n.column,
				color_r: cr,
				color_g: cg,
				color_b: cb,
				color_a: ca,
				is_dimmed: n.is_dimmed,
				is_selected: n.oid === selectedOid,
				is_comparison: n.oid === comparisonOid,
				is_merge: n.is_merge,
				is_stash: n.is_stash,
				sel_color_r: sr,
				sel_color_g: sg,
				sel_color_b: sb
			});
			nodeHitMap.set(n.oid, { x, y, radius: nodeRadius });
		}
		return result;
	}

	function buildEdges(
		start: number,
		end: number,
		skipIdx1: number | null = null,
		skipIdx2: number | null = null
	): RenderEdge[] {
		const result: RenderEdge[] = [];
		for (let i = 0; i < layout.edges.length; i++) {
			const e = layout.edges[i];
			if (!isEdgeVisible(e, start, end)) continue;
			if (i === skipIdx1 || i === skipIdx2) continue;
			result.push({
				from_row: e.from_row,
				from_col: e.from_col,
				to_row: e.to_row,
				to_col: e.to_col,
				color_r: e.color.r,
				color_g: e.color.g,
				color_b: e.color.b,
				is_dimmed: e.is_dimmed,
				edge_type: e.edge_type,
				edge_style: e.edge_style
			});
		}
		return result;
	}

	let pendingRender = 0;

	function scheduleRender() {
		cancelAnimationFrame(pendingRender);
		pendingRender = requestAnimationFrame(doRender);
	}

	async function doRender() {
		if (!canvasEl) return;
		dpr = devicePixelRatio || 1;
		const width = Math.round(layout.total_columns * laneWidth + PADDING_LEFT + 10);
		const height = Math.round((visibleEnd - visibleStart) * rowHeight);
		if (width < 1 || height < 1) return;

		const physW = Math.round(width * dpr);
		const physH = Math.round(height * dpr);

		if (canvasEl.width !== physW || canvasEl.height !== physH) {
			canvasEl.width = physW;
			canvasEl.height = physH;
			canvasEl.style.width = width + 'px';
			canvasEl.style.height = height + 'px';
		}

		const input: RenderGraphInput = {
			width: physW,
			height: physH,
			scale: dpr,
			visible_start: visibleStart,
			visible_end: visibleEnd,
			total_columns: layout.total_columns,
			row_height: rowHeight,
			lane_width: laneWidth,
			padding_left: PADDING_LEFT,
			node_radius: nodeRadius,
			nodes: buildNodes(visibleStart, visibleEnd),
			edges: buildEdges(visibleStart, visibleEnd)
		};

		try {
			const pixels = await invoke('render_graph', { input });
			if (!canvasEl) return;
			ctx ??= canvasEl.getContext('2d');
			if (!ctx) return;

			const imageData = ctx.createImageData(physW, physH);
			const data = imageData.data;
			const src =
				pixels instanceof Uint8Array
					? pixels
					: pixels instanceof ArrayBuffer
						? new Uint8Array(pixels)
						: new Uint8Array(pixels as number[]);
			const len = Math.min(src.length, data.length);
			data.set(src.subarray(0, len));
			ctx.putImageData(imageData, 0, 0);
			baseImageData = imageData;

			overdrawEdgeHighlights();
		} catch (err) {
			console.warn('wgpu render failed, falling back:', err);
		}
	}

	function overdrawEdgeHighlights() {
		if (!ctx || (hoveredEdgeIdx === null && selectedEdgeIdx === null)) return;
		ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
		for (const { edge, idx, coords } of visibleEdgeData) {
			if (idx === selectedEdgeIdx) {
				drawEdgeHighlight(ctx, coords, colorToCSS(edge.color), 3.5);
				drawEdgeEndpoints(ctx, coords, colorToCSS(edge.color), nodeRadius);
			} else if (idx === hoveredEdgeIdx) {
				drawEdgeHighlight(ctx, coords, colorToCSS(edge.color), 2.5);
				drawEdgeEndpoints(ctx, coords, colorToCSS(edge.color), nodeRadius);
			}
		}
	}

	$effect(() => {
		void visibleStart;
		void visibleEnd;
		void layout;
		void selectedOid;
		void comparisonOid;
		void rowHeight;
		void laneWidth;
		void nodeRadius;
		scheduleRender();
	});

	$effect(() => {
		void hoveredEdgeIdx;
		void selectedEdgeIdx;
		if (!ctx || !baseImageData) return;
		if (baseImageData.width !== ctx.canvas.width || baseImageData.height !== ctx.canvas.height)
			return;
		ctx.putImageData(baseImageData, 0, 0);
		overdrawEdgeHighlights();
	});

	$effect(() => {
		return () => cancelAnimationFrame(pendingRender);
	});

	function handleEdgeClick(mx: number, my: number): boolean {
		for (const { edge, idx, coords } of visibleEdgeData) {
			if (edgeHitTest(mx, my, coords, EDGE_HIT_TOLERANCE)) {
				if (selectedEdgeIdx === idx) {
					const farOid = edgeFarOid(edge, layout, selectedOid ?? null);
					if (farOid) {
						onEdgeNavigate?.(farOid);
						onSelect?.(farOid, false);
					}
					selectedEdgeIdx = null;
				} else {
					selectedEdgeIdx = idx;
				}
				return true;
			}
		}
		return false;
	}

	function handleClick(e: MouseEvent) {
		const rect = canvasEl.getBoundingClientRect();
		const mx = e.clientX - rect.left;
		const my = e.clientY - rect.top;
		for (const [oid, pos] of nodeHitMap) {
			if (nodeHitTest(mx, my, pos.x, pos.y, pos.radius)) {
				selectedEdgeIdx = null;
				onSelect?.(oid, e.ctrlKey || e.metaKey);
				return;
			}
		}
		if (!handleEdgeClick(mx, my)) {
			selectedEdgeIdx = null;
		}
	}

	function handleMouseMove(e: MouseEvent) {
		const rect = canvasEl.getBoundingClientRect();
		const mx = e.clientX - rect.left;
		const my = e.clientY - rect.top;
		const row = Math.floor(my / rowHeight) + visibleStart;
		const hitRadius = nodeRadius + 4;

		for (const n of layout.nodes) {
			if (n.row === row) {
				const nx = columnCenterX(n.column, laneWidth, PADDING_LEFT);
				const ny = nodeCenterY(n.row, visibleStart, rowHeight);
				if (Math.abs(mx - nx) < hitRadius && Math.abs(my - ny) < hitRadius) {
					if (n.is_stash) {
						const stash = layout.stash_markers.find((s) => s.stash_oid === n.oid);
						tooltip = {
							x: e.clientX - rect.left + 12,
							y: e.clientY - rect.top - 8,
							text: stash?.message ?? n.oid.substring(0, 7) + ' stash'
						};
					} else {
						const ci = commitMap.get(n.oid);
						if (ci) {
							tooltip = {
								x: e.clientX - rect.left + 12,
								y: e.clientY - rect.top - 8,
								text: `${ci.short_oid} ${ci.summary}`
							};
						}
					}
					hoveredEdgeIdx = null;
					return;
				}
			}
		}

		let newHovered: number | null = null;
		for (const { idx, coords } of visibleEdgeData) {
			if (edgeHitTest(mx, my, coords, EDGE_HIT_TOLERANCE)) {
				newHovered = idx;
				break;
			}
		}
		hoveredEdgeIdx = newHovered;
		tooltip = null;
	}

	function handleMouseLeave() {
		tooltip = null;
		hoveredEdgeIdx = null;
	}
</script>

<div class="relative" style="width: {containerWidth}px; height: {containerHeight}px;">
	<canvas
		bind:this={canvasEl}
		onclick={handleClick}
		onmousemove={handleMouseMove}
		onmouseleave={handleMouseLeave}
		class="block absolute inset-0"
		style="image-rendering: auto; width: {containerWidth}px; height: {containerHeight}px;"
	></canvas>
	{#if tooltip}
		<div
			class="pointer-events-none absolute z-50 max-w-[250px] rounded bg-gray-800 px-2 py-1 text-xs text-gray-200 shadow-lg border border-gray-700"
			style="left: {tooltip.x}px; top: {tooltip.y}px;"
		>
			{tooltip.text}
		</div>
	{/if}
	<div class="absolute inset-0" style="pointer-events: none;">
		{#each visibleStashLabels as s (s.index)}
			<span
				class="absolute text-xs font-bold font-mono leading-none select-none"
				style="left: {s.left}px; top: {s.top}px; transform: translateY(-50%); color: #f59e0b;"
				>S{s.index}</span
			>
		{/each}
	</div>
</div>
