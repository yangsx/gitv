<script lang="ts">
	import { invoke } from '@tauri-apps/api/core';
	import type { GraphLayout, RenderGraphInput, RenderNode, RenderEdge, CommitInfo } from '$lib/bindings/types';
	import { showStashes } from '$lib/stores/preferences';
	import {
		columnCenterX,
		nodeCenterY,
		stashX,
		stashY,
		isEdgeVisible,
		nodeHitTest,
		SELECT_RGB,
		COMPARISON_RGB
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
		selectedOid = null,
		comparisonOid = null,
		onSelect,
		onStashSelect
	}: Props = $props();

	let canvasEl: HTMLCanvasElement;
	let ctx: CanvasRenderingContext2D | null = $state(null);
	let dpr = $state(1);

	let containerWidth = $derived(Math.round(layout.total_columns * laneWidth + PADDING_LEFT + 10));
	let containerHeight = $derived(Math.round((visibleEnd - visibleStart) * rowHeight));

	let visibleStashes = $derived.by(() => {
		if (!$showStashes) return [];
		return layout.stash_markers
			.filter((s) => s.row >= visibleStart && s.row <= visibleEnd)
			.map((s) => ({
				index: s.stash_index,
				left: Math.round(stashX(s.column, laneWidth, PADDING_LEFT, nodeRadius)),
				top: Math.round(stashY(s.row, visibleStart, rowHeight))
			}));
	});

	let commitMap = $derived(new Map(commits.map((c) => [c.oid, c])));

	let tooltip = $state<{ x: number; y: number; text: string } | null>(null);

	// Build a lookup from node OID to its render data for hit testing
	let nodeHitMap = $state(new Map<string, { x: number; y: number; radius: number }>());

	function buildNodes(start: number, end: number): RenderNode[] {
		const result: RenderNode[] = [];
		nodeHitMap = new Map();
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
				sel_color_r: sr,
				sel_color_g: sg,
				sel_color_b: sb
			});
			nodeHitMap.set(n.oid, { x, y, radius: nodeRadius });
		}
		return result;
	}

	function buildEdges(start: number, end: number): RenderEdge[] {
		const result: RenderEdge[] = [];
		for (const e of layout.edges) {
			if (!isEdgeVisible(e, start, end)) continue;
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
			let src: Uint8Array | number[];
			if (pixels instanceof Uint8Array) {
				src = pixels;
			} else if (pixels instanceof ArrayBuffer) {
				src = new Uint8Array(pixels);
			} else {
				src = pixels as number[];
			}
			for (let i = 0; i < src.length && i < data.length; i++) {
				data[i] = src[i];
			}
			ctx.putImageData(imageData, 0, 0);
		} catch (err) {
			console.warn('wgpu render failed, falling back:', err);
		}
	}

	$effect(() => {
		void visibleStart;
		void visibleEnd;
		void layout;
		void selectedOid;
		void comparisonOid;
		scheduleRender();
	});

	function handleClick(e: MouseEvent) {
		const rect = canvasEl.getBoundingClientRect();
		const mx = e.clientX - rect.left;
		const my = e.clientY - rect.top;
		for (const [oid, pos] of nodeHitMap) {
			if (nodeHitTest(mx, my, pos.x, pos.y, pos.radius)) {
				onSelect?.(oid, e.ctrlKey || e.metaKey);
				return;
			}
		}
	}

	function handleMouseMove(e: MouseEvent) {
		const rect = canvasEl.getBoundingClientRect();
		const mx = e.clientX - rect.left;
		const my = e.clientY - rect.top;
		const row = Math.floor(my / rowHeight) + visibleStart;
		const hitRadius = nodeRadius + 4;

		if ($showStashes) {
			for (const stash of layout.stash_markers) {
				if (stash.row === row) {
					const sx = stashX(stash.column, laneWidth, PADDING_LEFT, nodeRadius);
					const sy = stashY(stash.row, visibleStart, rowHeight);
					if (Math.abs(mx - sx) < hitRadius && Math.abs(my - sy) < hitRadius) {
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

		for (const n of layout.nodes) {
			if (n.row === row) {
				const nx = columnCenterX(n.column, laneWidth, PADDING_LEFT);
				const ny = nodeCenterY(n.row, visibleStart, rowHeight);
				if (Math.abs(mx - nx) < hitRadius && Math.abs(my - ny) < hitRadius) {
					const ci = commitMap.get(n.oid);
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
	{#if $showStashes}
		<div class="absolute inset-0" style="pointer-events: none;">
			{#each visibleStashes as s (s.index)}
				<span
					class="absolute text-xs font-bold font-mono leading-none cursor-pointer select-none"
					style="left: {s.left}px; top: {s.top}px; transform: translateY(-50%); color: #f59e0b; pointer-events: auto;"
					onclick={() => onStashSelect?.(s.index)}
					onkeydown={(e) => {
						if (e.key === 'Enter' || e.key === ' ') {
							e.preventDefault();
							onStashSelect?.(s.index);
						}
					}}
					role="button"
					tabindex="-1"
					aria-label="Stash {s.index}">S{s.index}</span
				>
			{/each}
		</div>
	{/if}
</div>
