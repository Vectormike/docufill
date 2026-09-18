<script lang="ts">
	let { feature }: { feature: 'vault' | 'questions' | 'invite' } = $props();

	// Ruled rows share one rhythm so every illustration reads as the same page.
	const rows = [48, 70, 92, 114, 136];
</script>

<div
	class="grid aspect-[5/3] w-full place-items-center overflow-hidden rounded-card border border-line bg-canvas"
	aria-hidden="true"
>
	{#if feature === 'vault'}
		<svg viewBox="0 0 320 190" class="h-full w-full">
			<rect
				x="70"
				y="18"
				width="180"
				height="154"
				fill="var(--surface)"
				stroke="var(--line)"
				rx="2"
			/>
			<line x1="98" y1="18" x2="98" y2="172" stroke="var(--line-soft)" stroke-width="1.5" />
			{#each rows as row, index (row)}
				{#if index === 2}
					<!-- The single gap is the only thing marked; the rest came from the vault. -->
					<rect x="106" y={row - 5} width="64" height="15" fill="var(--marker)" />
				{/if}
				<rect x="82" y={row} width="8" height="2" fill="var(--line)" />
				<rect
					x="110"
					y={row}
					width={index === 2 ? 44 : index === 0 ? 88 : 68}
					height="5"
					fill={index === 2 ? 'var(--line)' : 'var(--ink)'}
					opacity={index === 2 ? '1' : '0.8'}
				/>
			{/each}
			<line x1="110" y1="158" x2="180" y2="158" stroke="var(--line-soft)" stroke-width="1.5" />
		</svg>
	{:else if feature === 'questions'}
		<svg viewBox="0 0 320 190" class="h-full w-full">
			<rect
				x="46"
				y="18"
				width="228"
				height="154"
				fill="var(--surface)"
				stroke="var(--line)"
				rx="2"
			/>
			<line x1="76" y1="18" x2="76" y2="172" stroke="var(--line-soft)" stroke-width="1.5" />
			{#each rows.slice(0, 2) as row (row)}
				<rect x="60" y={row} width="8" height="2" fill="var(--line)" />
				<rect x="88" y={row} width="104" height="5" fill="var(--ink)" opacity="0.82" />
				<rect x="206" y={row} width="46" height="4" fill="var(--line)" />
			{/each}

			<rect x="60" y="92" width="8" height="2" fill="var(--ink)" />
			<rect x="86" y="84" width="122" height="17" fill="var(--marker)" />
			<rect x="88" y="90" width="118" height="6" fill="var(--ink)" />
			<line x1="88" y1="120" x2="208" y2="120" stroke="var(--ink)" stroke-width="2" />
			<rect x="88" y="108" width="5" height="10" fill="var(--ink)" />

			<rect x="60" y="148" width="8" height="2" fill="var(--line)" />
			<rect x="88" y="148" width="76" height="5" fill="var(--line)" />
			<rect x="206" y="147" width="46" height="4" fill="var(--line)" />
		</svg>
	{:else}
		<svg viewBox="0 0 320 190" class="h-full w-full">
			<rect
				x="26"
				y="18"
				width="132"
				height="154"
				fill="var(--surface)"
				stroke="var(--line)"
				rx="2"
			/>
			{#each rows as row, index (row)}
				<rect x="42" y={row} width={index % 2 === 0 ? 96 : 74} height="5" fill="var(--ink)" />
			{/each}
			<rect x="42" y="158" width="52" height="5" fill="var(--line)" />

			<line x1="166" y1="95" x2="196" y2="95" stroke="var(--brand-strong)" stroke-width="2" />
			<path
				d="M190 89l7 6-7 6"
				fill="none"
				stroke="var(--brand-strong)"
				stroke-width="2"
				stroke-linecap="round"
				stroke-linejoin="round"
			/>

			<rect
				x="204"
				y="50"
				width="90"
				height="90"
				fill="var(--surface)"
				stroke="var(--line)"
				rx="2"
			/>
			<rect x="218" y="70" width="62" height="15" fill="var(--marker)" />
			<rect x="220" y="76" width="58" height="5" fill="var(--ink)" />
			<rect x="218" y="102" width="50" height="15" fill="var(--marker)" />
			<rect x="220" y="108" width="46" height="5" fill="var(--ink)" />
		</svg>
	{/if}
</div>
