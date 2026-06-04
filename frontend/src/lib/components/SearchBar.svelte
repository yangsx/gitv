<script lang="ts">
	import { searchCommits } from '$lib/bindings/commands';
	import { searchQuery, searchResults, operationState } from '$lib/stores/repository';
	import { showToast } from '$lib/stores/toast';

	interface Props {
		repoPath: string;
	}

	let { repoPath }: Props = $props();

	let inputText = $state('');
	let showOptions = $state(false);
	let useRegex = $state(false);
	let authorFilter = $state('');
	let debounceTimer: ReturnType<typeof setTimeout> | null = null;

	function handleInput() {
		if (debounceTimer) clearTimeout(debounceTimer);
		debounceTimer = setTimeout(() => executeSearch(), 200);
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Enter') {
			if (debounceTimer) clearTimeout(debounceTimer);
			executeSearch();
		}
		if (e.key === 'Escape') {
			inputText = '';
			authorFilter = '';
			searchQuery.set(null);
			searchResults.set([]);
		}
	}

	async function executeSearch() {
		if (!inputText && !authorFilter) {
			searchQuery.set(null);
			searchResults.set([]);
			return;
		}

		const query = {
			text: inputText || undefined,
			use_regex: useRegex,
			author: authorFilter || undefined,
			combine_mode: 'And' as const,
			sha_prefix: undefined,
			date_range: undefined
		};

		searchQuery.set(query);

		if (!repoPath) return;
		operationState.set('Searching');
		try {
			const results = await searchCommits(repoPath, query);
			searchResults.set(results);
			showToast(`Search: ${results.length} match${results.length !== 1 ? 'es' : ''}`, 'info');
		} catch {
			searchResults.set([]);
			showToast('Search failed', 'error');
		} finally {
			operationState.set('Idle');
		}
	}

	function clearSearch() {
		inputText = '';
		authorFilter = '';
		searchQuery.set(null);
		searchResults.set([]);
	}
</script>

<div class="flex items-center gap-2" role="search" aria-label="Search commits">
	<div class="relative flex-1">
		<input
			type="text"
			class="w-full rounded border border-gray-700 bg-gray-800 px-3 py-1.5 text-sm text-gray-200 placeholder-gray-500 focus:border-blue-500 focus:outline-none"
			placeholder="Search commits... (Esc to clear)"
			aria-label="Search commits"
			bind:value={inputText}
			oninput={handleInput}
			onkeydown={handleKeydown}
		/>
		{#if inputText || authorFilter}
			<button
				class="absolute right-2 top-1/2 -translate-y-1/2 text-gray-500 hover:text-gray-300"
				onclick={clearSearch}
				aria-label="Clear search"
			>
				✕
			</button>
		{/if}
	</div>

	<button
		class="rounded px-2 py-1.5 text-xs {showOptions
			? 'bg-gray-600 text-gray-200'
			: 'bg-gray-800 text-gray-400 hover:text-gray-200'} border border-gray-700"
		onclick={() => (showOptions = !showOptions)}
		aria-label="Search options"
		aria-expanded={showOptions}
	>
		Opts
	</button>

	{#if $searchQuery}
		<span class="shrink-0 text-xs text-gray-500" role="status" aria-live="polite">
			{$searchResults.length} match{$searchResults.length !== 1 ? 'es' : ''}
		</span>
	{/if}
</div>

{#if showOptions}
	<div class="mt-1 flex items-center gap-3" role="group" aria-label="Search options">
		<label class="flex items-center gap-1 text-xs text-gray-400">
			<input type="checkbox" bind:checked={useRegex} onchange={executeSearch} />
			Regex
		</label>
		<input
			type="text"
			class="rounded border border-gray-700 bg-gray-800 px-2 py-1 text-xs text-gray-300 placeholder-gray-500"
			placeholder="Author filter"
			aria-label="Author filter"
			bind:value={authorFilter}
			oninput={handleInput}
		/>
	</div>
{/if}
