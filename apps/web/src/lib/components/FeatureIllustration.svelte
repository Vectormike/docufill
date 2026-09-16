<script lang="ts">
	let { feature }: { feature: 'vault' | 'questions' | 'invite' } = $props();
</script>

<div
	class="grid aspect-[5/3] w-full place-items-center overflow-hidden rounded-[1.35rem] border border-line bg-brand-soft"
	aria-hidden="true"
>
	{#if feature === 'vault'}
		<svg viewBox="0 0 320 190" class="h-full w-full">
			<rect
				x="58"
				y="28"
				width="128"
				height="138"
				rx="18"
				fill="var(--surface-raised)"
				stroke="var(--line)"
			/>
			<rect x="80" y="52" width="52" height="9" rx="4.5" fill="var(--brand)" />
			<rect x="80" y="76" width="82" height="7" rx="3.5" fill="var(--line)" />
			<rect x="80" y="96" width="72" height="7" rx="3.5" fill="var(--line)" />
			<rect x="80" y="116" width="88" height="7" rx="3.5" fill="var(--line)" />
			<circle cx="214" cy="102" r="48" fill="var(--brand)" />
			<path
				d="M194 103l13 13 26-29"
				fill="none"
				stroke="#211d14"
				stroke-linecap="round"
				stroke-linejoin="round"
				stroke-width="10"
			/>
		</svg>
	{:else if feature === 'questions'}
		<svg viewBox="0 0 320 190" class="h-full w-full">
			<rect
				x="42"
				y="30"
				width="236"
				height="130"
				rx="22"
				fill="var(--surface-raised)"
				stroke="var(--line)"
			/>
			<circle cx="72" cy="65" r="14" fill="var(--brand)" />
			<rect x="98" y="57" width="118" height="8" rx="4" fill="var(--ink)" opacity=".8" />
			<rect x="98" y="75" width="78" height="6" rx="3" fill="var(--line)" />
			<rect
				x="62"
				y="105"
				width="196"
				height="34"
				rx="10"
				fill="var(--canvas)"
				stroke="var(--line)"
			/>
			<path
				d="M225 115l9 9 17-19"
				fill="none"
				stroke="var(--positive)"
				stroke-linecap="round"
				stroke-linejoin="round"
				stroke-width="5"
			/>
		</svg>
	{:else}
		<svg viewBox="0 0 320 190" class="h-full w-full">
			<circle cx="112" cy="94" r="42" fill="var(--surface-raised)" stroke="var(--line)" />
			<circle cx="208" cy="94" r="42" fill="var(--surface-raised)" stroke="var(--line)" />
			<circle cx="112" cy="80" r="13" fill="var(--brand)" />
			<path d="M88 118c8-25 40-25 48 0" fill="var(--brand)" />
			<circle cx="208" cy="80" r="13" fill="var(--ink-muted)" />
			<path d="M184 118c8-25 40-25 48 0" fill="var(--ink-muted)" />
			<path
				d="M154 94h12m-6-6 6 6-6 6"
				fill="none"
				stroke="var(--brand-strong)"
				stroke-linecap="round"
				stroke-linejoin="round"
				stroke-width="5"
			/>
			<path
				d="M211 136c18 0 34 8 42 22"
				fill="none"
				stroke="var(--brand-strong)"
				stroke-linecap="round"
				stroke-width="5"
			/>
		</svg>
	{/if}
</div>
