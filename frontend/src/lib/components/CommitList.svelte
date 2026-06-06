<script lang="ts">
	import { t } from '$lib/stores/locale';
	import type { CommitInfo, GraphLayout } from '$lib/bindings/types';
	import CommitGraph from './CommitGraph.svelte';
	import CommitRow from './CommitRow.svelte';

	interface Props {
		commits: CommitInfo[];
		layout: GraphLayout | null;
		selectedOid: string | null;
		matchingOids?: Set<string>;
		onSelect: (_oid: string, _ctrlKey: boolean) => void;
		onContextMenu?: (_e: MouseEvent, _oid: string) => void;
		rowHeight?: number;
		graphWidth?: number;
	}

	let {
		commits,
		layout,
		selectedOid,
		matchingOids,
		onSelect,
		onContextMenu,
		rowHeight = 28,
		graphWidth = 200
	}: Props = $props();

	let containerEl: HTMLDivElement;
	let scrollTop = $state(0);
	let scrollVersion = $state(0);
	let containerHeight = $state(800);
	let rafId = 0;
	let pendingScrollTop = 0;

	const BUFFER = 10;

	let commitsByOid = $derived(new Map(commits.map((c) => [c.oid, c])));

	let orderedCommits = $derived(
		layout
			? [...layout.nodes]
					.sort((a, b) => a.row - b.row)
					.map((n) => commitsByOid.get(n.oid))
					.filter((c): c is CommitInfo => c !== undefined)
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
				scrollVersion++;
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

	$effect(() => {
		if (!containerEl || !selectedOid || !orderedCommits.length) return;
		const idx = orderedCommits.findIndex((c) => c.oid === selectedOid);
		if (idx >= 0) scrollToIndex(idx);
	});

	function handleKeydown(e: KeyboardEvent) {
		if (!orderedCommits.length) return;
		const currentIdx = orderedCommits.findIndex((c) => c.oid === selectedOid);
		const lastIdx = orderedCommits.length - 1;
		const pageSize = Math.max(1, Math.floor(containerHeight / rowHeight));

		if (e.key === 'ArrowDown' || e.key === 'j') {
			e.preventDefault();
			const next = currentIdx < 0 ? 0 : Math.min(currentIdx + 1, lastIdx);
			onSelect(orderedCommits[next].oid, false);
			scrollToIndex(next);
		} else if (e.key === 'ArrowUp' || e.key === 'k') {
			e.preventDefault();
			const prev = currentIdx < 0 ? 0 : Math.max(currentIdx - 1, 0);
			onSelect(orderedCommits[prev].oid, false);
			scrollToIndex(prev);
		} else if (e.key === 'PageDown') {
			e.preventDefault();
			const next = currentIdx < 0 ? pageSize : Math.min(currentIdx + pageSize, lastIdx);
			onSelect(orderedCommits[next].oid, false);
			scrollToIndex(next);
		} else if (e.key === 'PageUp') {
			e.preventDefault();
			const prev = currentIdx < 0 ? 0 : Math.max(currentIdx - pageSize, 0);
			onSelect(orderedCommits[prev].oid, false);
			scrollToIndex(prev);
		} else if (e.key === 'Home') {
			e.preventDefault();
			onSelect(orderedCommits[0].oid, false);
			scrollToIndex(0);
		} else if (e.key === 'End') {
			e.preventDefault();
			onSelect(orderedCommits[lastIdx].oid, false);
			scrollToIndex(lastIdx);
		}
	}

	function scrollToIndex(idx: number) {
		const targetTop = idx * rowHeight;
		const targetBottom = targetTop + rowHeight;
		if (targetTop < scrollTop) {
			containerEl.scrollTop = targetTop;
		} else if (targetBottom > scrollTop + containerHeight) {
			containerEl.scrollTop = targetBottom - containerHeight;
		}
	}
</script>

<div class="flex h-full" role="listbox" aria-label={$t('commit_list.aria')}>
	<div
		bind:this={containerEl}
		class="flex-1 overflow-y-auto"
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
					<div class="shrink-0 overflow-hidden" style="width: {graphWidth}px;" aria-hidden="true">
						<CommitGraph
							{layout}
							{rowHeight}
							{visibleStart}
							{visibleEnd}
							{scrollVersion}
							{onSelect}
						/>
					</div>
				{/if}
				<div class="flex-1 min-w-0">
					{#each visibleCommits as commit (commit.oid)}
						<CommitRow
							id="commit-{commit.oid}"
							{commit}
							isSelected={commit.oid === selectedOid}
							isDimmed={matchingOids ? !matchingOids.has(commit.oid) : false}
							onclick={onSelect}
							oncontextmenu={onContextMenu}
						/>
					{/each}
				</div>
			</div>
		</div>
	</div>
</div>
