<script lang="ts">
	import { getVersion } from '@tauri-apps/api/app';
	import { commands as commandsStore } from '$lib/stores/commands';
	import { t } from '$lib/stores/locale';
	import { logPath } from '$lib/stores/debug';
	import { openLogDirectory } from '$lib/bindings/commands';

	interface Props {
		onclose: () => void;
	}

	let { onclose }: Props = $props();

	let appVersion = $state('…');

	$effect(() => {
		getVersion()
			.then((v) => (appVersion = v))
			.catch(() => (appVersion = '?'));
	});

	let commands = $derived($commandsStore.filter((c) => c.shortcut));
	let shortcutCategories = $derived(
		[...new Set(commands.map((c) => c.category ?? ''))].filter(Boolean).sort()
	);

	let closeBtn: HTMLButtonElement | undefined = $state();
	let x = $state(Math.max(0, Math.round((window.innerWidth - 420) / 2)));
	let y = $state(Math.max(0, Math.round((window.innerHeight - 400) / 2)));
	let isDragging = $state(false);
	let dragOffsetX = $state(0);
	let dragOffsetY = $state(0);

	$effect(() => {
		if (closeBtn) closeBtn.focus();
	});

	function onHeaderMouseDown(e: MouseEvent) {
		isDragging = true;
		dragOffsetX = e.clientX - x;
		dragOffsetY = e.clientY - y;
	}

	function onMouseMove(e: MouseEvent) {
		if (!isDragging) return;
		x = e.clientX - dragOffsetX;
		y = e.clientY - dragOffsetY;
	}

	function onMouseUp() {
		isDragging = false;
	}
</script>

<svelte:window
	onkeydown={(e) => {
		if (e.key === 'Escape') {
			e.preventDefault();
			onclose();
		}
	}}
	onmousemove={onMouseMove}
	onmouseup={onMouseUp}
/>

<div
	class="fixed z-50"
	style="left: {x}px; top: {y}px;"
	role="dialog"
	aria-label={$t('info.title')}
	tabindex="-1"
>
	<div
		class="flex flex-col rounded-lg border border-gray-700 bg-gray-900 shadow-2xl overflow-hidden"
		style="min-width: 320px; min-height: 200px; max-width: min(90vw, 500px); max-height: min(90vh, 600px); width: 420px; {isDragging
			? 'cursor: grabbing;'
			: ''}"
	>
		<div
			class="flex items-center justify-between border-b border-gray-800 px-4 py-2 cursor-grab select-none"
			onmousedown={onHeaderMouseDown}
			role="toolbar"
			aria-label={$t('info.title')}
			tabindex="-1"
		>
			<h2 class="text-sm font-semibold text-gray-100">{$t('info.title')}</h2>
			<button
				bind:this={closeBtn}
				class="rounded p-1 text-gray-500 hover:bg-gray-800 hover:text-white transition-colors"
				onclick={onclose}
				aria-label={$t('info.close_aria')}
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

		<div class="overflow-y-auto flex-1 px-4 py-3 space-y-4">
			<!-- App Info -->
			<section>
				<div class="flex items-center gap-2">
					<span class="text-base font-bold text-gray-100">gitv</span>
					<span class="text-xs text-gray-400">{$t('info.version')} {appVersion}</span>
				</div>
				<div class="mt-1 space-y-0.5 text-xs text-gray-400">
					<p>{$t('info.license')}: MIT</p>
				</div>
			</section>

			<div class="border-t border-gray-800"></div>

			<!-- Keyboard Shortcuts -->
			<section>
				<h3 class="font-semibold uppercase tracking-wider text-gray-500 mb-2">
					{$t('preferences.section_shortcuts')}
				</h3>
				{#if commands.length === 0}
					<p class="text-sm text-gray-500 italic">{$t('preferences.no_shortcuts')}</p>
				{:else}
					{#each shortcutCategories as cat (cat)}
						<div class="mb-2">
							<h4 class="text-xs text-gray-500 font-semibold mb-1">
								{$t('shortcut_categories.' + cat)}
							</h4>
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
			</section>

			<div class="border-t border-gray-800"></div>

			<!-- Logging -->
			<section>
				<h3 class="font-semibold uppercase tracking-wider text-gray-500 mb-2">
					{$t('preferences.section_logging')}
				</h3>
				{#if $logPath}
					<div class="space-y-2">
						<div class="flex items-center gap-2">
							<span class="text-sm text-gray-400 shrink-0">{$t('preferences.log_path')}</span>
							<span class="truncate text-sm text-gray-300" title={$logPath}>
								{$logPath}
							</span>
						</div>
						<button
							class="rounded bg-gray-700 px-2.5 py-1 text-xs text-gray-200 hover:bg-gray-600 transition-colors"
							onclick={() => openLogDirectory().catch(() => {})}
						>
							{$t('preferences.open_log_dir')}
						</button>
					</div>
				{:else}
					<p class="text-sm text-gray-500 italic">{$t('preferences.log_unavailable')}</p>
				{/if}
			</section>
		</div>
	</div>
</div>
