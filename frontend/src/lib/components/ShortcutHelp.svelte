<script lang="ts">
	import { commands as commandsStore } from '$lib/stores/commands';
	import { t } from '$lib/stores/locale';

	interface Props {
		onclose: () => void;
	}

	let { onclose }: Props = $props();

	let commands = $derived($commandsStore.filter((c) => c.shortcut));
	let categories = $derived(
		[...new Set(commands.map((c) => c.category ?? ''))].filter(Boolean).sort()
	);

	let closeBtn: HTMLButtonElement | undefined = $state();

	$effect(() => {
		if (closeBtn) closeBtn.focus();
	});
</script>

<svelte:window
	onkeydown={(e) => {
		if (e.key === 'Escape') {
			e.preventDefault();
			onclose();
		}
	}}
/>

<!-- svelte-ignore a11y_click_events_have_key_events a11y_interactive_supports_focus -->
<div
	class="fixed inset-0 z-50 flex items-center justify-center bg-black/50"
	onclick={onclose}
	role="dialog"
	aria-label={$t('shortcut_help.title')}
	tabindex="-1"
>
	<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
	<div
		class="w-[420px] max-h-[70vh] rounded-lg border border-gray-700 bg-gray-900 shadow-2xl overflow-hidden"
		onclick={(e) => e.stopPropagation()}
		role="document"
		tabindex="-1"
	>
		<div class="flex items-center justify-between border-b border-gray-800 px-4 py-2">
			<h2 class="text-sm font-semibold text-gray-100">{$t('shortcut_help.title')}</h2>
			<button
				bind:this={closeBtn}
				class="rounded p-1 text-gray-500 hover:bg-gray-800 hover:text-white transition-colors"
				onclick={onclose}
				aria-label={$t('shortcut_help.close_aria')}
			>
				<svg
					class="h-3.5 w-3.5"
					fill="none"
					viewBox="0 0 24 24"
					stroke="currentColor"
					aria-hidden="true"
				>
					<path
						stroke-linecap="round"
						stroke-linejoin="round"
						stroke-width="2"
						d="M6 18L18 6M6 6l12 12"
					/>
				</svg>
			</button>
		</div>
		<div class="overflow-y-auto px-4 py-3 space-y-3 max-h-[60vh]">
			{#if commands.length === 0}
				<p class="text-sm text-gray-500 italic text-center py-4">{$t('shortcut_help.none')}</p>
			{:else}
				{#each categories as cat (cat)}
					<div>
						<h3 class="text-xs font-semibold uppercase tracking-wider text-gray-500 mb-1">
							{$t('shortcut_categories.' + cat)}
						</h3>
						<div class="space-y-1">
							{#each commands.filter((c) => c.category === cat) as cmd (cmd.id)}
								<div class="flex items-center justify-between py-1">
									<span class="text-sm text-gray-300">{cmd.label}</span>
									<kbd
										class="rounded bg-gray-800 px-2 py-0.5 font-mono text-xs text-gray-400 border border-gray-700"
									>
										{cmd.shortcut}
									</kbd>
								</div>
							{/each}
						</div>
					</div>
				{/each}
			{/if}
			<p class="text-xs text-gray-500 text-center pt-2 border-t border-gray-800">
				{$t('shortcut_help.dismiss_hint')}
			</p>
		</div>
	</div>
</div>
