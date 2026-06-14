<script lang="ts">
	import { t, translate } from '$lib/stores/locale';
	import type { StashEntry } from '$lib/bindings/types';
	import { getStashList } from '$lib/bindings/commands';
	import { formatGitDateTime } from '$lib/utils/format-date';
	import { showToast } from '$lib/stores/toast';

	let {
		repoPath,
		refreshSignal,
		onstashselect
	}: {
		repoPath: string;
		refreshSignal: number;
		onstashselect?: (_stash: StashEntry) => void;
	} = $props();

	let stashes = $state<StashEntry[]>([]);
	let loading = $state(true);

	$effect(() => {
		void repoPath;
		void refreshSignal;
		loadStashes();
	});

	async function loadStashes() {
		loading = true;
		try {
			stashes = await getStashList(repoPath);
		} catch {
			stashes = [];
			showToast(translate('sidebar.stash_load_failed'), 'error');
		} finally {
			loading = false;
		}
	}
</script>

{#if loading}
	<div class="text-gray-500">{$t('sidebar.loading_stashes')}</div>
{:else if stashes.length === 0}
	<div class="text-gray-500 italic">{$t('sidebar.no_stashes')}</div>
{:else}
	<div class="space-y-1">
		{#each stashes as stash (stash.index)}
			<button
				class="w-full rounded px-2 py-1.5 text-left hover:bg-gray-800"
				aria-label={$t('sidebar.stash_aria', { n: stash.index, message: stash.message })}
				onclick={() => onstashselect?.(stash)}
			>
				<div class="flex items-center gap-1">
					<span class="text-yellow-400" aria-hidden="true">
						<svg class="h-3 w-3" fill="currentColor" viewBox="0 0 20 20" aria-hidden="true">
							<path d="M17.293 13.293A8 8 0 016.707 2.707a8.001 8.001 0 1010.586 10.586z" />
						</svg>
					</span>
					<span class="truncate text-gray-200">{'stash@' + stash.index}</span>
					<span class="ml-auto text-gray-500">{formatGitDateTime(stash.time)}</span>
				</div>
				<div class="mt-0.5 truncate text-gray-400">{stash.message}</div>
				{#if stash.file_summary.length > 0}
					<div class="mt-0.5 text-gray-500">
						{$t(
							stash.file_summary.length === 1 ? 'sidebar.file_count' : 'sidebar.file_count_plural',
							{ count: stash.file_summary.length }
						)}
					</div>
				{/if}
			</button>
		{/each}
	</div>
{/if}
