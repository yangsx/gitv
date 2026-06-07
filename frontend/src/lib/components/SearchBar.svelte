<script lang="ts">
	import { searchCommits } from '$lib/bindings/commands';
	import {
		searchQuery,
		searchResults,
		operationState,
		sortBy,
		sortAsc,
		searchShowMode
	} from '$lib/stores/repository';
	import { showToast } from '$lib/stores/toast';
	import { t, translate } from '$lib/stores/locale';

	interface Props {
		repoPath: string;
	}

	let { repoPath }: Props = $props();

	let inputText = $state('');
	let showOptions = $state(false);
	let useRegex = $state(false);
	let authorFilter = $state('');
	let shaPrefix = $state('');
	let dateFrom = $state('');
	let dateTo = $state('');
	let filePath = $state('');
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
		if (!inputText && !authorFilter && !shaPrefix && !dateFrom && !dateTo && !filePath) {
			searchQuery.set(null);
			searchResults.set([]);
			return;
		}

		const date_range =
			dateFrom || dateTo ? { from: dateFrom || undefined, to: dateTo || undefined } : undefined;

		const query = {
			text: inputText || undefined,
			use_regex: useRegex,
			author: authorFilter || undefined,
			combine_mode: 'And' as const,
			sha_prefix: shaPrefix || undefined,
			date_range,
			file_path: filePath || undefined
		};

		searchQuery.set(query);

		if (!repoPath) return;
		operationState.set('Searching');
		try {
			const results = await searchCommits(repoPath, query);
			searchResults.set(results);
			showToast(
				translate(results.length === 1 ? 'search.matches' : 'search.matches_plural', {
					count: results.length
				}),
				'info'
			);
		} catch {
			searchResults.set([]);
			showToast(translate('page.search_failed'), 'error');
		} finally {
			operationState.set('Idle');
		}
	}

	function clearSearch() {
		inputText = '';
		authorFilter = '';
		shaPrefix = '';
		dateFrom = '';
		dateTo = '';
		filePath = '';
		searchQuery.set(null);
		searchResults.set([]);
	}
</script>

<div class="flex items-center gap-2" role="search" aria-label={$t('search.search_aria')}>
	<div class="relative flex-1">
		<input
			type="text"
			class="w-full rounded border border-gray-700 bg-gray-800 px-3 py-1.5 text-sm text-gray-200 placeholder-gray-500 focus:border-blue-500 focus:outline-none"
			placeholder={$t('search.placeholder')}
			aria-label={$t('search.search_aria')}
			bind:value={inputText}
			oninput={handleInput}
			onkeydown={handleKeydown}
		/>
		{#if inputText || authorFilter}
			<button
				class="absolute right-2 top-1/2 -translate-y-1/2 text-gray-500 hover:text-gray-300"
				onclick={clearSearch}
				aria-label={$t('search.clear')}
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
		aria-label={$t('search.options_aria')}
		aria-expanded={showOptions}
	>
		{$t('search.opts')}
	</button>

	{#if $searchQuery}
		<span class="shrink-0 text-xs text-gray-500" role="status" aria-live="polite">
			{$t($searchResults.length === 1 ? 'search.matches' : 'search.matches_plural', {
				count: $searchResults.length
			})}
		</span>
	{/if}
</div>

{#if showOptions}
	<div class="mt-1 flex items-center gap-3" role="group" aria-label={$t('search.options_aria')}>
		<label class="flex items-center gap-1 text-xs text-gray-400">
			<input type="checkbox" bind:checked={useRegex} onchange={executeSearch} />
			{$t('search.regex')}
		</label>
		<input
			type="text"
			class="rounded border border-gray-700 bg-gray-800 px-2 py-1 text-xs text-gray-300 placeholder-gray-500"
			placeholder={$t('search.author_filter')}
			aria-label={$t('search.author_filter')}
			bind:value={authorFilter}
			oninput={handleInput}
		/>
		<input
			type="text"
			class="w-28 rounded border border-gray-700 bg-gray-800 px-2 py-1 text-xs text-gray-300 placeholder-gray-500"
			placeholder={$t('search.sha_prefix')}
			aria-label={$t('search.sha_prefix')}
			bind:value={shaPrefix}
			oninput={handleInput}
		/>
		<input
			type="date"
			class="rounded border border-gray-700 bg-gray-800 px-2 py-1 text-xs text-gray-300"
			aria-label={$t('search.date_from')}
			bind:value={dateFrom}
			oninput={handleInput}
		/>
		<input
			type="date"
			class="rounded border border-gray-700 bg-gray-800 px-2 py-1 text-xs text-gray-300"
			aria-label={$t('search.date_to')}
			bind:value={dateTo}
			oninput={handleInput}
		/>
		<input
			type="text"
			class="rounded border border-gray-700 bg-gray-800 px-2 py-1 text-xs text-gray-300 placeholder-gray-500"
			placeholder={$t('search.file_path')}
			aria-label={$t('search.file_path')}
			bind:value={filePath}
			oninput={handleInput}
		/>
		<span class="text-xs text-gray-500">{$t('search.sort_by')}</span>
		<select
			class="rounded border border-gray-700 bg-gray-800 px-1.5 py-1 text-xs text-gray-300"
			aria-label={$t('search.sort_by')}
			value={$sortBy}
			onchange={(e) =>
				sortBy.set((e.target as HTMLSelectElement).value as 'date' | 'author' | 'sha')}
		>
			<option value="date">{$t('search.sort_date')}</option>
			<option value="author">{$t('search.sort_author')}</option>
			<option value="sha">{$t('search.sort_sha')}</option>
		</select>
		<button
			class="rounded px-1.5 py-1 text-xs border border-gray-700 {$sortAsc
				? 'bg-gray-600 text-gray-200'
				: 'bg-gray-800 text-gray-400 hover:text-gray-200'}"
			onclick={() => sortAsc.update((v) => !v)}
			aria-label={$sortAsc ? $t('search.sort_desc') : $t('search.sort_asc')}
		>
			{$sortAsc ? $t('search.sort_asc') : $t('search.sort_desc')}
		</button>
		<button
			class="rounded px-2 py-1 text-xs border border-gray-700 {$searchShowMode === 'hide-nonhits'
				? 'bg-blue-600 text-white'
				: 'bg-gray-800 text-gray-400 hover:text-gray-200'}"
			onclick={() =>
				searchShowMode.update((v) => (v === 'hide-nonhits' ? 'insitu' : 'hide-nonhits'))}
			aria-label={$searchShowMode === 'hide-nonhits'
				? $t('search.show_nonmatches')
				: $t('search.hide_nonmatches')}
		>
			{$searchShowMode === 'hide-nonhits'
				? $t('search.show_nonmatches')
				: $t('search.hide_nonmatches')}
		</button>
	</div>
{/if}
