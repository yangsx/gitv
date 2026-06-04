<script lang="ts">
	import '../app.css';
	import { initLogging } from '$lib/logging';

	initLogging();

	let { children } = $props();
</script>

{@render children()}
