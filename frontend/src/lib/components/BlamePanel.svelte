<script lang="ts">
	import { t } from '$lib/stores/locale';
	import type { Blame } from '$lib/bindings/types';
	import { getBlame } from '$lib/bindings/commands';
	import { formatDate } from '$lib/utils/format-date';
	import { createGenerationGuard } from '$lib/utils/async-guard';

	const loadGen = createGenerationGuard();

	let {
		repoPath,
		filePath,
		atCommit,
		oncommitclick,
		onclose
	}: {
		repoPath: string;
		filePath: string;
		atCommit?: string;
		oncommitclick?: (_oid: string) => void;
		onclose?: () => void;
	} = $props();

	let blame = $state<Blame | null>(null);
	let loading = $state(true);
	let error = $state('');

	$effect(() => {
		loadBlame();
	});

	async function loadBlame() {
		const gen = loadGen.next();
		loading = true;
		error = '';
		try {
			const result = await getBlame(repoPath, filePath, atCommit);
			if (!loadGen.isStale(gen)) blame = result;
		} catch (e: unknown) {
			if (!loadGen.isStale(gen)) error = e instanceof Error ? e.message : String(e);
		} finally {
			if (!loadGen.isStale(gen)) loading = false;
		}
	}

	let currentCommit = $state<string | null>(null);

	let blameGroups = $derived(
		(() => {
			if (!blame) return [];
			const groups: {
				oid: string;
				author: string;
				date: string;
				lines: { num: number; content: string }[];
			}[] = [];
			let current: (typeof groups)[0] | null = null;
			for (const line of blame.lines) {
				if (!current || current.oid !== line.commit_oid) {
					current = {
						oid: line.commit_oid,
						author: line.author.name,
						date: formatDate(line.time),
						lines: []
					};
					groups.push(current);
				}
				current.lines.push({ num: line.line_number, content: line.content });
			}
			return groups;
		})()
	);
</script>

<div class="absolute inset-0 z-50 flex flex-col bg-gray-900">
	<div class="flex items-center justify-between border-b border-gray-700 px-3 py-1.5">
		<div class="flex items-center gap-2">
			<span class="text-xs font-medium text-gray-300">{$t('blame.title')}</span>
			<span class="font-mono text-xs text-gray-400">{filePath}</span>
		</div>
		<button
			class="rounded p-1 text-gray-400 hover:bg-gray-800 hover:text-white"
			onclick={onclose}
			aria-label={$t('blame.close')}
		>
			<svg class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
				<path
					stroke-linecap="round"
					stroke-linejoin="round"
					stroke-width="2"
					d="M6 18L18 6M6 6l12 12"
				/>
			</svg>
		</button>
	</div>

	{#if loading}
		<div class="flex items-center justify-center py-8 text-sm text-gray-500">
			{$t('blame.loading')}
		</div>
	{:else if error}
		<div class="p-4 text-sm text-red-400">{error}</div>
	{:else if blame}
		<div class="flex-1 overflow-auto">
			<table
				class="w-full border-collapse text-xs"
				aria-label={$t('blame.aria', { file: filePath })}
			>
				<thead class="sr-only">
					<tr>
						<th scope="col">{$t('blame.line_number')}</th>
						<th scope="col">{$t('blame.author_date')}</th>
						<th scope="col">{$t('blame.content')}</th>
					</tr>
				</thead>
				<tbody>
					{#each blameGroups as group (group.oid + '-' + group.lines[0]?.num)}
						{#each group.lines as line, i (line.num)}
							<tr
								class={currentCommit === group.oid
									? 'bg-blue-900/20'
									: 'border-b border-gray-800/50 hover:bg-gray-800/50'}
							>
								<td class="w-10 border-r border-gray-800 px-1 py-0.5 text-right text-gray-500"
									>#{line.num}</td
								>
								<td
									class="w-48 cursor-pointer border-r border-gray-800 px-2 py-0.5"
									role="button"
									tabindex="0"
									aria-label="Toggle highlight for commit {group.oid.slice(0, 7)} by {group.author}"
									onclick={() => {
										currentCommit = currentCommit === group.oid ? null : group.oid;
									}}
									onkeydown={(e: KeyboardEvent) => {
										if (e.key === 'Enter' || e.key === ' ') {
											e.preventDefault();
											currentCommit = currentCommit === group.oid ? null : group.oid;
										}
									}}
									ondblclick={() => oncommitclick?.(group.oid)}
								>
									{#if i === 0}
										<div class="truncate text-gray-400" title="{group.author} · {group.date}">
											<span class="text-gray-300">{group.author}</span>
											<span class="ml-1 text-gray-500">· {group.date}</span>
										</div>
									{:else}
										<div class="text-gray-600">...</div>
									{/if}
								</td>
								<td class="px-2 py-0.5">
									<pre class="whitespace-pre-wrap font-mono text-gray-300">{line.content}</pre>
								</td>
							</tr>
						{/each}
					{/each}
				</tbody>
			</table>
		</div>
	{/if}
</div>
