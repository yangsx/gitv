<script lang="ts">
	import { toasts, dismissToast, type Toast } from '$lib/stores/toast';

	function severityClasses(severity: Toast['severity']): string {
		switch (severity) {
			case 'error':
				return 'bg-red-900/90 border-red-700 text-red-100';
			case 'warning':
				return 'bg-amber-900/90 border-amber-700 text-amber-100';
			default:
				return 'bg-gray-800/90 border-gray-600 text-gray-100';
		}
	}
</script>

{#if $toasts.length > 0}
	<div class="fixed bottom-4 right-4 z-50 flex flex-col gap-2 pointer-events-none">
		{#each $toasts as toast (toast.id)}
			<div
				class="pointer-events-auto flex items-center gap-2 rounded border px-3 py-2 text-sm shadow-lg {severityClasses(
					toast.severity
				)}"
			>
				<span class="flex-1">{toast.message}</span>
				{#if toast.severity === 'error'}
					<button
						class="ml-2 opacity-60 hover:opacity-100"
						onclick={() => dismissToast(toast.id)}
						aria-label="Dismiss"
					>
						&times;
					</button>
				{/if}
			</div>
		{/each}
	</div>
{/if}
