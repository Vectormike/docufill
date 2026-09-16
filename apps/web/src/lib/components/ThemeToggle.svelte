<script lang="ts">
	import { Moon, Sun } from '@lucide/svelte';
	import { onMount } from 'svelte';

	let theme = $state<'light' | 'dark'>('light');
	let mounted = $state(false);

	onMount(() => {
		const saved = localStorage.getItem('docufill-theme');
		const prefersDark = window.matchMedia('(prefers-color-scheme: dark)').matches;
		theme = saved === 'dark' || (saved !== 'light' && prefersDark) ? 'dark' : 'light';
		applyTheme();
		mounted = true;
	});

	function toggle() {
		theme = theme === 'light' ? 'dark' : 'light';
		localStorage.setItem('docufill-theme', theme);
		applyTheme();
	}

	function applyTheme() {
		document.documentElement.dataset.theme = theme;
	}
</script>

<button
	type="button"
	onclick={toggle}
	disabled={!mounted}
	class="grid size-11 place-items-center rounded-full border border-line bg-surface-raised text-ink-muted transition hover:border-brand hover:text-ink"
	aria-label={`Switch to ${theme === 'light' ? 'dark' : 'light'} theme`}
	title={`Switch to ${theme === 'light' ? 'dark' : 'light'} theme`}
>
	{#if theme === 'light'}
		<Moon size={18} />
	{:else}
		<Sun size={18} />
	{/if}
</button>
