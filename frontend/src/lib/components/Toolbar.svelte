<script lang="ts">
	import { graphColorMode, graphHideMerges, graphOrientation } from '$lib/stores/repository';

	function toggleColorMode() {
		graphColorMode.update((v) => (v === 'by-branch' ? 'by-author' : 'by-branch'));
	}

	function toggleHideMerges() {
		graphHideMerges.update((v) => !v);
	}

	function toggleOrientation() {
		graphOrientation.update((v) => (v === 'top-to-bottom' ? 'bottom-to-top' : 'top-to-bottom'));
	}
</script>

<div class="flex items-center gap-1">
	<button
		class="rounded px-2 py-1 text-xs transition-colors {$graphColorMode === 'by-author'
			? 'bg-blue-700/50 text-blue-300'
			: 'text-gray-400 hover:bg-gray-800 hover:text-white'}"
		onclick={toggleColorMode}
		title="Toggle color by author"
	>
		<svg class="h-3.5 w-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
			<path
				stroke-linecap="round"
				stroke-linejoin="round"
				stroke-width="2"
				d="M16 7a4 4 0 11-8 0 4 4 0 018 0zM12 14a7 7 0 00-7 7h14a7 7 0 00-7-7z"
			/>
		</svg>
	</button>

	<button
		class="rounded px-2 py-1 text-xs transition-colors {$graphHideMerges
			? 'bg-blue-700/50 text-blue-300'
			: 'text-gray-400 hover:bg-gray-800 hover:text-white'}"
		onclick={toggleHideMerges}
		title="Toggle hide merge commits"
	>
		<svg class="h-3.5 w-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
			<path
				stroke-linecap="round"
				stroke-linejoin="round"
				stroke-width="2"
				d="M13 10V3L4 14h7v7l9-11h-7z"
			/>
		</svg>
	</button>

	<button
		class="rounded px-2 py-1 text-xs transition-colors {$graphOrientation === 'bottom-to-top'
			? 'bg-blue-700/50 text-blue-300'
			: 'text-gray-400 hover:bg-gray-800 hover:text-white'}"
		onclick={toggleOrientation}
		title="Toggle graph orientation"
	>
		<svg class="h-3.5 w-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
			<path
				stroke-linecap="round"
				stroke-linejoin="round"
				stroke-width="2"
				d="M7 16V4m0 0L3 8m4-4l4 4m6 0v12m0 0l4-4m-4 4l-4-4"
			/>
		</svg>
	</button>
</div>
