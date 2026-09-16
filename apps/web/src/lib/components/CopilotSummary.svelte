<script lang="ts">
	import { AlertTriangle, CheckCircle2, PenLine, Sparkles, UsersRound } from '@lucide/svelte';
	import type { DocumentDetail } from '$lib/api/client';
	import Button from './Button.svelte';

	let {
		document,
		onstart,
		onreview
	}: { document: DocumentDetail; onstart: () => void; onreview: () => void } = $props();

	const items = $derived([
		{
			label: 'Grounded answers',
			value: document.copilot.grounded_fields,
			icon: CheckCircle2,
			tone: 'text-positive'
		},
		{
			label: 'Need your input',
			value: document.copilot.questions_needing_input,
			icon: AlertTriangle,
			tone: 'text-warning'
		},
		{
			label: 'AI drafts to review',
			value: document.copilot.ambiguous_fields,
			icon: Sparkles,
			tone: 'text-warning'
		},
		{
			label: 'Signature fields',
			value: document.copilot.signature_fields,
			icon: PenLine,
			tone: 'text-brand-strong'
		},
		{
			label: 'Participant sections',
			value: document.copilot.participant_sections,
			icon: UsersRound,
			tone: 'text-brand-strong'
		}
	]);
</script>

<section class="surface overflow-hidden">
	<div class="border-b border-line bg-brand-soft/60 p-5 sm:p-7">
		<p class="eyebrow">Document Copilot</p>
		<h2 class="mt-2 text-2xl font-semibold tracking-tight text-ink">
			I found {document.copilot.total_fields} fields
		</h2>
		<p class="mt-2 text-sm leading-6 text-ink-muted">
			Start with the small set that needs attention. You can still review every proposed answer.
		</p>
	</div>
	<div class="grid grid-cols-2 gap-px bg-line sm:grid-cols-5">
		{#each items as item (item.label)}
			<div class="bg-surface-raised p-4">
				<item.icon size={17} class={item.tone} />
				<p class="mt-3 text-2xl font-extrabold text-ink">{item.value}</p>
				<p class="mt-1 text-[11px] leading-4 font-bold text-ink-muted">{item.label}</p>
			</div>
		{/each}
	</div>
	<div class="flex flex-col gap-3 p-5 sm:flex-row sm:justify-end sm:p-6">
		<Button variant="secondary" onclick={onreview}>Review all answers</Button>
		<Button onclick={onstart}>
			{document.copilot.questions_needing_input > 0
				? 'Answer missing questions'
				: 'Review questions'}
		</Button>
	</div>
</section>
