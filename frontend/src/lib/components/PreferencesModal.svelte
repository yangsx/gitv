<script lang="ts">
	import {
		graphColorMode,
		graphHideMerges,
		graphOrientation,
		graphPalette
	} from '$lib/stores/repository';
	import {
		diffMode,
		diffWhitespace,
		showStashes,
		renderer,
		theme,
		fontSize,
		highContrast,
		savePreferences
	} from '$lib/stores/preferences';
	import { t, locale, setLocale as setAppLocale } from '$lib/stores/locale';
	import type { Locale } from '$lib/stores/locale';
	import { getCommands } from '$lib/stores/commands';
	import { logPath } from '$lib/stores/debug';
	import { openLogDirectory } from '$lib/bindings/commands';

	interface Props {
		onclose: () => void;
	}

	let { onclose }: Props = $props();

	let dialogEl: HTMLDivElement | undefined = $state();
	let closeBtn: HTMLButtonElement | undefined = $state();
	let x = $state(Math.max(0, Math.round((window.innerWidth - 384) / 2)));
	let y = $state(Math.max(0, Math.round((window.innerHeight - 400) / 2)));
	let isDragging = $state(false);
	let dragOffsetX = $state(0);
	let dragOffsetY = $state(0);

	$effect(() => {
		if (closeBtn) closeBtn.focus();
	});

	function getFocusableElements(): HTMLElement[] {
		if (!dialogEl) return [];
		return Array.from(
			dialogEl.querySelectorAll<HTMLElement>(
				'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])'
			)
		);
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape') {
			e.preventDefault();
			onclose();
			return;
		}
		if (e.key === 'Tab') {
			const focusable = getFocusableElements();
			if (focusable.length === 0) return;
			const first = focusable[0];
			const last = focusable[focusable.length - 1];
			if (e.shiftKey) {
				if (document.activeElement === first) {
					e.preventDefault();
					last.focus();
				}
			} else {
				if (document.activeElement === last) {
					e.preventDefault();
					first.focus();
				}
			}
		}
	}

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

	function setColorMode(mode: 'by-branch' | 'by-author') {
		graphColorMode.set(mode);
		savePreferences();
	}

	function toggleHideMerges() {
		graphHideMerges.update((v) => !v);
		savePreferences();
	}

	function toggleShowStashes() {
		showStashes.update((v) => !v);
		savePreferences();
	}

	function setRenderer(r: 'wgpu' | 'canvas2d') {
		renderer.set(r);
		savePreferences();
	}

	function setOrientation(orient: 'top-to-bottom' | 'bottom-to-top') {
		graphOrientation.set(orient);
		savePreferences();
	}

	function setPalette(palette: 'default' | 'deuteranopia' | 'protanopia' | 'tritanopia') {
		graphPalette.set(palette);
		savePreferences();
	}

	function setDiffMode(mode: 'normal' | 'word-diff' | 'stat-only') {
		diffMode.set(mode);
		savePreferences();
	}

	function setDiffWhitespace(
		ws: 'none' | 'ignore-space-change' | 'ignore-all-space' | 'ignore-blank-lines'
	) {
		diffWhitespace.set(ws);
		savePreferences();
	}

	function setLanguage(lang: Locale) {
		setAppLocale(lang);
		savePreferences();
	}

	const colorModes = ['by-branch', 'by-author'] as const;
	const orientations = ['top-to-bottom', 'bottom-to-top'] as const;
	const palettes = ['default', 'deuteranopia', 'protanopia', 'tritanopia'] as const;
	const diffModes = ['normal', 'word-diff', 'stat-only'] as const;
	const modeKey = (v: 'normal' | 'word-diff' | 'stat-only'): string =>
		v === 'word-diff' ? 'word_diff' : v === 'stat-only' ? 'stat_only' : v;
	const whitespaceModes = [
		'none',
		'ignore-space-change',
		'ignore-all-space',
		'ignore-blank-lines'
	] as const;
	const languages: Locale[] = ['en', 'zh-cn'];
	const themes = ['dark', 'light'] as const;

	function setTheme(t: 'dark' | 'light') {
		theme.set(t);
		savePreferences();
	}

	function setFontSize(e: Event) {
		const target = e.target as HTMLInputElement;
		const val = parseInt(target.value, 10);
		fontSize.set(val);
		savePreferences();
	}

	let commands = $derived(getCommands().filter((c) => c.shortcut));
	let shortcutCategories = $derived(
		[...new Set(commands.map((c) => c.category ?? ''))].filter(Boolean).sort()
	);
</script>

<svelte:window onkeydown={handleKeydown} onmousemove={onMouseMove} onmouseup={onMouseUp} />

<!-- svelte-ignore a11y_click_events_have_key_events -->
<div
	bind:this={dialogEl}
	class="fixed z-50"
	style="left: {x}px; top: {y}px;"
	role="dialog"
	aria-label={$t('preferences.title')}
	tabindex="-1"
>
	<div
		class="flex flex-col rounded-lg border border-gray-700 bg-gray-900 shadow-2xl overflow-hidden resize"
		style="min-width: 320px; min-height: 250px; max-width: min(90vw, 700px); max-height: min(90vh, 700px); width: 384px; {isDragging
			? 'cursor: grabbing;'
			: ''}"
		onresize={() => {}}
	>
		<div
			class="flex items-center justify-between border-b border-gray-800 px-3 py-2 cursor-grab select-none"
			onmousedown={onHeaderMouseDown}
			role="toolbar"
			aria-label={$t('preferences.drag_aria')}
			tabindex="-1"
		>
			<h2 class="text-xs font-semibold text-gray-100">{$t('preferences.title')}</h2>
			<button
				bind:this={closeBtn}
				class="rounded p-1 text-gray-500 hover:bg-gray-800 hover:text-white transition-colors"
				onclick={onclose}
				aria-label={$t('preferences.close_aria')}
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

		<div class="overflow-y-auto flex-1 px-3 py-2 space-y-4 text-xs">
			<!-- Graph Section -->
			<section>
				<h3 class="font-semibold uppercase tracking-wider text-gray-500 mb-2">
					{$t('preferences.section_graph')}
				</h3>
				<div class="space-y-2">
					<div class="flex items-center justify-between">
						<span class="text-gray-300">{$t('preferences.color_mode')}</span>
						<div
							class="flex gap-1"
							role="radiogroup"
							aria-label={$t('preferences.color_mode_aria')}
						>
							{#each colorModes as v (v)}
								<button
									class="whitespace-nowrap rounded px-2 py-1 text-xs transition-colors {$graphColorMode ===
									v
										? 'bg-blue-700/50 text-blue-300'
										: 'text-gray-400 hover:bg-gray-800 hover:text-white'}"
									onclick={() => setColorMode(v)}
									role="radio"
									aria-checked={$graphColorMode === v}
								>
									{$t(v === 'by-branch' ? 'preferences.by_branch' : 'preferences.by_author')}
								</button>
							{/each}
						</div>
					</div>

					<div class="flex items-center justify-between">
						<span class="text-gray-300">{$t('preferences.orientation')}</span>
						<div
							class="flex gap-1"
							role="radiogroup"
							aria-label={$t('preferences.orientation_aria')}
						>
							{#each orientations as v (v)}
								<button
									class="whitespace-nowrap rounded px-2 py-1 text-xs transition-colors {$graphOrientation ===
									v
										? 'bg-blue-700/50 text-blue-300'
										: 'text-gray-400 hover:bg-gray-800 hover:text-white'}"
									onclick={() => setOrientation(v)}
									role="radio"
									aria-checked={$graphOrientation === v}
								>
									{$t(
										v === 'top-to-bottom'
											? 'preferences.top_to_bottom'
											: 'preferences.bottom_to_top'
									)}
								</button>
							{/each}
						</div>
					</div>

					<div class="flex items-center justify-between">
						<span class="text-gray-300">{$t('preferences.palette')}</span>
						<div class="flex gap-1" role="radiogroup" aria-label={$t('preferences.palette_aria')}>
							{#each palettes as v (v)}
								<button
									class="whitespace-nowrap rounded px-2 py-1 text-xs transition-colors {$graphPalette ===
									v
										? 'bg-blue-700/50 text-blue-300'
										: 'text-gray-400 hover:bg-gray-800 hover:text-white'}"
									onclick={() => setPalette(v)}
									role="radio"
									aria-checked={$graphPalette === v}
								>
									{$t(`preferences.palette_${v}`)}
								</button>
							{/each}
						</div>
					</div>

					<div class="flex items-center justify-between">
						<span class="text-gray-300">{$t('preferences.hide_merges')}</span>
						<button
							class="relative h-5 w-9 rounded-full transition-colors {$graphHideMerges
								? 'bg-blue-600'
								: 'bg-gray-700'}"
							onclick={toggleHideMerges}
							role="switch"
							aria-checked={$graphHideMerges}
							aria-label={$t('preferences.hide_merges_aria')}
						>
							<span
								class="absolute top-0.5 left-0.5 h-4 w-4 rounded-full bg-white transition-transform {$graphHideMerges
									? 'translate-x-4'
									: 'translate-x-0'}"
							></span>
						</button>
					</div>

					<div class="flex items-center justify-between">
						<span class="text-gray-300">{$t('preferences.show_stashes')}</span>
						<button
							class="relative h-5 w-9 rounded-full transition-colors {$showStashes
								? 'bg-blue-600'
								: 'bg-gray-700'}"
							onclick={toggleShowStashes}
							role="switch"
							aria-checked={$showStashes}
							aria-label={$t('preferences.show_stashes')}
						>
							<span
								class="absolute top-0.5 left-0.5 h-4 w-4 rounded-full bg-white transition-transform {$showStashes
									? 'translate-x-4'
									: 'translate-x-0'}"
							></span>
						</button>
					</div>

					<div class="flex items-center justify-between">
						<span class="text-gray-300">{$t('preferences.renderer')}</span>
						<div class="flex gap-1" role="radiogroup" aria-label={$t('preferences.renderer_aria')}>
							<button
								class="whitespace-nowrap rounded px-2 py-1 text-xs transition-colors {$renderer ===
								'wgpu'
									? 'bg-blue-700/50 text-blue-300'
									: 'text-gray-400 hover:bg-gray-800 hover:text-white'}"
								onclick={() => setRenderer('wgpu')}
								role="radio"
								aria-checked={$renderer === 'wgpu'}
							>
								{$t('preferences.renderer_wgpu')}
							</button>
							<button
								class="whitespace-nowrap rounded px-2 py-1 text-xs transition-colors {$renderer ===
								'canvas2d'
									? 'bg-blue-700/50 text-blue-300'
									: 'text-gray-400 hover:bg-gray-800 hover:text-white'}"
								onclick={() => setRenderer('canvas2d')}
								role="radio"
								aria-checked={$renderer === 'canvas2d'}
							>
								{$t('preferences.renderer_canvas2d')}
							</button>
						</div>
					</div>
				</div>
			</section>

			<div class="border-t border-gray-800"></div>

			<!-- Diff Section -->
			<section>
				<h3 class="font-semibold uppercase tracking-wider text-gray-500 mb-2">
					{$t('preferences.section_diff')}
				</h3>
				<div class="space-y-2">
					<div class="flex items-center justify-between">
						<span class="text-gray-300">{$t('preferences.default_mode')}</span>
						<div class="flex gap-1" role="radiogroup" aria-label={$t('preferences.diff_mode_aria')}>
							{#each diffModes as v (v)}
								<button
									class="whitespace-nowrap rounded px-2 py-1 text-xs transition-colors {$diffMode ===
									v
										? 'bg-blue-700/50 text-blue-300'
										: 'text-gray-400 hover:bg-gray-800 hover:text-white'}"
									onclick={() => setDiffMode(v)}
									role="radio"
									aria-checked={$diffMode === v}
								>
									{$t(`preferences.mode_${modeKey(v)}`)}
								</button>
							{/each}
						</div>
					</div>

					<div class="flex items-center justify-between">
						<span class="text-gray-300">{$t('preferences.whitespace')}</span>
						<div
							class="flex gap-1"
							role="radiogroup"
							aria-label={$t('preferences.whitespace_aria')}
						>
							{#each whitespaceModes as v (v)}
								<button
									class="whitespace-nowrap rounded px-2 py-1 text-xs transition-colors {$diffWhitespace ===
									v
										? 'bg-blue-700/50 text-blue-300'
										: 'text-gray-400 hover:bg-gray-800 hover:text-white'}"
									onclick={() => setDiffWhitespace(v)}
									role="radio"
									aria-checked={$diffWhitespace === v}
								>
									{$t(
										`preferences.ws_${v === 'none' ? 'show' : v === 'ignore-space-change' ? 'space' : v === 'ignore-all-space' ? 'all' : 'blanks'}`
									)}
								</button>
							{/each}
						</div>
					</div>
				</div>
			</section>

			<div class="border-t border-gray-800"></div>

			<!-- Language Section -->
			<section>
				<h3 class="font-semibold uppercase tracking-wider text-gray-500 mb-2">
					{$t('preferences.language')}
				</h3>
				<div class="flex items-center justify-between">
					<span class="text-gray-300">{$t('preferences.language')}</span>
					<div class="flex gap-1" role="radiogroup" aria-label={$t('preferences.language_aria')}>
						{#each languages as lang (lang)}
							<button
								class="whitespace-nowrap rounded px-2 py-1 text-xs transition-colors {$locale ===
								lang
									? 'bg-blue-700/50 text-blue-300'
									: 'text-gray-400 hover:bg-gray-800 hover:text-white'}"
								onclick={() => setLanguage(lang)}
								role="radio"
								aria-checked={$locale === lang}
							>
								{$t(`preferences.lang_${lang === 'en' ? 'en' : 'zh_cn'}`)}
							</button>
						{/each}
					</div>
				</div>
			</section>

			<div class="border-t border-gray-800"></div>

			<!-- Appearance Section -->
			<section>
				<h3 class="font-semibold uppercase tracking-wider text-gray-500 mb-2">
					{$t('preferences.section_appearance')}
				</h3>
				<div class="space-y-3">
					<div class="flex items-center justify-between">
						<span class="text-gray-300">{$t('preferences.theme')}</span>
						<div class="flex gap-1" role="radiogroup" aria-label={$t('preferences.theme_aria')}>
							{#each themes as v (v)}
								<button
									class="whitespace-nowrap rounded px-2 py-1 text-xs transition-colors {$theme === v
										? 'bg-blue-700/50 text-blue-300'
										: 'text-gray-400 hover:bg-gray-800 hover:text-white'}"
									onclick={() => setTheme(v)}
									role="radio"
									aria-checked={$theme === v}
								>
									{$t(`preferences.theme_${v}`)}
								</button>
							{/each}
						</div>
					</div>
					<div class="flex items-center justify-between">
						<span class="text-gray-300">{$t('preferences.font_size')}</span>
						<div class="flex items-center gap-2">
							<span class="text-gray-500 text-xs">10</span>
							<input
								type="range"
								min="10"
								max="24"
								step="1"
								value={$fontSize}
								oninput={setFontSize}
								class="w-24 h-1 appearance-none bg-gray-700 rounded-full cursor-pointer accent-blue-500"
								aria-label={$t('preferences.font_size_aria')}
							/>
							<span class="text-gray-500 text-xs">24</span>
							<span class="w-6 text-right text-gray-300 text-xs">{$fontSize}</span>
						</div>
					</div>
					<div class="flex items-center justify-between">
						<span class="text-gray-300">{$t('preferences.high_contrast')}</span>
						<button
							class="relative h-5 w-9 rounded-full transition-colors {$highContrast
								? 'bg-blue-600'
								: 'bg-gray-700'}"
							onclick={() => {
								highContrast.update((v) => !v);
								savePreferences();
							}}
							role="switch"
							aria-checked={$highContrast}
							aria-label={$t('preferences.high_contrast_aria')}
						>
							<span
								class="absolute top-0.5 left-0.5 h-4 w-4 rounded-full bg-white transition-transform {$highContrast
									? 'translate-x-4'
									: 'translate-x-0'}"
							></span>
						</button>
					</div>
				</div>
			</section>

			<div class="border-t border-gray-800"></div>

			<!-- Keyboard Shortcuts Section -->
			<section>
				<h3 class="font-semibold uppercase tracking-wider text-gray-500 mb-2">
					{$t('preferences.section_shortcuts')}
				</h3>
				{#if commands.length === 0}
					<p class="text-gray-500 italic">{$t('preferences.no_shortcuts')}</p>
				{:else}
					{#each shortcutCategories as cat (cat)}
						<div class="mb-2">
							<h4 class="text-xs text-gray-500 font-semibold mb-1">{cat}</h4>
							<div class="space-y-1">
								{#each commands.filter((c) => c.category === cat) as cmd (cmd.id)}
									<div class="flex items-center justify-between">
										<span class="text-gray-300 text-xs">{cmd.label}</span>
										<kbd class="rounded bg-gray-800 px-1.5 py-0.5 font-mono text-xs text-gray-400">
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

			<!-- Logging Section -->
			<section>
				<h3 class="font-semibold uppercase tracking-wider text-gray-500 mb-2">
					{$t('preferences.section_logging')}
				</h3>
				{#if $logPath}
					<div class="space-y-2">
						<div class="flex items-center gap-2">
							<span class="text-gray-400 shrink-0">{$t('preferences.log_path')}</span>
							<span class="truncate text-gray-300 text-[11px]" title={$logPath}>
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
					<p class="text-gray-500 italic">{$t('preferences.log_unavailable')}</p>
				{/if}
			</section>
		</div>
	</div>
</div>

<style>
	:global(.resize)::-webkit-resizer {
		background: #4b5563;
		border-radius: 0 0 0.5rem 0;
	}
</style>
