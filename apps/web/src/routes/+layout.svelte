<script lang="ts">
	import '@fontsource-variable/manrope';
	// Weight axis only; the unicode-range split keeps non-Latin files unfetched.
	import '@fontsource-variable/newsreader/wght.css';
	import './layout.css';
	import favicon from '$lib/assets/favicon.svg';
	import ConnectivityBanner from '$lib/components/ConnectivityBanner.svelte';
	import UpdatePrompt from '$lib/components/UpdatePrompt.svelte';

	let { children } = $props();
</script>

<svelte:head>
	<link rel="icon" href={favicon} />
	<title>Docufill — Fill forms once</title>
</svelte:head>
<ConnectivityBanner />
<UpdatePrompt />
{@render children()}
