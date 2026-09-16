<script lang="ts">
	import { Calculator, CircleHelp, Sparkles, UserCheck } from '@lucide/svelte';

	let { source, confidence }: { source: string; confidence?: number | null } = $props();

	const details = $derived(
		source === 'profile'
			? { label: 'Confirmed profile', className: 'bg-positive/10 text-positive', icon: UserCheck }
			: source === 'derived'
				? { label: 'Calculated', className: 'bg-brand-soft text-brand-strong', icon: Calculator }
				: source === 'ai_draft'
					? { label: 'AI draft — review', className: 'bg-warning/10 text-warning', icon: Sparkles }
					: source === 'user' || source === 'participant'
						? {
								label: 'Confirmed answer',
								className: 'bg-positive/10 text-positive',
								icon: UserCheck
							}
						: { label: 'Needs answer', className: 'bg-canvas text-ink-muted', icon: CircleHelp }
	);
</script>

<span
	class="inline-flex items-center gap-1.5 rounded-full px-2.5 py-1 text-[11px] font-extrabold {details.className}"
>
	<details.icon size={13} />
	{details.label}
	{#if confidence != null && source === 'ai_draft'}
		· {Math.round(confidence * 100)}%
	{/if}
</span>
