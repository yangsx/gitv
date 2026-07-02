<script lang="ts">
	import { t } from '$lib/stores/locale';
	import type { CommitInfo, GraphLayout } from '$lib/bindings/types';
	import { searchResults } from '$lib/stores/repository';
	import { debug } from '$lib/stores/debug';
	import {
		GRAPH_PADDING_LEFT,
		GRAPH_LANE_WIDTH,
		HASH_COLUMN_WIDTH,
		AUTHOR_COLUMN_WIDTH,
		DATE_COLUMN_WIDTH,
		GRAPH_GAP,
		MIN_TEXT_WIDTH
	} from '$lib/constants';
	import CommitRow from './CommitRow.svelte';
	import CommitGraph from './CommitGraph.svelte';

	interface Props {
		commits: CommitInfo[];
		layout: GraphLayout | null;
		selectedOid: string | null;
		comparisonOid?: string | null;
		matchingOids?: Set<string>;
		onSelect: (_oid: string, _ctrlKey: boolean) => void;
		onContextMenu?: (_e: MouseEvent, _oid: string) => void;
		rowHeight?: number;
		onEdgeNavigate?: (_oid: string) => void;
		onLoadMore?: () => void;
		loadedCommitCount?: number;
	}

	let {
		commits,
		layout,
		selectedOid,
		comparisonOid = null,
		matchingOids,
		onSelect,
		onContextMenu,
		rowHeight = 28,
		onLoadMore,
		loadedCommitCount,
		onEdgeNavigate
	}: Props = $props();

	let highlightsByOid = $derived(new Map($searchResults.map((r) => [r.commit_oid, r.highlights])));
	let matchTypeByOid = $derived(new Map($searchResults.map((r) => [r.commit_oid, r.match_type])));

	let containerEl: HTMLDivElement;
	let scrollTop = $state(0);
	let containerHeight = $state(800);
	let containerWidth = $state(800);
	let rafId = 0;
	let pendingScrollTop = 0;

	let graphTooltip = $state<{ x: number; y: number; text: string } | null>(null);

	const BUFFER = 10;

	let commitsByOid = $derived(new Map(commits.map((c) => [c.oid, c])));

	let sortedLayoutNodes = $derived(layout ? [...layout.nodes].sort((a, b) => a.row - b.row) : null);

	let orderedCommits = $derived(
		sortedLayoutNodes
			? (sortedLayoutNodes.map((n) => commitsByOid.get(n.oid) ?? null) as (CommitInfo | null)[])
			: (commits as (CommitInfo | null)[])
	);

	let effectiveTotalRows = $derived(orderedCommits.length);

	let viewportRows = $derived(Math.ceil(containerHeight / rowHeight) + 2 * BUFFER);
	let visibleStart = $derived(Math.max(0, Math.floor(scrollTop / rowHeight) - BUFFER));
	let visibleEnd = $derived(Math.min(effectiveTotalRows, visibleStart + viewportRows));
	let totalHeight = $derived(effectiveTotalRows * rowHeight);
	let visibleCommits = $derived(orderedCommits.slice(visibleStart, visibleEnd));
	let visibleHeight = $derived((visibleEnd - visibleStart) * rowHeight);

	const LOAD_MORE_BUFFER = 200;

	$effect(() => {
		if (!onLoadMore) return;
		const loaded = loadedCommitCount ?? commits.length;
		if (loaded >= effectiveTotalRows) return;
		if (visibleEnd > loaded - LOAD_MORE_BUFFER) {
			onLoadMore();
		}
	});

	let maxGraphWidth = $derived(
		layout
			? layout.total_columns * GRAPH_LANE_WIDTH + GRAPH_PADDING_LEFT + GRAPH_GAP
			: GRAPH_PADDING_LEFT + GRAPH_GAP
	);

	let contentWidth = $derived(
		Math.max(
			HASH_COLUMN_WIDTH + maxGraphWidth + MIN_TEXT_WIDTH + AUTHOR_COLUMN_WIDTH + DATE_COLUMN_WIDTH,
			containerWidth
		)
	);

	function rowGraphOffset(row: number): number {
		if (!layout) return GRAPH_PADDING_LEFT + GRAPH_GAP;
		const maxCol = layout.row_max_column[row] ?? layout.total_columns;
		return maxCol * GRAPH_LANE_WIDTH + GRAPH_PADDING_LEFT + GRAPH_GAP;
	}

	let lastDebugUpdate = 0;
	$effect(() => {
		void effectiveTotalRows;
		void visibleCommits;
		const now = performance.now();
		if (now - lastDebugUpdate < 200) return;
		lastDebugUpdate = now;
		debug.update((d) => ({
			...d,
			totalCommits: effectiveTotalRows,
			visibleCommits: visibleCommits.filter((c) => c !== null).length
		}));
	});

	function onScroll(e: Event) {
		const el = e.target as HTMLDivElement;
		pendingScrollTop = el.scrollTop;
		if (!rafId) {
			rafId = requestAnimationFrame(() => {
				scrollTop = pendingScrollTop;
				rafId = 0;
			});
		}
	}

	function handleResize() {
		if (containerEl) {
			containerHeight = containerEl.clientHeight;
			containerWidth = containerEl.clientWidth;
		}
	}

	$effect(() => {
		handleResize();
		const observer = new ResizeObserver(handleResize);
		if (containerEl) observer.observe(containerEl);
		return () => observer.disconnect();
	});

	$effect(() => {
		return () => {
			if (rafId) cancelAnimationFrame(rafId);
		};
	});

	let selectedIdx = $state(0);

	function findLoadedNeighbor(start: number, direction: 1 | -1): number {
		let i = start + direction;
		while (i >= 0 && i < orderedCommits.length) {
			if (orderedCommits[i] !== null) return i;
			i += direction;
		}
		return -1;
	}

	$effect(() => {
		if (!orderedCommits.length) return;
		if (!selectedOid) {
			selectedIdx = 0;
			return;
		}
		const idx = orderedCommits.findIndex((c) => c?.oid === selectedOid);
		if (idx >= 0) selectedIdx = idx;
		else selectedIdx = 0;
	});

	$effect(() => {
		if (!containerEl || !selectedOid || !orderedCommits.length) return;
		const idx = orderedCommits.findIndex((c) => c?.oid === selectedOid);
		if (idx < 0) return;
		const targetTop = idx * rowHeight;
		const viewTop = containerEl.scrollTop;
		const viewBottom = viewTop + containerEl.clientHeight;
		if (targetTop < viewTop || targetTop + rowHeight > viewBottom) {
			scrollToIndex(idx, true);
		}
	});

	function handleKeydown(e: KeyboardEvent) {
		if (!orderedCommits.length) return;
		if (containerEl && document.activeElement !== containerEl) {
			containerEl.focus();
		}
		const lastIdx = orderedCommits.length - 1;
		const pageSize = Math.max(1, Math.floor(containerHeight / rowHeight));

		if (e.key === 'ArrowDown' || e.key === 'j') {
			e.preventDefault();
			const next = findLoadedNeighbor(selectedIdx, 1);
			if (next < 0 || next === selectedIdx) return;
			selectedIdx = next;
			onSelect(orderedCommits[selectedIdx]!.oid, false);
			scrollToIndex(selectedIdx);
		} else if (e.key === 'ArrowUp' || e.key === 'k') {
			e.preventDefault();
			const prev = findLoadedNeighbor(selectedIdx, -1);
			if (prev < 0 || prev === selectedIdx) return;
			selectedIdx = prev;
			onSelect(orderedCommits[selectedIdx]!.oid, false);
			scrollToIndex(selectedIdx);
		} else if (e.key === 'PageDown') {
			e.preventDefault();
			let target = Math.min(selectedIdx + pageSize, lastIdx);
			while (target < lastIdx && orderedCommits[target] === null) target++;
			if (orderedCommits[target] === null) return;
			selectedIdx = target;
			onSelect(orderedCommits[selectedIdx]!.oid, false);
			scrollToIndex(selectedIdx);
		} else if (e.key === 'PageUp') {
			e.preventDefault();
			let target = Math.max(selectedIdx - pageSize, 0);
			while (target > 0 && orderedCommits[target] === null) target--;
			if (orderedCommits[target] === null) return;
			selectedIdx = target;
			onSelect(orderedCommits[selectedIdx]!.oid, false);
			scrollToIndex(selectedIdx);
		} else if (e.key === 'Home') {
			e.preventDefault();
			let first = 0;
			while (first < lastIdx && orderedCommits[first] === null) first++;
			if (orderedCommits[first] === null) return;
			selectedIdx = first;
			onSelect(orderedCommits[first]!.oid, false);
			containerEl.scrollTop = 0;
		} else if (e.key === 'End') {
			e.preventDefault();
			let last = lastIdx;
			while (last > 0 && orderedCommits[last] === null) last--;
			if (orderedCommits[last] === null) return;
			selectedIdx = last;
			onSelect(orderedCommits[last]!.oid, false);
			containerEl.scrollTop = containerEl.scrollHeight - containerEl.clientHeight;
		}
	}

	function scrollToIndex(idx: number, center = false) {
		const targetTop = idx * rowHeight;
		if (center) {
			containerEl.scrollTop =
				targetTop - Math.floor(containerEl.clientHeight / 2) + Math.floor(rowHeight / 2);
		} else {
			const viewBottom = containerEl.scrollTop + containerEl.clientHeight;
			if (targetTop < containerEl.scrollTop) {
				containerEl.scrollTop = targetTop;
			} else if (targetTop + rowHeight > viewBottom) {
				containerEl.scrollTop = targetTop + rowHeight - containerEl.clientHeight;
			}
		}
	}

	function handleEdgeNavigate(oid: string) {
		const idx = orderedCommits.findIndex((c) => c?.oid === oid);
		if (idx >= 0) scrollToIndex(idx, true);
	}

	function handleGraphTooltip(data: { x: number; y: number; text: string } | null) {
		graphTooltip = data;
	}
</script>

<div class="flex h-full min-h-0" role="listbox" aria-label={$t('commit_list.aria')}>
	<div
		bind:this={containerEl}
		class="flex-1 min-h-0 overflow-auto"
		onscroll={onScroll}
		onkeydown={handleKeydown}
		tabindex="0"
		role="listbox"
		aria-label={$t('commit_list.commits_aria')}
		aria-activedescendant={selectedOid ? `commit-${selectedOid}` : undefined}
	>
		<div style="height: {totalHeight}px; width: {contentWidth}px; position: relative;">
			<div
				class="relative"
				style="transform: translateY({visibleStart * rowHeight}px); width: {contentWidth}px;"
			>
				{#if layout}
					<div
						class="absolute top-0 overflow-hidden"
						style="left: {HASH_COLUMN_WIDTH}px; width: {maxGraphWidth}px; height: {visibleHeight}px; z-index: 0;"
						aria-hidden="true"
					>
						<CommitGraph
							{layout}
							{commits}
							{rowHeight}
							{visibleStart}
							{visibleEnd}
							{selectedOid}
							{comparisonOid}
							{onSelect}
							onEdgeNavigate={onEdgeNavigate ?? handleEdgeNavigate}
							onTooltip={handleGraphTooltip}
							visibleWidth={maxGraphWidth}
						/>
					</div>
				{/if}
				{#each visibleCommits as commit, i (commit?.oid ?? `placeholder-${visibleStart + i}`)}
					{#if commit}
						<CommitRow
							id="commit-{commit.oid}"
							{commit}
							isSelected={commit.oid === selectedOid}
							isComparison={commit.oid === comparisonOid}
							isDimmed={matchingOids ? !matchingOids.has(commit.oid) : false}
							highlights={highlightsByOid.get(commit.oid)}
							matchType={matchTypeByOid.get(commit.oid)}
							onclick={onSelect}
							oncontextmenu={onContextMenu}
							{rowHeight}
							graphOffset={rowGraphOffset(visibleStart + i)}
						/>
					{:else}
						<div class="flex w-full items-center" style="height: {rowHeight}px;">
							<span
								class="sticky left-0 z-10 shrink-0 bg-gray-800"
								style="width: {HASH_COLUMN_WIDTH}px;"
							></span>
							<div class="shrink-0" style="width: {rowGraphOffset(visibleStart + i)}px;"></div>
							<span class="flex-1 bg-gray-800"></span>
							<span
								class="sticky z-10 shrink-0 bg-gray-800"
								style="right: {DATE_COLUMN_WIDTH}px; width: {AUTHOR_COLUMN_WIDTH}px;"
							></span>
							<span
								class="sticky right-0 z-10 shrink-0 bg-gray-800"
								style="width: {DATE_COLUMN_WIDTH}px;"
							></span>
						</div>
					{/if}
				{/each}
			</div>
		</div>
	</div>
</div>

{#if graphTooltip}
	<div
		class="pointer-events-none fixed z-50 max-w-[250px] rounded bg-gray-800 px-2 py-1 text-xs text-gray-200 shadow-lg border border-gray-700"
		style="left: {graphTooltip.x}px; top: {graphTooltip.y}px;"
	>
		{graphTooltip.text}
	</div>
{/if}
