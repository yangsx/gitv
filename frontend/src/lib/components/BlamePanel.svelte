<script lang="ts">
	import { t } from '$lib/stores/locale';
	import type { Blame } from '$lib/bindings/types';
	import { getBlame } from '$lib/bindings/commands';
	import { formatGitDateTime } from '$lib/utils/format-date';
	import { createGenerationGuard } from '$lib/utils/async-guard';
	import { highlightLines, type HighlightToken } from '$lib/utils/highlight';
	import { draggable } from '$lib/actions/draggable';
	import { dialogStackOffset } from '$lib/stores/dialog';

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
	let blameTokens = $state<HighlightToken[][] | null>(null);

	const { offset: stackOffset, unregister } = dialogStackOffset();
	let dialogWidth = $state(700);
	let dialogHeight = $state(450);
	// svelte-ignore state_referenced_locally
	let x = $state(Math.max(0, Math.round((window.innerWidth - dialogWidth) / 2)) + stackOffset);
	// svelte-ignore state_referenced_locally
	let y = $state(Math.max(0, Math.round((window.innerHeight - dialogHeight) / 2)) + stackOffset);

	function handleDragMove(newX: number, newY: number) {
		x = newX;
		y = newY;
	}

	function handleResizeStart(e: MouseEvent) {
		e.preventDefault();
		e.stopPropagation();
		const startX = e.clientX;
		const startY = e.clientY;
		const startWidth = dialogWidth;
		const startHeight = dialogHeight;

		function onMouseMove(e: MouseEvent) {
			const maxW = Math.min(Math.round(window.innerWidth * 0.9), 1000);
			const maxH = Math.min(Math.round(window.innerHeight * 0.9), 800);
			dialogWidth = Math.max(400, Math.min(maxW, startWidth + (e.clientX - startX)));
			dialogHeight = Math.max(200, Math.min(maxH, startHeight + (e.clientY - startY)));
		}

		function onMouseUp() {
			window.removeEventListener('mousemove', onMouseMove);
			window.removeEventListener('mouseup', onMouseUp);
		}

		window.addEventListener('mousemove', onMouseMove);
		window.addEventListener('mouseup', onMouseUp);
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape') {
			e.preventDefault();
			onclose?.();
		}
	}

	$effect(() => {
		return () => unregister();
	});

	$effect(() => {
		loadBlame();
	});

	async function loadBlame() {
		const gen = loadGen.next();
		loading = true;
		error = '';
		blameTokens = null;
		try {
			const result = await getBlame(repoPath, filePath, atCommit);
			if (!loadGen.isStale(gen)) {
				blame = result;
				if (result) {
					const content = result.lines.map((l) => l.content).join('\n');
					highlightLines(content, filePath).then((tokens) => {
						if (!loadGen.isStale(gen)) blameTokens = tokens;
					});
				}
			}
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
						date: formatGitDateTime(line.time),
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

<svelte:window onkeydown={handleKeydown} />

<div
	class="fixed z-50"
	style="left: {x}px; top: {y}px;"
	role="dialog"
	aria-label={$t('blame.title')}
	tabindex="-1"
>
	<div
		class="flex flex-col rounded-lg border border-gray-700 bg-gray-900 shadow-2xl overflow-hidden relative"
		style:width="{dialogWidth}px"
		style:height="{dialogHeight}px"
	>
		<div
			class="flex items-center justify-between border-b border-gray-800 px-3 py-2 cursor-grab select-none shrink-0"
			use:draggable={{ onMove: handleDragMove }}
		>
			<div class="flex items-center gap-2 min-w-0">
				<span class="text-xs font-medium text-gray-300 shrink-0">{$t('blame.title')}</span>
				<span class="font-mono text-xs text-gray-400 truncate">{filePath}</span>
			</div>
			<button
				class="rounded p-1 text-gray-400 hover:bg-gray-800 hover:text-white shrink-0"
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
								<tr class={currentCommit === group.oid ? 'bg-blue-900/20' : 'hover:bg-gray-800/50'}>
									<td class="w-10 px-1 py-0.5 text-right text-gray-600 select-none">#{line.num}</td>
									<td
										class="w-48 cursor-pointer px-2 py-0.5"
										role="button"
										tabindex="0"
										aria-label={$t('blame.toggle_highlight', { sha: group.oid.slice(0, 7) })}
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
										{#if blameTokens && blameTokens[line.num - 1]}
											<pre
												class="whitespace-pre-wrap text-gray-300"
												style="font-family: monospace !important">{#each blameTokens[line.num - 1] as token (token)}<span
														style={token.color ? `color: ${token.color}` : undefined}
														>{token.content}</span
													>{/each}</pre>
										{:else}
											<pre
												class="whitespace-pre-wrap text-gray-300"
												style="font-family: monospace !important">{line.content}</pre>
										{/if}
									</td>
								</tr>
							{/each}
						{/each}
					</tbody>
				</table>
			</div>
		{/if}

		<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
		<div
			class="absolute bottom-0 right-0 z-10 w-4 h-4 cursor-nwse-resize"
			role="separator"
			aria-orientation="vertical"
			aria-label="resize"
			onmousedown={handleResizeStart}
		>
			<svg viewBox="0 0 16 16" class="w-full h-full text-gray-600">
				<path fill="currentColor" d="M16 16L8 16L16 8Z" />
			</svg>
		</div>
	</div>
</div>
