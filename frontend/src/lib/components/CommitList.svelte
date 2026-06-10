<script lang="ts">
	import { t } from '$lib/stores/locale';
	import type { CommitInfo, GraphLayout } from '$lib/bindings/types';
	import { searchResults } from '$lib/stores/repository';
	import CommitRow from './CommitRow.svelte';
	import GraphRenderer from './graph/GraphRenderer.svelte';

	interface Props {
		commits: CommitInfo[];
		layout: GraphLayout | null;
		selectedOid: string | null;
		comparisonOid?: string | null;
		matchingOids?: Set<string>;
		onSelect: (_oid: string, _ctrlKey: boolean) => void;
		onContextMenu?: (_e: MouseEvent, _oid: string) => void;
		rowHeight?: number;
		graphWidth?: number;
		onEdgeNavigate?: (_oid: string) => void;
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
		graphWidth = 200,
		onEdgeNavigate
	}: Props = $props();

	let highlightsByOid = $derived(new Map($searchResults.map((r) => [r.commit_oid, r.highlights])));

	let containerEl: HTMLDivElement;
	let scrollTop = $state(0);
	let containerHeight = $state(800);
	let rafId = 0;
	let pendingScrollTop = 0;

	const BUFFER = 10;

	let commitsByOid = $derived(new Map(commits.map((c) => [c.oid, c])));

	let orderedCommits = $derived(
		layout
			? (() => {
					const mapped = [...layout.nodes]
						.sort((a, b) => a.row - b.row)
						.map((n) => commitsByOid.get(n.oid));
					if (import.meta.env.DEV) {
						const missing = mapped.filter((c) => c === undefined).length;
						if (missing > 0) {
							console.warn(`orderedCommits: ${missing} layout nodes not found in commitsByOid`);
						}
					}
					return mapped.filter((c): c is CommitInfo => c !== undefined);
				})()
			: commits
	);

	let visibleStart = $derived(Math.max(0, Math.floor(scrollTop / rowHeight) - BUFFER));
	let visibleEnd = $derived(
		Math.min(orderedCommits.length, Math.ceil((scrollTop + containerHeight) / rowHeight) + BUFFER)
	);
	let totalHeight = $derived(orderedCommits.length * rowHeight);
	let visibleCommits = $derived(orderedCommits.slice(visibleStart, visibleEnd));

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
		}
	}

	$effect(() => {
		handleResize();
		const observer = new ResizeObserver(handleResize);
		if (containerEl) observer.observe(containerEl);
		return () => observer.disconnect();
	});

	let selectedIdx = $state(0);

	$effect(() => {
		if (!orderedCommits.length) return;
		if (!selectedOid) {
			selectedIdx = 0;
			return;
		}
		const idx = orderedCommits.findIndex((c) => c.oid === selectedOid);
		if (idx >= 0) selectedIdx = idx;
	});

	$effect(() => {
		if (!containerEl || !selectedOid || !orderedCommits.length) return;
		const idx = orderedCommits.findIndex((c) => c.oid === selectedOid);
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
			const next = Math.min(selectedIdx + 1, lastIdx);
			if (next === selectedIdx) return;
			selectedIdx = next;
			onSelect(orderedCommits[selectedIdx].oid, false);
			scrollToIndex(selectedIdx);
		} else if (e.key === 'ArrowUp' || e.key === 'k') {
			e.preventDefault();
			const prev = Math.max(selectedIdx - 1, 0);
			if (prev === selectedIdx) return;
			selectedIdx = prev;
			onSelect(orderedCommits[selectedIdx].oid, false);
			scrollToIndex(selectedIdx);
		} else if (e.key === 'PageDown') {
			e.preventDefault();
			const next = Math.min(selectedIdx + pageSize, lastIdx);
			if (next === selectedIdx) return;
			selectedIdx = next;
			onSelect(orderedCommits[selectedIdx].oid, false);
			scrollToIndex(selectedIdx);
		} else if (e.key === 'PageUp') {
			e.preventDefault();
			const prev = Math.max(selectedIdx - pageSize, 0);
			if (prev === selectedIdx) return;
			selectedIdx = prev;
			onSelect(orderedCommits[selectedIdx].oid, false);
			scrollToIndex(selectedIdx);
		} else if (e.key === 'Home') {
			e.preventDefault();
			selectedIdx = 0;
			onSelect(orderedCommits[0].oid, false);
			containerEl.scrollTop = 0;
		} else if (e.key === 'End') {
			e.preventDefault();
			selectedIdx = lastIdx;
			onSelect(orderedCommits[lastIdx].oid, false);
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
		const idx = orderedCommits.findIndex((c) => c.oid === oid);
		if (idx >= 0) scrollToIndex(idx, true);
	}
</script>

<div class="flex h-full min-h-0" role="listbox" aria-label={$t('commit_list.aria')}>
	<div
		bind:this={containerEl}
		class="flex-1 min-h-0 overflow-y-auto"
		onscroll={onScroll}
		onkeydown={handleKeydown}
		tabindex="0"
		role="listbox"
		aria-label={$t('commit_list.commits_aria')}
		aria-activedescendant={selectedOid ? `commit-${selectedOid}` : undefined}
	>
		<div style="height: {totalHeight}px; position: relative;">
			<div class="flex" style="transform: translateY({visibleStart * rowHeight}px);">
				{#if layout}
					<div
						class="shrink-0 overflow-x-auto overflow-y-hidden relative"
						style="width: {graphWidth}px;"
						aria-hidden="true"
					>
						<GraphRenderer
							{layout}
							{commits}
							{rowHeight}
							{visibleStart}
							{visibleEnd}
							{selectedOid}
							{comparisonOid}
							{onSelect}
							onEdgeNavigate={onEdgeNavigate ?? handleEdgeNavigate}
						/>
					</div>
				{/if}
				<div class="flex-1 min-w-0">
					{#each visibleCommits as commit (commit.oid)}
						<CommitRow
							id="commit-{commit.oid}"
							{commit}
							isSelected={commit.oid === selectedOid}
							isComparison={commit.oid === comparisonOid}
							isDimmed={matchingOids ? !matchingOids.has(commit.oid) : false}
							highlights={highlightsByOid.get(commit.oid)}
							onclick={onSelect}
							oncontextmenu={onContextMenu}
						/>
					{/each}
				</div>
			</div>
		</div>
	</div>
</div>
