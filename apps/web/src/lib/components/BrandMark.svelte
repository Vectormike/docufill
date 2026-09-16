<script lang="ts">
	let { size = 'md', showWordmark = true }: { size?: 'sm' | 'md' | 'lg'; showWordmark?: boolean } =
		$props();

	const dimensions = $derived(size === 'sm' ? 32 : size === 'lg' ? 72 : 42);
</script>

<span class="inline-flex items-center gap-2.5" aria-label="Docufill" role="img">
	<svg
		width={dimensions}
		height={dimensions}
		viewBox="0 0 48 48"
		fill="none"
		aria-hidden="true"
		class="shrink-0"
	>
		<rect width="48" height="48" rx="14" fill="var(--brand)" />
		<path d="M14 9h14l7 7v23H14V9Z" fill="var(--ink)" />
		<path d="M28 9v8h7" fill="var(--brand)" />
		<path
			d="M19 23h11M19 28h9M19 33h6"
			stroke="var(--brand)"
			stroke-linecap="round"
			stroke-width="2.5"
		/>
	</svg>
	{#if showWordmark}
		<span
			class:text-2xl={size === 'lg'}
			class:text-base={size === 'sm'}
			class="font-extrabold tracking-[-0.04em] text-ink"
		>
			Docufill
		</span>
	{/if}
</span>
