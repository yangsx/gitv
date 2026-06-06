<script lang="ts">
	import { t } from '$lib/stores/locale';
	import type { CommitInfo, Ref } from '$lib/bindings/types';

	interface Props {
		commit: CommitInfo;
		isSelected: boolean;
		isDimmed?: boolean;
		onclick: (_oid: string, _ctrlKey: boolean) => void;
		oncontextmenu?: (_e: MouseEvent, _oid: string) => void;
		id?: string;
	}

	let { commit, isSelected, isDimmed = false, onclick, oncontextmenu, id }: Props = $props();

	const STAGED_OID = '__staged__';
	const UNSTAGED_OID = '__unstaged__';
	// svelte-ignore state_referenced_locally
	const isVirtual = commit.oid === STAGED_OID || commit.oid === UNSTAGED_OID;
	// svelte-ignore state_referenced_locally
	const isStaged = commit.oid === STAGED_OID;

	function formatTime(iso: string): string {
		try {
			return new Date(iso).toLocaleDateString(undefined, {
				month: 'short',
				day: 'numeric',
				year: 'numeric'
			});
		} catch {
			return '';
		}
	}

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
		role="option"
		aria-selected={isSelected}
		class="flex w-full items-center gap-3 px-3 h-7 text-left text-sm hover:bg-gray-700 focus:outline-none {isSelected
			? 'bg-blue-900/40 text-blue-200'
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
			{commit.summary}
		</span>
	</button>
{:else}
	<button
		{id}
		role="option"
		aria-selected={isSelected}
		class="flex w-full items-center gap-3 px-3 h-7 text-left text-sm hover:bg-gray-700 focus:outline-none {isSelected
			? 'bg-blue-900/40 text-blue-200'
			: isDimmed
				? 'text-gray-600'
				: 'text-gray-300'}"
		aria-label={$t('commit_row.aria_label', {
			summary: commit.summary,
			author: commit.author.name,
			date: formatTime(commit.commit_time)
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
			{#each commit.refs as ref (ref.Branch?.name ?? ref.Tag?.name ?? ref.Remote?.name ?? '')}
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
		<span class="min-w-0 truncate">{commit.summary}</span>
		<span class="ml-auto shrink-0 text-xs text-gray-500">{commit.author.name}</span>
		<span class="shrink-0 text-xs text-gray-600">{formatTime(commit.commit_time)}</span>
	</button>
{/if}
