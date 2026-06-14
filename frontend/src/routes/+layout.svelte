<script lang="ts">
	import '../app.css';
	import { initLogging } from '$lib/logging';
	import { locale } from '$lib/stores/locale';

	initLogging();

	let { children } = $props();

	$effect(() => {
		if (typeof document !== 'undefined') {
			document.documentElement.lang = $locale;
		}
	});
</script>

{@render children()}
