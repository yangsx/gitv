<script lang="ts">
	import type { Snippet } from 'svelte';
	import { t } from '$lib/stores/locale';

	type Tab = 'refs' | 'stash' | 'reflog' | 'history';

	let {
		refs,
		stash,
		reflog,
		history,
		gotoTab,
		width = 220
	}: {
		refs?: Snippet;
		stash?: Snippet;
		reflog?: Snippet;
		history?: Snippet;
		gotoTab?: Tab;
		width?: number;
	} = $props();

	let activeTab = $state<Tab>('refs');
	let collapsed = $state(false);

	const tabs = $derived(
		[
			{ id: 'refs' as Tab, label: $t('sidebar.refs'), snippet: refs },
			{ id: 'stash' as Tab, label: $t('sidebar.stash'), snippet: stash },
			{ id: 'reflog' as Tab, label: $t('sidebar.reflog'), snippet: reflog },
			...(history ? [{ id: 'history' as Tab, label: $t('sidebar.history'), snippet: history }] : [])
		].filter((t): t is { id: Tab; label: string; snippet: Snippet } => t.snippet !== undefined)
	);

	$effect(() => {
		if (gotoTab && tabs.some((t) => t.id === gotoTab)) {
			activeTab = gotoTab;
		}
	});

	$effect(() => {
		if (activeTab === 'history' && !history) {
			activeTab = 'refs';
		}
	});
</script>

{#if collapsed}
	<div
		class="flex flex-col items-center gap-2 border-r border-gray-800 bg-gray-900 py-2 px-1"
		role="navigation"
		aria-label={$t('sidebar.aria')}
	>
		<button
			class="rounded p-1 text-gray-400 hover:bg-gray-800 hover:text-white"
			onclick={() => (collapsed = false)}
			title={$t('sidebar.expand')}
			aria-label={$t('sidebar.expand')}
		>
			<svg class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor" aria-hidden="true">
				<path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7" />
			</svg>
		</button>
	</div>
{:else}
	<div
		class="flex flex-col border-r border-gray-800 bg-gray-900"
		style="width: {width}px; min-width: 150px;"
		role="navigation"
		aria-label={$t('sidebar.aria')}
	>
		<div class="flex items-center justify-between border-b border-gray-800 px-2 py-1">
			<div class="flex gap-1" role="tablist" aria-label={$t('sidebar.tabs_aria')}>
				{#each tabs as tab (tab.id)}
					<button
						role="tab"
						aria-selected={activeTab === tab.id}
						aria-controls="sidebar-panel-{tab.id}"
						class="whitespace-nowrap rounded px-2 py-0.5 text-xs {activeTab === tab.id
							? 'bg-gray-700 text-white'
							: 'text-gray-400 hover:text-white'}"
						onclick={() => (activeTab = tab.id)}
					>
						{tab.label}
					</button>
				{/each}
			</div>
			<button
				class="rounded p-0.5 text-gray-400 hover:bg-gray-800 hover:text-white"
				onclick={() => (collapsed = true)}
				title={$t('sidebar.collapse')}
				aria-label={$t('sidebar.collapse')}
			>
				<svg
					class="h-3.5 w-3.5"
					fill="none"
					viewBox="0 0 24 24"
					stroke="currentColor"
					aria-hidden="true"
				>
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
			{#each tabs as tab (tab.id)}
				{#if activeTab === tab.id}
					<div id="sidebar-panel-{tab.id}" role="tabpanel" aria-label="{tab.label} panel">
						{@render tab.snippet()}
					</div>
				{/if}
			{/each}
		</div>
	</div>
{/if}
