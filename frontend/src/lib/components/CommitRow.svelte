<script lang="ts">
	import { t } from '$lib/stores/locale';
	import type { CommitInfo, Ref, Highlight, MatchType } from '$lib/bindings/types';
	import { commitSummary } from '$lib/bindings/types';
	import {
		STAGED_OID,
		UNSTAGED_OID,
		HASH_COLUMN_WIDTH,
		AUTHOR_COLUMN_WIDTH,
		DATE_COLUMN_WIDTH
	} from '$lib/constants';
	import { formatGitDateTime } from '$lib/utils/format-date';

	interface Props {
		commit: CommitInfo;
		isSelected: boolean;
		isComparison?: boolean;
		isDimmed?: boolean;
		highlights?: Highlight[];
		matchType?: MatchType;
		onclick: (_oid: string, _ctrlKey: boolean) => void;
		oncontextmenu?: (_e: MouseEvent, _oid: string) => void;
		id?: string;
		rowHeight?: number;
		graphOffset?: number;
		rowIndex?: number;
		rowNumberColumnWidth?: number;
		hashColumnWidth?: number;
	}

	let {
		commit,
		isSelected,
		isComparison = false,
		isDimmed = false,
		highlights = [],
		matchType,
		onclick,
		oncontextmenu,
		id,
		rowHeight = 28,
		graphOffset = 0,
		rowIndex,
		rowNumberColumnWidth = 0,
		hashColumnWidth = HASH_COLUMN_WIDTH
	}: Props = $props();

	function escapeHtml(s: string): string {
		return s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
	}

	function renderSummary(): string {
		const summary = commitSummary(commit);
		if (!highlights || highlights.length === 0) return escapeHtml(summary);
		const parts: string[] = [];
		let lastEnd = 0;
		const sorted = [...highlights].sort((a, b) => a.start - b.start);
		for (const h of sorted) {
			if (h.start >= summary.length) break;
			if (h.start > lastEnd) {
				parts.push(escapeHtml(summary.slice(lastEnd, h.start)));
			}
			const end = Math.min(h.start + h.length, summary.length);
			if (end > h.start) {
				parts.push('<mark class="bg-yellow-500/40 rounded px-0.5">');
				parts.push(escapeHtml(summary.slice(h.start, end)));
				parts.push('</mark>');
			}
			lastEnd = Math.max(lastEnd, end);
		}
		if (lastEnd < summary.length) {
			parts.push(escapeHtml(summary.slice(lastEnd)));
		}
		return parts.join('');
	}

	// svelte-ignore state_referenced_locally
	const isVirtual = commit.oid === STAGED_OID || commit.oid === UNSTAGED_OID;
	// svelte-ignore state_referenced_locally
	const isStaged = commit.oid === STAGED_OID;

	function refLabel(r: Ref): string | null {
		if (r.Branch) return r.Branch.is_head ? `(${r.Branch.name})` : r.Branch.name;
		if (r.Tag) return r.Tag.name;
		if (r.Remote) return `${r.Remote.remote}/${r.Remote.name}`;
		return null;
	}

	function refColorClass(r: Ref): string {
		if (r.Branch?.is_head) return 'text-green-400 font-medium';
		if (r.Branch) return 'text-green-600';
		if (r.Tag) return 'text-yellow-400';
		if (r.Remote) return 'text-gray-500';
		return 'text-gray-400';
	}

	let rowBgClass = $derived(
		isSelected ? 'bg-blue-900/40' : isComparison ? 'bg-indigo-900/40' : 'bg-gray-800'
	);

	let rowHoverClass = $derived(isSelected || isComparison ? '' : 'group-hover:bg-gray-700');

	let textClass = $derived(
		isSelected
			? 'text-blue-200'
			: isComparison
				? 'text-indigo-200'
				: isDimmed
					? 'text-gray-600'
					: 'text-gray-300'
	);
</script>

<button
	{id}
	tabindex="-1"
	role="option"
	aria-selected={isSelected}
	class="group flex w-full items-center text-left text-sm focus:outline-none"
	style="height: {rowHeight}px;"
	aria-label={isVirtual
		? $t(isStaged ? 'page.staged' : 'page.unstaged')
		: $t('commit_row.aria_label', {
				summary: commitSummary(commit),
				author: commit.author.name,
				date: formatGitDateTime(commit.commit_time)
			}) +
			(commit.refs.length > 0 ? ', ' + commit.refs.map(refLabel).filter(Boolean).join(', ') : '')}
	onclick={(e: Event & { ctrlKey?: boolean; metaKey?: boolean }) =>
		onclick(commit.oid, !!(e.ctrlKey || e.metaKey))}
	oncontextmenu={(e: MouseEvent) => oncontextmenu?.(e, commit.oid)}
>
	<!-- Sticky row-number column (frozen left, hover-reveal) -->
	{#if rowNumberColumnWidth > 0}
		<span
			class="sticky left-0 z-10 flex shrink-0 items-center justify-end {rowBgClass} {rowHoverClass}"
			style="width: {rowNumberColumnWidth}px;"
		>
			{#if rowIndex !== undefined}
				<span
					class="px-1 font-mono text-xs text-gray-500 opacity-0 transition-opacity duration-100 group-hover:opacity-100"
					style="font-variant-numeric: tabular-nums;"
				>
					{rowIndex}
				</span>
			{/if}
		</span>
	{/if}

	<!-- Sticky hash column (frozen left, offset by row-number gutter) -->
	<span
		class="sticky z-10 flex shrink-0 items-center {rowBgClass} {rowHoverClass}"
		style="left: {rowNumberColumnWidth}px; width: {hashColumnWidth}px;"
	>
		{#if isVirtual}
			<span
				class="ml-2 inline-block h-2 w-2 rounded-full {isStaged ? 'bg-green-400' : 'bg-orange-400'}"
				aria-hidden="true"
			></span>
		{:else}
			<span class="px-2 font-mono text-xs text-gray-500">
				{commit.short_oid}
			</span>
		{/if}
	</span>

	<!-- Graph spacer (transparent — canvas shows through) -->
	<div
		class="shrink-0"
		style="width: {graphOffset}px; pointer-events: none;"
		aria-hidden="true"
	></div>

	<!-- Flowing text: inline refs + message -->
	<span
		class="relative z-[1] flex min-w-0 flex-1 items-center gap-1 truncate py-0.5 pl-1 pr-2 {rowBgClass} {rowHoverClass} {textClass}"
	>
		{#if isVirtual}
			<span class="truncate font-medium {isStaged ? 'text-green-300' : 'text-orange-300'}">
				{$t(isStaged ? 'page.staged' : 'page.unstaged')}
			</span>
		{:else}
			{#each commit.refs as ref (ref.Branch ? 'b:' + ref.Branch.name : ref.Tag ? 't:' + ref.Tag.name : ref.Remote ? 'r:' + ref.Remote.remote + '/' + ref.Remote.name : '')}
				{#if ref.Remote}
					<span class="shrink-0 text-xs">
						<span class="text-cyan-400">{ref.Remote.remote}/</span>
						<span class="text-green-600">{ref.Remote.name}</span>
					</span>
				{:else}
					{@const label = refLabel(ref)}
					{#if label}
						<span class="shrink-0 text-xs {refColorClass(ref)}">
							{label}
						</span>
					{/if}
				{/if}
			{/each}
			<span class="min-w-0 truncate">
				<!-- eslint-disable-next-line svelte/no-at-html-tags -->
				{@html renderSummary()}
			</span>
			{#if matchType === 'Patch'}
				<span
					class="shrink-0 text-xs text-yellow-400"
					title={$t('commit_row.patch_match')}
					aria-label={$t('commit_row.patch_match')}
				>
					⌗
				</span>
			{/if}
		{/if}
	</span>

	<!-- Sticky author column (frozen right) -->
	<span
		class="sticky z-10 shrink-0 truncate px-2 text-right text-xs text-gray-500 {rowBgClass} {rowHoverClass}"
		style="right: {DATE_COLUMN_WIDTH}px; width: {AUTHOR_COLUMN_WIDTH}px;"
	>
		{commit.author.name}
	</span>

	<!-- Sticky date column (frozen right) -->
	<span
		class="sticky right-0 z-10 shrink-0 truncate px-2 text-right text-xs text-gray-600 {rowBgClass} {rowHoverClass}"
		style="width: {DATE_COLUMN_WIDTH}px;"
	>
		{formatGitDateTime(commit.commit_time)}
	</span>
</button>
