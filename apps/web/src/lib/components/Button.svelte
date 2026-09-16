<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLButtonAttributes } from 'svelte/elements';

	let {
		children,
		variant = 'primary',
		loading = false,
		class: className = '',
		disabled,
		...rest
	}: HTMLButtonAttributes & {
		children: Snippet;
		variant?: 'primary' | 'secondary' | 'ghost' | 'danger';
		loading?: boolean;
	} = $props();

	const variants = {
		primary: 'border-brand bg-brand text-[#211d14] hover:brightness-95',
		secondary: 'border-line bg-surface-raised text-ink hover:bg-canvas',
		ghost: 'border-transparent bg-transparent text-ink-muted hover:bg-canvas hover:text-ink',
		danger: 'border-negative/25 bg-negative/8 text-negative hover:bg-negative/12'
	};
</script>

<button
	{...rest}
	disabled={disabled || loading}
	aria-busy={loading}
	class="inline-flex min-h-11 items-center justify-center gap-2 rounded-control border px-4 py-2.5 text-sm font-semibold transition duration-[var(--motion-fast)] ease-out disabled:cursor-not-allowed disabled:opacity-55 {variants[
		variant
	]} {className}"
>
	{#if loading}
		<span
			class="size-4 animate-spin rounded-full border-2 border-current border-r-transparent"
			aria-hidden="true"
		></span>
	{/if}
	{@render children()}
</button>
