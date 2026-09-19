<script lang="ts">
	import { Eye, Pencil, UserRound } from '@lucide/svelte';
	import type { DocumentField } from '$lib/api/client';
	import Button from './Button.svelte';
	import SourceBadge from './SourceBadge.svelte';

	let {
		fields,
		onedit,
		onpreview,
		previewing = false
	}: {
		fields: DocumentField[];
		onedit: (fieldId: string) => void;
		onpreview: () => Promise<void>;
		previewing?: boolean;
	} = $props();
</script>

<section class="surface overflow-hidden">
	<div
		class="flex flex-col justify-between gap-4 border-b border-line bg-canvas/70 p-5 sm:flex-row sm:items-center sm:p-6"
	>
		<div>
			<h2 class="text-xl font-extrabold text-ink">Answer review</h2>
			<p class="mt-1 text-xs text-ink-muted">
				Check the facts and sources before generating the real PDF preview.
			</p>
		</div>
		<Button onclick={onpreview} loading={previewing}><Eye size={17} /> Generate PDF preview</Button>
	</div>
	<div class="divide-y divide-line">
		{#each fields as field (field.id)}
			<div class="flex items-start gap-4 p-4 sm:p-5">
				<div class="min-w-0 flex-1">
					<div class="flex flex-wrap items-center gap-2">
						<p class="text-sm font-extrabold text-ink">{field.label}</p>
						<SourceBadge source={field.source} confidence={field.confidence} />
						{#if field.participant_id}
							<span
								class="inline-flex items-center gap-1 rounded-full bg-canvas px-2 py-1 text-[10px] font-bold text-ink-muted"
							>
								<UserRound size={11} /> Participant
							</span>
						{/if}
					</div>
					<p
						class="mt-2 text-sm leading-6 whitespace-pre-wrap {field.value ||
						field.value_preview ||
						field.required === false
							? 'text-ink'
							: 'font-semibold text-warning'}"
					>
						{field.value ??
							field.value_preview ??
							(field.required === false ? 'Left blank' : 'No confirmed answer')}
					</p>
				</div>
				{#if !field.participant_id && field.kind !== 'signature'}
					<Button
						variant="ghost"
						onclick={() => onedit(field.id)}
						aria-label={`Edit ${field.label}`}
					>
						<Pencil size={16} />
					</Button>
				{/if}
			</div>
		{/each}
	</div>
</section>
