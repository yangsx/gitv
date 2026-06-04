<script lang="ts">
	import { searchCommits } from '$lib/bindings/commands';
	import { searchQuery, searchResults, isLoading } from '$lib/stores/repository';

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
		isLoading.set(true);
		try {
			const results = await searchCommits(repoPath, query);
			searchResults.set(results);
		} catch {
			searchResults.set([]);
		} finally {
			isLoading.set(false);
		}
	}

	function clearSearch() {
		inputText = '';
		authorFilter = '';
		searchQuery.set(null);
		searchResults.set([]);
	}
</script>

<div class="flex items-center gap-2">
	<div class="relative flex-1">
		<input
			type="text"
			class="w-full rounded border border-gray-700 bg-gray-800 px-3 py-1.5 text-sm text-gray-200 placeholder-gray-500 focus:border-blue-500 focus:outline-none"
			placeholder="Search commits... (Esc to clear)"
			bind:value={inputText}
			oninput={handleInput}
			onkeydown={handleKeydown}
		/>
		{#if inputText || authorFilter}
			<button
				class="absolute right-2 top-1/2 -translate-y-1/2 text-gray-500 hover:text-gray-300"
				onclick={clearSearch}
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
	>
		Opts
	</button>

	{#if $searchQuery}
		<span class="shrink-0 text-xs text-gray-500">
			{$searchResults.length} match{$searchResults.length !== 1 ? 'es' : ''}
		</span>
	{/if}
</div>

{#if showOptions}
	<div class="mt-1 flex items-center gap-3">
		<label class="flex items-center gap-1 text-xs text-gray-400">
			<input type="checkbox" bind:checked={useRegex} onchange={executeSearch} />
			Regex
		</label>
		<input
			type="text"
			class="rounded border border-gray-700 bg-gray-800 px-2 py-1 text-xs text-gray-300 placeholder-gray-500"
			placeholder="Author filter"
			bind:value={authorFilter}
			oninput={handleInput}
		/>
	</div>
{/if}
