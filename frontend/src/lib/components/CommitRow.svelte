<script lang="ts">
	import type { CommitInfo, Ref } from '$lib/bindings/types';

	interface Props {
		commit: CommitInfo;
		isSelected: boolean;
		onclick: (_oid: string) => void;
	}

	let { commit, isSelected, onclick }: Props = $props();

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

<button
	class="flex w-full items-center gap-3 px-3 py-1 text-left text-sm hover:bg-gray-700 {isSelected
		? 'bg-blue-900/40 text-blue-200'
		: 'text-gray-300'}"
	onclick={() => onclick(commit.oid)}
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
