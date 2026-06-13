<script lang="ts">
	import { searchCommits, cancelPatchSearch } from '$lib/bindings/commands';
	import type {
		PatchSearchProgress,
		PatchSearchComplete,
		PatchSearchError
	} from '$lib/bindings/types';
	import { listen } from '@tauri-apps/api/event';
	import {
		searchQuery,
		searchResults,
		operationState,
		sortBy,
		sortAsc,
		searchShowMode,
		patchSearchActive,
		patchSearchId,
		patchSearchProgress
	} from '$lib/stores/repository';
	import { showToast, updateToast, dismissToast } from '$lib/stores/toast';
	import { t, translate } from '$lib/stores/locale';

	interface Props {
		repoPath: string;
	}

	let { repoPath }: Props = $props();

	let inputText = $state('');
	let showOptions = $state(false);
	let useRegex = $state(false);
	let searchPatch = $state(false);
	let authorFilter = $state('');
	let shaPrefix = $state('');
	let dateFrom = $state('');
	let dateTo = $state('');
	let filePath = $state('');
	let debounceTimer: ReturnType<typeof setTimeout> | null = null;
	let containerRef: HTMLDivElement | undefined = $state();
	let searchToastId: number | null = null;
	let unlistenPatch: (() => void) | null = null;
	let expectedPatchSearchId: number | null = null;

	function handleInput() {
		if (debounceTimer) clearTimeout(debounceTimer);
		debounceTimer = setTimeout(() => executeSearch(), 200);
	}

	async function cancelActivePatchSearch() {
		if ($patchSearchId !== null) {
			await cancelPatchSearch($patchSearchId);
		}
		patchSearchActive.set(false);
		patchSearchId.set(null);
		patchSearchProgress.set(null);
		expectedPatchSearchId = null;
	}

	function teardownPatchListener() {
		if (unlistenPatch) {
			unlistenPatch();
			unlistenPatch = null;
		}
	}

	async function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Enter') {
			if (debounceTimer) clearTimeout(debounceTimer);
			await executeSearch();
		}
		if (e.key === 'Escape') {
			e.stopPropagation();
			if (showOptions) {
				showOptions = false;
				return;
			}
			inputText = '';
			authorFilter = '';
			searchQuery.set(null);
			searchResults.set([]);
			if (searchToastId !== null) {
				dismissToast(searchToastId);
				searchToastId = null;
			}
			await cancelActivePatchSearch();
			teardownPatchListener();
		}
	}

	async function executeSearch() {
		if (!inputText && !authorFilter && !shaPrefix && !dateFrom && !dateTo && !filePath) {
			searchQuery.set(null);
			searchResults.set([]);
			if (searchToastId !== null) {
				dismissToast(searchToastId);
				searchToastId = null;
			}
			await cancelActivePatchSearch();
			teardownPatchListener();
			return;
		}

		const date_range =
			dateFrom || dateTo ? { from: dateFrom || undefined, to: dateTo || undefined } : undefined;

		const query = {
			text: inputText || undefined,
			use_regex: useRegex,
			search_patch: searchPatch,
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
			await cancelActivePatchSearch();
			teardownPatchListener();

			if (searchPatch) {
				unlistenPatch = await listen<PatchSearchProgress | PatchSearchComplete | PatchSearchError>(
					'patch-search-progress',
					(event) => {
						const payload = event.payload;
						if (expectedPatchSearchId !== null && payload.search_id !== expectedPatchSearchId) {
							return;
						}
						if ('matches' in payload) {
							searchResults.update((r) => [...r, ...payload.matches]);
							patchSearchProgress.set({
								checked: payload.checked,
								total: payload.total
							});
						} else if ('total_checked' in payload) {
							patchSearchActive.set(false);
							patchSearchId.set(null);
							patchSearchProgress.set(null);
							expectedPatchSearchId = null;
						} else if ('message' in payload) {
							patchSearchActive.set(false);
							patchSearchId.set(null);
							patchSearchProgress.set(null);
							expectedPatchSearchId = null;
							showToast(payload.message, 'error');
						}
					}
				);
			}

			const response = await searchCommits(repoPath, query);
			searchResults.set(response.results);

			const totalCount = response.results.length;
			const msg = translate(totalCount === 1 ? 'search.matches' : 'search.matches_plural', {
				count: totalCount
			});
			if (searchToastId !== null) {
				updateToast(searchToastId, msg, 'info');
			} else {
				searchToastId = showToast(msg, 'info');
			}

			if (response.patch_search_id !== null) {
				expectedPatchSearchId = response.patch_search_id;
				patchSearchId.set(response.patch_search_id);
				patchSearchActive.set(true);
				patchSearchProgress.set({
					checked: 0,
					total: response.patch_search_total ?? 0
				});
			} else if (unlistenPatch) {
				unlistenPatch();
				unlistenPatch = null;
			}
		} catch {
			searchResults.set([]);
			showToast(translate('page.search_failed'), 'error');
			teardownPatchListener();
		} finally {
			operationState.set('Idle');
		}
	}

	async function clearSearch() {
		inputText = '';
		authorFilter = '';
		shaPrefix = '';
		dateFrom = '';
		dateTo = '';
		filePath = '';
		searchQuery.set(null);
		searchResults.set([]);
		if (searchToastId !== null) {
			dismissToast(searchToastId);
			searchToastId = null;
		}
		await cancelActivePatchSearch();
		teardownPatchListener();
	}

	async function handleCancelPatch() {
		await cancelActivePatchSearch();
		teardownPatchListener();
	}

	function handleClickOutside(e: MouseEvent) {
		if (containerRef && !containerRef.contains(e.target as Node)) {
			showOptions = false;
		}
	}

	$effect(() => {
		return () => {
			if (debounceTimer) clearTimeout(debounceTimer);
			teardownPatchListener();
			if ($patchSearchId !== null) {
				cancelPatchSearch($patchSearchId).catch(() => {});
			}
		};
	});
</script>

<svelte:window onclick={handleClickOutside} />

<!-- svelte-ignore binding_property_non_reactive -->
<div bind:this={containerRef} class="relative" role="search" aria-label={$t('search.search_aria')}>
	<div class="flex items-center gap-2">
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

		{#if $patchSearchActive && $patchSearchProgress}
			<span class="shrink-0 text-xs text-cyan-400">
				{$t('search.patch_progress', $patchSearchProgress)}
			</span>
			<button
				class="shrink-0 rounded px-2 py-1 text-xs border border-gray-700 bg-red-800/50 text-red-300 hover:bg-red-700/50"
				onclick={handleCancelPatch}
				aria-label={$t('search.cancel')}
			>
				{$t('search.cancel')}
			</button>
		{/if}
	</div>

	{#if showOptions}
		<div
			class="absolute right-0 top-full mt-1 z-50 flex items-center gap-3 rounded-md border border-gray-700 bg-gray-900/95 px-3 py-2 shadow-lg backdrop-blur-sm"
			role="group"
			aria-label={$t('search.options_aria')}
		>
			<label class="flex items-center gap-1 text-xs text-gray-400">
				<input type="checkbox" bind:checked={useRegex} onchange={executeSearch} />
				{$t('search.regex')}
			</label>
			<label class="flex items-center gap-1 text-xs text-gray-400">
				<input type="checkbox" bind:checked={searchPatch} onchange={executeSearch} />
				{$t('search.patch')}
			</label>
			<input
				type="text"
				class="w-24 rounded border border-gray-700 bg-gray-800 px-2 py-1 text-xs text-gray-300 placeholder-gray-500"
				placeholder={$t('search.author_filter')}
				aria-label={$t('search.author_filter')}
				bind:value={authorFilter}
				oninput={handleInput}
			/>
			<input
				type="text"
				class="w-24 rounded border border-gray-700 bg-gray-800 px-2 py-1 text-xs text-gray-300 placeholder-gray-500"
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
				class="w-24 rounded border border-gray-700 bg-gray-800 px-2 py-1 text-xs text-gray-300 placeholder-gray-500"
				placeholder={$t('search.file_path')}
				aria-label={$t('search.file_path')}
				bind:value={filePath}
				oninput={handleInput}
			/>
			<span class="text-xs text-gray-500">{$t('search.sort_by')}</span>
			<select
				class="rounded border border-gray-700 px-1.5 py-1 text-xs text-gray-300"
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
</div>
