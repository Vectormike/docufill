<script lang="ts">
	let { source, confidence }: { source: string; confidence?: number | null } = $props();

	const label = $derived(
		source === 'profile'
			? 'your profile'
			: source === 'derived'
				? 'calculated'
				: source === 'ai_draft'
					? 'draft — review'
					: source === 'user'
						? 'you answered'
						: source === 'participant'
							? 'participant'
							: 'needs you'
	);
</script>

<span class="gutter whitespace-nowrap {source === 'missing' ? 'text-ink' : ''}">
	{label}{#if confidence != null && source === 'ai_draft'}
		· {Math.round(confidence * 100)}%
	{/if}
</span>
