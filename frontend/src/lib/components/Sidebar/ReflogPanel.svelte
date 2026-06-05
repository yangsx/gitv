<script lang="ts">
	import type { ReflogEntry } from '$lib/bindings/types';
	import { getReflog } from '$lib/bindings/commands';
	import { t } from '$lib/stores/locale';

	let {
		repoPath,
		onentryselect
	}: {
		repoPath: string;
		onentryselect?: (_oid: string) => void;
	} = $props();

	let entries = $state<ReflogEntry[]>([]);
	let loading = $state(true);
	let filterOp = $state('');

	$effect(() => {
		void repoPath;
		loadReflog();
	});

	async function loadReflog() {
		loading = true;
		try {
			entries = await getReflog(repoPath);
		} catch {
			entries = [];
		} finally {
			loading = false;
		}
	}

	function formatTime(timeStr: string): string {
		try {
			const d = new Date(timeStr);
			return (
				d.toLocaleDateString() +
				' ' +
				d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })
			);
		} catch {
			return '';
		}
	}

	function stripReflogPrefix(msg: string): string {
		return msg.replace(
			/^(commit(?: \([^)]+\))?|checkout|pull|merge|rebase|reset|revert|cherry-pick|bisect|am|replace|rebase -i(?: \([^)]+\))?):\s*/i,
			''
		);
	}

	let filteredEntries = $derived(
		filterOp
			? entries.filter((e) => e.message.toLowerCase().includes(filterOp.toLowerCase()))
			: entries
	);
</script>

<div class="space-y-2">
	<div class="flex items-center gap-1">
		<span class="rounded border border-gray-700 bg-gray-800 px-1.5 py-0.5 text-xs text-gray-300">
			{$t('sidebar.reflog_select')}
		</span>
		<input
			type="text"
			class="flex-1 rounded border border-gray-700 bg-gray-800 px-1.5 py-0.5 text-xs text-gray-300 placeholder-gray-500"
			placeholder={$t('sidebar.reflog_filter')}
			aria-label={$t('sidebar.reflog_filter')}
			bind:value={filterOp}
		/>
	</div>

	{#if loading}
		<div class="text-gray-500">{$t('sidebar.loading_reflog')}</div>
	{:else if filteredEntries.length === 0}
		<div class="text-gray-500 italic">{$t('sidebar.no_reflog')}</div>
	{:else}
		<div class="space-y-0.5">
			{#each filteredEntries.slice(0, 100) as entry, i (entry.oid + '-' + i)}
				<button
					class="w-full rounded px-1.5 py-1 text-left hover:bg-gray-800"
					aria-label={$t('sidebar.reflog_aria', {
						message: stripReflogPrefix(entry.message).slice(0, 50),
						oid: entry.oid.slice(0, 7),
						author: entry.author.name
					})}
					onclick={() => onentryselect?.(entry.oid)}
				>
					<div class="flex items-center gap-1">
						<span class="font-mono text-gray-400">{entry.oid.slice(0, 7)}</span>
						<span class="truncate text-gray-300"
							>{stripReflogPrefix(entry.message).slice(0, 50)}</span
						>
					</div>
					<div class="mt-0.5 text-gray-500">{entry.author.name} · {formatTime(entry.time)}</div>
				</button>
			{/each}
		</div>
	{/if}
</div>
