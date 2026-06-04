<script lang="ts">
	import type { Snippet } from 'svelte';

	type Tab = 'refs' | 'stash' | 'reflog';

	let {
		refs,
		stash,
		reflog
	}: {
		refs?: Snippet;
		stash?: Snippet;
		reflog?: Snippet;
	} = $props();

	let activeTab = $state<Tab>('refs');
	let collapsed = $state(false);
</script>

{#if collapsed}
	<div class="flex flex-col items-center gap-2 border-r border-gray-800 bg-gray-900 py-2 px-1">
		<button
			class="rounded p-1 text-gray-400 hover:bg-gray-800 hover:text-white"
			onclick={() => (collapsed = false)}
			title="Expand sidebar"
		>
			<svg class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
				<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7" />
			</svg>
		</button>
	</div>
{:else}
	<div
		class="flex flex-col border-r border-gray-800 bg-gray-900"
		style="width: 220px; min-width: 180px;"
	>
		<div class="flex items-center justify-between border-b border-gray-800 px-2 py-1">
			<div class="flex gap-1">
				{#each ['refs', 'stash', 'reflog'] as tab (tab)}
					<button
						class="rounded px-2 py-0.5 text-xs {activeTab === tab
							? 'bg-gray-700 text-white'
							: 'text-gray-400 hover:text-white'}"
						onclick={() => (activeTab = tab as Tab)}
					>
						{tab === 'refs' ? 'Refs' : tab === 'stash' ? 'Stash' : 'Reflog'}
					</button>
				{/each}
			</div>
			<button
				class="rounded p-0.5 text-gray-400 hover:bg-gray-800 hover:text-white"
				onclick={() => (collapsed = true)}
				title="Collapse sidebar"
			>
				<svg class="h-3.5 w-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
					<path
						stroke-linecap="round"
						stroke-linejoin="round"
						stroke-width="2"
						d="M15 19l-7-7 7-7"
					/>
				</svg>
			</button>
		</div>

		<div class="flex-1 overflow-y-auto p-2 text-xs">
			{#if activeTab === 'refs' && refs}
				{@render refs()}
			{:else if activeTab === 'stash' && stash}
				{@render stash()}
			{:else if activeTab === 'reflog' && reflog}
				{@render reflog()}
			{/if}
		</div>
	</div>
{/if}
