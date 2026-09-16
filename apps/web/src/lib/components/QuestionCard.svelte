<script lang="ts">
	import { ArrowLeft, ArrowRight, Check, X } from '@lucide/svelte';
	import type { DocumentField } from '$lib/api/client';
	import Button from './Button.svelte';
	import SourceBadge from './SourceBadge.svelte';

	let {
		field,
		index,
		total,
		onsave,
		onreject,
		onback,
		onnext
	}: {
		field: DocumentField;
		index: number;
		total: number;
		onsave: (value: string) => Promise<void>;
		onreject: () => Promise<void>;
		onback: () => void;
		onnext: () => void;
	} = $props();

	let answer = $state('');
	let saving = $state(false);
	let saved = $state(false);
	let editingAi = $state(false);
	let timer: ReturnType<typeof setTimeout> | undefined;

	$effect(() => {
		answer = field.value ?? field.value_preview ?? '';
		editingAi = field.source !== 'ai_draft';
		saved = Boolean(field.confirmed_at);
	});

	function scheduleSave() {
		saved = false;
		if (!editingAi || !answer.trim()) return;
		clearTimeout(timer);
		timer = setTimeout(() => void save(), 700);
	}

	async function save() {
		if (!answer.trim()) return;
		clearTimeout(timer);
		saving = true;
		try {
			await onsave(answer.trim());
			saved = true;
		} finally {
			saving = false;
		}
	}

	async function acceptDraft() {
		editingAi = true;
		await save();
	}

	async function next() {
		if (answer.trim() && editingAi && !saved) await save();
		onnext();
	}
</script>

<article class="surface overflow-hidden">
	<div class="border-b border-line bg-canvas/70 px-5 py-4 sm:px-7">
		<div class="flex items-center justify-between gap-3">
			<p class="text-xs font-extrabold tracking-[0.14em] text-ink-muted uppercase">
				Question {index + 1} of {total}
			</p>
			<SourceBadge source={field.source} confidence={field.confidence} />
		</div>
		<div class="mt-3 h-1.5 overflow-hidden rounded-full bg-line">
			<div
				class="h-full rounded-full bg-brand transition-all"
				style={`width: ${((index + 1) / total) * 100}%`}
			></div>
		</div>
	</div>

	<div class="p-5 sm:p-7">
		<p class="text-xs font-bold text-ink-muted">Page {field.page_number} · {field.kind}</p>
		<h2 class="mt-2 text-2xl font-semibold tracking-tight text-balance text-ink">
			{field.label}
		</h2>
		{#if field.instructions}
			<p class="mt-3 rounded-xl border border-line bg-canvas p-3 text-sm leading-6 text-ink-muted">
				{field.instructions}
			</p>
		{/if}

		<div class="mt-6">
			{#if field.kind === 'checkbox' || field.kind === 'declaration'}
				<label
					class="flex min-h-14 cursor-pointer items-center gap-3 rounded-xl border border-line bg-canvas p-4"
				>
					<input
						type="checkbox"
						checked={['true', 'yes', 'checked'].includes(answer.toLowerCase())}
						onchange={(event) => {
							answer = event.currentTarget.checked ? 'yes' : 'no';
							scheduleSave();
						}}
						class="rounded border-line text-brand focus:ring-brand"
					/>
					<span class="text-sm font-bold text-ink">Yes, I confirm</span>
				</label>
			{:else if field.kind === 'multiline' || field.kind === 'address'}
				<textarea
					rows="5"
					bind:value={answer}
					oninput={scheduleSave}
					disabled={!editingAi}
					class="w-full rounded-control border-line bg-canvas text-base leading-7 text-ink disabled:opacity-75"
					placeholder="Type your answer"></textarea>
			{:else}
				<input
					type={field.kind === 'date'
						? 'date'
						: field.kind === 'email'
							? 'email'
							: field.kind === 'phone'
								? 'tel'
								: field.kind === 'number'
									? 'number'
									: 'text'}
					bind:value={answer}
					oninput={scheduleSave}
					disabled={!editingAi}
					class="min-h-13 w-full rounded-control border-line bg-canvas text-base text-ink disabled:opacity-75"
					placeholder="Type your answer"
				/>
			{/if}

			{#if field.source === 'ai_draft' && !field.confirmed_at}
				<div class="mt-4 rounded-xl border border-warning/25 bg-warning/8 p-4">
					<p class="text-xs leading-5 font-bold text-warning">
						{field.source_explanation ?? 'This draft used related confirmed details.'}
					</p>
					<p class="mt-1 text-xs leading-5 text-ink-muted">
						Based on {field.source_reference_count}
						{field.source_reference_count === 1 ? 'approved detail' : 'approved details'}. Docufill
						will never treat this draft as fact until you accept or edit it.
					</p>
					<div class="mt-3 flex flex-wrap gap-2">
						<Button onclick={acceptDraft} loading={saving}><Check size={16} /> Accept</Button>
						<Button variant="secondary" onclick={() => (editingAi = true)}>Edit</Button>
						<Button variant="ghost" onclick={onreject}><X size={16} /> Reject</Button>
					</div>
				</div>
			{/if}

			<p
				class="mt-3 min-h-5 text-xs font-semibold {saved ? 'text-positive' : 'text-ink-muted'}"
				aria-live="polite"
			>
				{saving ? 'Saving securely…' : saved ? 'Saved' : answer ? 'Autosaves after you pause' : ''}
			</p>
		</div>

		<div class="mt-7 flex items-center justify-between gap-3">
			<Button variant="ghost" onclick={onback} disabled={index === 0}
				><ArrowLeft size={16} /> Back</Button
			>
			<Button onclick={next} disabled={!answer.trim() || (!editingAi && !field.confirmed_at)}>
				{index === total - 1 ? 'Review answers' : 'Next'}
				<ArrowRight size={16} />
			</Button>
		</div>
	</div>
</article>
