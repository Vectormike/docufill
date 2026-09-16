<script lang="ts">
	import { resolve } from '$app/paths';
	import BrandMark from './BrandMark.svelte';
</script>

<header class="border-b border-line bg-surface">
	<div class="mx-auto flex min-h-18 max-w-4xl items-center justify-between px-5 sm:px-8">
		<a href={resolve('/')} aria-label="Docufill home"><BrandMark size="sm" /></a>
		<a
			href={resolve('/')}
			class="inline-flex min-h-11 items-center text-sm font-extrabold text-brand-strong"
			>Return home</a
		>
	</div>
</header>
