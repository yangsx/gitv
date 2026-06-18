<script lang="ts">
	import { t } from '$lib/stores/locale';
	import type { CommitInfo, Ref, Highlight, MatchType } from '$lib/bindings/types';
	import { STAGED_OID, UNSTAGED_OID } from '$lib/constants';
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
		rowHeight = 28
	}: Props = $props();

	function escapeHtml(s: string): string {
		return s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
	}

	function renderSummary(): string {
		const summary = commit.summary;
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
</script>

{#if isVirtual}
	<button
		{id}
		tabindex="-1"
		role="option"
		aria-selected={isSelected}
		style="height: {rowHeight}px;"
		class="flex w-full items-center gap-3 px-3 text-left text-sm hover:bg-gray-700 focus:outline-none {isSelected
			? 'bg-blue-900/40 text-blue-200'
			: isComparison
				? 'bg-indigo-900/40 text-indigo-200'
				: 'text-gray-300'}"
		aria-label={$t('commit_row.staged_aria', {
			summary: commit.summary,
			type: isStaged ? 'staged' : 'unstaged'
		})}
		onclick={(e: Event & { ctrlKey?: boolean; metaKey?: boolean }) =>
			onclick(commit.oid, !!(e.ctrlKey || e.metaKey))}
	>
		<span class="w-[80px] shrink-0 flex items-center gap-1">
			<span
				class="inline-block h-2 w-2 rounded-full {isStaged ? 'bg-green-400' : 'bg-orange-400'}"
				aria-hidden="true"
			></span>
		</span>
		<span class="min-w-0 truncate font-medium {isStaged ? 'text-green-300' : 'text-orange-300'}">
			<!-- eslint-disable-next-line svelte/no-at-html-tags -->
			{@html renderSummary()}
		</span>
	</button>
{:else}
	<button
		{id}
		tabindex="-1"
		role="option"
		aria-selected={isSelected}
		style="height: {rowHeight}px;"
		class="flex w-full items-center gap-3 px-3 text-left text-sm hover:bg-gray-700 focus:outline-none {isSelected
			? 'bg-blue-900/40 text-blue-200'
			: isComparison
				? 'bg-indigo-900/40 text-indigo-200'
				: isDimmed
					? 'text-gray-600'
					: 'text-gray-300'}"
		aria-label={$t('commit_row.aria_label', {
			summary: commit.summary,
			author: commit.author.name,
			date: formatGitDateTime(commit.commit_time)
		}) +
			(commit.refs.length > 0 ? ', ' + commit.refs.map(refLabel).filter(Boolean).join(', ') : '')}
		onclick={(e: Event & { ctrlKey?: boolean; metaKey?: boolean }) =>
			onclick(commit.oid, !!(e.ctrlKey || e.metaKey))}
		oncontextmenu={(e: MouseEvent) => oncontextmenu?.(e, commit.oid)}
	>
		<span class="w-[80px] shrink-0 font-mono text-xs text-gray-500">
			{commit.short_oid}
		</span>
		<span class="flex shrink-0 gap-1 overflow-hidden">
			{#each commit.refs as ref (ref.Branch ? 'b:' + ref.Branch.name : ref.Tag ? 't:' + ref.Tag.name : ref.Remote ? 'r:' + ref.Remote.remote + '/' + ref.Remote.name : '')}
				{@const label = refLabel(ref)}
				{#if label}
					<span
						class="inline-block rounded px-1 text-xs {ref.Branch?.is_head
							? 'bg-green-700/50 text-green-300'
							: ref.Tag
								? 'bg-yellow-700/50 text-yellow-300'
								: 'bg-gray-600/50 text-gray-400'}"
					>
						{label}
					</span>
				{/if}
			{/each}
		</span>
		<span class="min-w-0 truncate">
			<!-- eslint-disable-next-line svelte/no-at-html-tags -->
			{@html renderSummary()}
		</span>
		{#if matchType === 'Patch'}
			<span
				class="shrink-0 text-yellow-400 text-xs"
				title={$t('commit_row.patch_match')}
				aria-label={$t('commit_row.patch_match')}
			>
				⌗
			</span>
		{/if}
		<span class="ml-auto shrink-0 text-xs text-gray-500">{commit.author.name}</span>
		<span class="shrink-0 text-xs text-gray-600">{formatGitDateTime(commit.commit_time)}</span>
	</button>
{/if}
