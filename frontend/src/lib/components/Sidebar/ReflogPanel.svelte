<script lang="ts">
	import type { Ref, ReflogEntry } from '$lib/bindings/types';
	import { getReflog } from '$lib/bindings/commands';
	import { t } from '$lib/stores/locale';
	import { formatGitDateTime } from '$lib/utils/format-date';

	let {
		repoPath,
		refs,
		onentryselect
	}: {
		repoPath: string;
		refs?: Ref[];
		onentryselect?: (_oid: string) => void;
	} = $props();

	let entries = $state<ReflogEntry[]>([]);
	let loading = $state(true);
	let selectedRef = $state('HEAD');
	let selectedOpType = $state('all');
	let filterText = $state('');

	let refNames = $derived.by(() => {
		const names: string[] = ['HEAD'];
		if (refs) {
			for (const r of refs) {
				if (r.Branch) names.push(r.Branch.name);
			}
		}
		return names;
	});

	$effect(() => {
		void repoPath;
		void selectedRef;
		loadReflog();
	});

	async function loadReflog() {
		loading = true;
		selectedOpType = 'all';
		try {
			const refName = selectedRef === 'HEAD' ? undefined : selectedRef;
			entries = await getReflog(repoPath, refName);
		} catch {
			entries = [];
		} finally {
			loading = false;
		}
	}

	const OP_RE =
		/^(commit(?: \([^)]+\))?|checkout|pull|merge|rebase(?: -i(?: \([^)]+\))?)?|reset|revert|cherry-pick|bisect|am|replace):/i;

	function extractOpType(msg: string): string {
		const m = OP_RE.exec(msg);
		if (!m) return 'other';
		let op = m[1].toLowerCase();
		if (op.startsWith('rebase')) op = 'rebase';
		return op;
	}

	function stripReflogPrefix(msg: string): string {
		return msg.replace(OP_RE, '').trim();
	}

	let opTypes = $derived.by(() => {
		const seen: string[] = [];
		for (const e of entries) {
			const op = extractOpType(e.message);
			if (!seen.includes(op)) seen.push(op);
		}
		return seen.sort();
	});

	let filteredEntries = $derived(
		entries.filter((e) => {
			if (selectedOpType !== 'all' && extractOpType(e.message) !== selectedOpType) return false;
			if (filterText && !e.message.toLowerCase().includes(filterText.toLowerCase())) return false;
			return true;
		})
	);
</script>

<div class="space-y-2">
	<div class="flex items-center gap-1">
		<select
			class="flex-1 rounded border border-gray-700 px-1.5 py-0.5 text-xs text-gray-300"
			aria-label={$t('sidebar.select_ref')}
			bind:value={selectedRef}
		>
			{#each refNames as name (name)}
				<option value={name}>{name}</option>
			{/each}
		</select>
	</div>

	<div class="flex items-center gap-1">
		<select
			class="flex-1 rounded border border-gray-700 px-1.5 py-0.5 text-xs text-gray-300"
			aria-label={$t('sidebar.reflog_op_type')}
			bind:value={selectedOpType}
		>
			<option value="all">{$t('sidebar.reflog_op_all')}</option>
			{#each opTypes as op (op)}
				<option value={op}>{op}</option>
			{/each}
		</select>
		<input
			type="text"
			class="flex-1 rounded border border-gray-700 bg-gray-800 px-1.5 py-0.5 text-xs text-gray-300 placeholder-gray-500"
			placeholder={$t('sidebar.reflog_filter')}
			aria-label={$t('sidebar.reflog_filter')}
			bind:value={filterText}
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
						<span class="rounded bg-gray-700/50 px-1 text-[10px] text-gray-400 shrink-0">
							{extractOpType(entry.message)}
						</span>
						<span class="font-mono text-gray-400">{entry.oid.slice(0, 7)}</span>
						<span class="truncate text-gray-300"
							>{stripReflogPrefix(entry.message).slice(0, 50)}</span
						>
					</div>
					<div class="mt-0.5 text-gray-500">
						{entry.author.name} · {formatGitDateTime(entry.time)}
					</div>
				</button>
			{/each}
		</div>
	{/if}
</div>
