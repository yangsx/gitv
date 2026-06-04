<script lang="ts">
	import { searchCommands, type Command } from '$lib/stores/commands';

	interface Props {
		onclose: () => void;
	}

	let { onclose }: Props = $props();

	let query = $state('');
	let selectedIndex = $state(0);
	let inputEl: HTMLInputElement | undefined = $state();

	let results = $derived(searchCommands(query));

	$effect(() => {
		void query;
		selectedIndex = 0;
	});

	$effect(() => {
		if (inputEl) inputEl.focus();
	});

	function execute(cmd: Command) {
		onclose();
		cmd.action();
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape') {
			e.preventDefault();
			onclose();
		} else if (e.key === 'ArrowDown') {
			e.preventDefault();
			selectedIndex = Math.min(selectedIndex + 1, results.length - 1);
		} else if (e.key === 'ArrowUp') {
			e.preventDefault();
			selectedIndex = Math.max(selectedIndex - 1, 0);
		} else if (e.key === 'Enter') {
			e.preventDefault();
			if (results[selectedIndex]) {
				execute(results[selectedIndex]);
			}
		}
	}

	function onBackdropClick(e: MouseEvent) {
		if (e.target === e.currentTarget) onclose();
	}
</script>

<svelte:window onkeydown={handleKeydown} />

<div
	class="fixed inset-0 z-50 flex items-start justify-center pt-[15vh]"
	style="background: rgba(0,0,0,0.5);"
	onclick={onBackdropClick}
	role="dialog"
	aria-label="Command palette"
>
	<div class="w-full max-w-lg rounded-lg border border-gray-700 bg-gray-900 shadow-2xl">
		<div class="flex items-center border-b border-gray-800 px-3">
			<svg
				class="h-4 w-4 text-gray-500 shrink-0"
				fill="none"
				viewBox="0 0 24 24"
				stroke="currentColor"
				aria-hidden="true"
			>
				<path
					stroke-linecap="round"
					stroke-linejoin="round"
					stroke-width="2"
					d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"
				/>
			</svg>
			<input
				bind:this={inputEl}
				bind:value={query}
				class="flex-1 bg-transparent px-2 py-3 text-sm text-gray-100 outline-none placeholder-gray-500"
				placeholder="Type a command..."
				aria-label="Search commands"
			/>
		</div>
		<ul class="max-h-64 overflow-y-auto py-1" role="listbox">
			{#each results as cmd, i (cmd.id)}
				<li>
					<button
						class="flex w-full items-center justify-between px-3 py-2 text-sm {i === selectedIndex
							? 'bg-blue-600/30 text-white'
							: 'text-gray-300 hover:bg-gray-800'}"
						onclick={() => execute(cmd)}
						role="option"
						aria-selected={i === selectedIndex}
					>
						<span>
							{#if cmd.category}
								<span class="text-gray-500 mr-1">{cmd.category}:</span>
							{/if}
							{cmd.label}
						</span>
						{#if cmd.shortcut}
							<kbd
								class="rounded border border-gray-700 bg-gray-800 px-1.5 py-0.5 text-[10px] text-gray-400 font-mono"
							>
								{cmd.shortcut}
							</kbd>
						{/if}
					</button>
				</li>
			{/each}
			{#if results.length === 0 && query.trim()}
				<li class="px-3 py-4 text-center text-sm text-gray-500">No matching commands</li>
			{/if}
		</ul>
	</div>
</div>
