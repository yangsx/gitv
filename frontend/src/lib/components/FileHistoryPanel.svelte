<script lang="ts">
	import type { FileHistoryEntry } from '$lib/bindings/types';
	import { getFileHistory } from '$lib/bindings/commands';
	import { t, translate } from '$lib/stores/locale';
	import { formatGitDateTime } from '$lib/utils/format-date';
	import { showToast } from '$lib/stores/toast';

	function entrySummary(entry: FileHistoryEntry): string {
		return entry.message.split('\n')[0] || '';
	}

	interface Props {
		repoPath: string;
		filePath: string;
		revision?: number;
		onclose?: () => void;
		onenterselect?: (_oid: string) => void;
	}

	let { repoPath, filePath, revision, onclose, onenterselect }: Props = $props();

	let entries = $state<FileHistoryEntry[]>([]);
	let loading = $state(true);

	$effect(() => {
		void filePath;
		void revision;
		loadHistory();
	});

	async function loadHistory() {
		loading = true;
		try {
			entries = await getFileHistory(repoPath, filePath);
		} catch {
			entries = [];
			showToast(translate('file_history.load_failed'), 'error');
		} finally {
			loading = false;
		}
	}
</script>

<div class="flex h-full flex-col bg-gray-900">
	<div class="flex items-center gap-2 border-b border-gray-700 px-4 py-2">
		<h3 class="truncate font-mono text-sm text-gray-300">
			{$t('file_history.title', { path: filePath })}
		</h3>
		{#if onclose}
			<button
				class="ml-auto shrink-0 rounded px-2 py-1 text-xs text-gray-400 hover:bg-gray-800 hover:text-gray-200"
				onclick={onclose}
			>
				{$t('file_history.close')}
			</button>
		{/if}
	</div>
	<div class="flex-1 overflow-y-auto">
		{#if loading}
			<div class="flex items-center justify-center py-8 text-sm text-gray-500">
				{$t('file_history.loading')}
			</div>
		{:else if entries.length === 0}
			<div class="flex items-center justify-center py-8 text-sm text-gray-500">
				{$t('file_history.none')}
			</div>
		{:else}
			{#each entries as entry (entry.commit_oid)}
				<button
					class="w-full border-b border-gray-800 px-3 py-1.5 text-left hover:bg-gray-800/50"
					aria-label={$t('file_history.entry_aria', {
						summary: entrySummary(entry),
						author: entry.author.name,
						oid: entry.commit_oid.slice(0, 8)
					})}
					onclick={() => onenterselect?.(entry.commit_oid)}
				>
					<div class="flex items-center gap-2">
						<span class="font-mono text-gray-500">{entry.commit_oid.slice(0, 8)}</span>
						<span class="flex-1 truncate text-gray-300">{entrySummary(entry)}</span>
						<span class="shrink-0 text-gray-500">{entry.author.name}</span>
						<span class="shrink-0 text-gray-600">{formatGitDateTime(entry.time)}</span>
					</div>
					{#if entry.old_path}
						<div class="mt-0.5 text-gray-500">
							{$t('file_history.renamed_from', { path: entry.old_path })}
						</div>
					{/if}
				</button>
			{/each}
		{/if}
	</div>
</div>
