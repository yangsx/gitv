<script lang="ts">
	import {
		graphColorMode,
		graphHideMerges,
		graphOrientation,
		graphPalette
	} from '$lib/stores/repository';
	import { diffMode, diffWhitespace, savePreferences } from '$lib/stores/preferences';

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

	const colorModes = [
		{ value: 'by-branch' as const, label: 'By Branch' },
		{ value: 'by-author' as const, label: 'By Author' }
	];

	const orientations = [
		{ value: 'top-to-bottom' as const, label: 'Top to Bottom' },
		{ value: 'bottom-to-top' as const, label: 'Bottom to Top' }
	];

	const palettes = [
		{ value: 'default' as const, label: 'Default' },
		{ value: 'deuteranopia' as const, label: 'Deuteranopia' },
		{ value: 'protanopia' as const, label: 'Protanopia' },
		{ value: 'tritanopia' as const, label: 'Tritanopia' }
	];

	const diffModes = [
		{ value: 'normal' as const, label: 'Normal' },
		{ value: 'word-diff' as const, label: 'Word Diff' },
		{ value: 'stat-only' as const, label: 'Stats Only' }
	];

	const whitespaceModes = [
		{ value: 'none' as const, label: 'Show' },
		{ value: 'ignore-space-change' as const, label: 'Space' },
		{ value: 'ignore-all-space' as const, label: 'All' },
		{ value: 'ignore-blank-lines' as const, label: 'Blanks' }
	];
</script>

<svelte:window onkeydown={handleKeydown} onmousemove={onMouseMove} onmouseup={onMouseUp} />

<!-- svelte-ignore a11y_click_events_have_key_events -->
<div
	bind:this={dialogEl}
	class="fixed z-50"
	style="left: {x}px; top: {y}px;"
	role="dialog"
	aria-label="Preferences"
	tabindex="-1"
>
	<div
		class="w-96 rounded-lg border border-gray-700 bg-gray-900 shadow-2xl"
		style={isDragging ? 'cursor: grabbing;' : ''}
	>
		<div
			class="flex items-center justify-between border-b border-gray-800 px-3 py-2 cursor-grab select-none"
			onmousedown={onHeaderMouseDown}
			role="toolbar"
			aria-label="Drag to move"
			tabindex="-1"
		>
			<h2 class="text-xs font-semibold text-gray-100">Preferences</h2>
			<button
				bind:this={closeBtn}
				class="rounded p-1 text-gray-500 hover:bg-gray-800 hover:text-white transition-colors"
				onclick={onclose}
				aria-label="Close preferences"
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

		<div class="max-h-[70vh] overflow-y-auto px-3 py-2 space-y-4 text-xs">
			<!-- Graph Section -->
			<section>
				<h3 class="font-semibold uppercase tracking-wider text-gray-500 mb-2">Graph</h3>
				<div class="space-y-2">
					<div class="flex items-center justify-between">
						<span class="text-gray-300">Color Mode</span>
						<div class="flex gap-1" role="radiogroup" aria-label="Graph color mode">
							{#each colorModes as m (m.value)}
								<button
									class="whitespace-nowrap rounded px-2 py-1 text-xs transition-colors {$graphColorMode ===
									m.value
										? 'bg-blue-700/50 text-blue-300'
										: 'text-gray-400 hover:bg-gray-800 hover:text-white'}"
									onclick={() => setColorMode(m.value)}
									role="radio"
									aria-checked={$graphColorMode === m.value}
								>
									{m.label}
								</button>
							{/each}
						</div>
					</div>

					<div class="flex items-center justify-between">
						<span class="text-gray-300">Orientation</span>
						<div class="flex gap-1" role="radiogroup" aria-label="Graph orientation">
							{#each orientations as o (o.value)}
								<button
									class="whitespace-nowrap rounded px-2 py-1 text-xs transition-colors {$graphOrientation ===
									o.value
										? 'bg-blue-700/50 text-blue-300'
										: 'text-gray-400 hover:bg-gray-800 hover:text-white'}"
									onclick={() => setOrientation(o.value)}
									role="radio"
									aria-checked={$graphOrientation === o.value}
								>
									{o.label}
								</button>
							{/each}
						</div>
					</div>

					<div class="flex items-center justify-between">
						<span class="text-gray-300">Color Palette</span>
						<div class="flex gap-1" role="radiogroup" aria-label="Color palette">
							{#each palettes as p (p.value)}
								<button
									class="whitespace-nowrap rounded px-2 py-1 text-xs transition-colors {$graphPalette ===
									p.value
										? 'bg-blue-700/50 text-blue-300'
										: 'text-gray-400 hover:bg-gray-800 hover:text-white'}"
									onclick={() => setPalette(p.value)}
									role="radio"
									aria-checked={$graphPalette === p.value}
								>
									{p.label}
								</button>
							{/each}
						</div>
					</div>

					<div class="flex items-center justify-between">
						<span class="text-gray-300">Hide Merge Commits</span>
						<button
							class="relative h-5 w-9 rounded-full transition-colors {$graphHideMerges
								? 'bg-blue-600'
								: 'bg-gray-700'}"
							onclick={toggleHideMerges}
							role="switch"
							aria-checked={$graphHideMerges}
							aria-label="Toggle hide merge commits"
						>
							<span
								class="absolute top-0.5 left-0.5 h-4 w-4 rounded-full bg-white transition-transform {$graphHideMerges
									? 'translate-x-4'
									: 'translate-x-0'}"
							></span>
						</button>
					</div>
				</div>
			</section>

			<div class="border-t border-gray-800"></div>

			<!-- Diff Section -->
			<section>
				<h3 class="font-semibold uppercase tracking-wider text-gray-500 mb-2">Diff Viewer</h3>
				<div class="space-y-2">
					<div class="flex items-center justify-between">
						<span class="text-gray-300">Default Mode</span>
						<div class="flex gap-1" role="radiogroup" aria-label="Diff mode">
							{#each diffModes as d (d.value)}
								<button
									class="whitespace-nowrap rounded px-2 py-1 text-xs transition-colors {$diffMode ===
									d.value
										? 'bg-blue-700/50 text-blue-300'
										: 'text-gray-400 hover:bg-gray-800 hover:text-white'}"
									onclick={() => setDiffMode(d.value)}
									role="radio"
									aria-checked={$diffMode === d.value}
								>
									{d.label}
								</button>
							{/each}
						</div>
					</div>

					<div class="flex items-center justify-between">
						<span class="text-gray-300">Whitespace</span>
						<div class="flex gap-1" role="radiogroup" aria-label="Whitespace mode">
							{#each whitespaceModes as w (w.value)}
								<button
									class="whitespace-nowrap rounded px-2 py-1 text-xs transition-colors {$diffWhitespace ===
									w.value
										? 'bg-blue-700/50 text-blue-300'
										: 'text-gray-400 hover:bg-gray-800 hover:text-white'}"
									onclick={() => setDiffWhitespace(w.value)}
									role="radio"
									aria-checked={$diffWhitespace === w.value}
								>
									{w.label}
								</button>
							{/each}
						</div>
					</div>
				</div>
			</section>
		</div>
	</div>
</div>
