<script lang="ts">
	import '@fontsource-variable/manrope';
	import '@fontsource-variable/newsreader';
	import './layout.css';
	import favicon from '$lib/assets/favicon.svg';
	import ConnectivityBanner from '$lib/components/ConnectivityBanner.svelte';

	let { children } = $props();
</script>

<svelte:head>
	<link rel="icon" href={favicon} />
	<title>Docufill — Fill forms once</title>
</svelte:head>
<ConnectivityBanner />
{@render children()}

