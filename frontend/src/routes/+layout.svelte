<script lang="ts">
	import '../app.css';
	import { initLogging } from '#lib/logging.ts';
	import { locale } from '#lib/stores/locale.ts';

	initLogging();

	let { children } = $props();

	$effect(() => {
		if (typeof document !== 'undefined') {
			document.documentElement.lang = $locale;
		}
	});
</script>

{@render children()}
