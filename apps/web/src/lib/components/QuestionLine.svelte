<script lang="ts">
	import type { DocumentField } from '$lib/api/client';
	import SourceBadge from './SourceBadge.svelte';

	let {
		field,
		number,
		active,
		onactivate,
		onsave,
		onreject,
		onnext
	}: {
		field: DocumentField;
		number: number;
		active: boolean;
		onactivate: () => void;
		onsave: (value: string) => Promise<void>;
		onreject: () => Promise<void>;
		onnext: () => void;
	} = $props();

	let answer = $state('');
	let saving = $state(false);
	let saved = $state(false);
	let editingDraft = $state(false);
	let timer: ReturnType<typeof setTimeout> | undefined;

	const delegated = $derived(Boolean(field.participant_id));
	const draft = $derived(field.source === 'ai_draft' && !field.confirmed_at);
	const value = $derived(field.value ?? field.value_preview ?? '');
	const inputType = $derived(
		field.kind === 'date'
			? 'date'
			: field.kind === 'email'
				? 'email'
				: field.kind === 'phone'
					? 'tel'
					: field.kind === 'number'
						? 'number'
						: 'text'
	);

	$effect(() => {
		answer = field.value ?? field.value_preview ?? '';
		editingDraft = false;
		saved = Boolean(field.confirmed_at);
	});

	function scheduleSave() {
		saved = false;
		if (!answer.trim()) return;
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

	async function saveAndContinue() {
		if (answer.trim() && !saved) await save();
		onnext();
	}
</script>

<div
	class="grid grid-cols-[1.5rem_minmax(0,1fr)_4rem] items-baseline gap-3 border-b border-line-soft transition-colors duration-[var(--motion-fast)] sm:grid-cols-[2rem_minmax(0,1fr)_6.5rem] sm:gap-5 sm:px-7 {active
		? 'bg-surface px-4 py-5'
		: 'px-4 py-3'}"
>
	<span class="gutter tabular-nums">{String(number).padStart(2, '0')}</span>

	<div class="min-w-0">
		{#if active}
			<h2 class="ask text-lg leading-7 text-ink sm:text-xl">
				<span class="mark">{field.label}</span>
			</h2>
			{#if field.instructions}
				<p class="mt-2 text-xs leading-5 text-ink-muted">{field.instructions}</p>
			{/if}

			<div class="mt-4">
				{#if field.kind === 'checkbox' || field.kind === 'declaration'}
					<label class="flex min-h-11 cursor-pointer items-center gap-3">
						<input
							type="checkbox"
							checked={['true', 'yes', 'checked'].includes(answer.toLowerCase())}
							onchange={(event) => {
								answer = event.currentTarget.checked ? 'yes' : 'no';
								scheduleSave();
							}}
							class="rounded-sm border-line text-ink focus:ring-brand-strong"
						/>
						<span class="text-sm text-ink">Yes, I confirm</span>
					</label>
				{:else if field.kind === 'multiline' || field.kind === 'address'}
					<textarea
						rows="3"
						bind:value={answer}
						oninput={scheduleSave}
						disabled={draft && !editingDraft}
						placeholder="Type your answer"
						class="w-full resize-none rounded-none border-0 border-b-[1.5px] border-line bg-transparent px-0 py-1 text-base leading-7 text-ink focus:border-ink focus:ring-0 disabled:opacity-70"
					></textarea>
				{:else}
					<input
						type={inputType}
						bind:value={answer}
						oninput={scheduleSave}
						onkeydown={(event) => {
							if (event.key === 'Enter') {
								event.preventDefault();
								void saveAndContinue();
							}
						}}
						disabled={draft && !editingDraft}
						placeholder="Type your answer"
						class="w-full rounded-none border-0 border-b-[1.5px] border-line bg-transparent px-0 py-1 text-base text-ink focus:border-ink focus:ring-0 disabled:opacity-70"
					/>
				{/if}
			</div>

			{#if draft}
				<p class="mt-3 text-xs leading-5 text-ink-muted">
					{field.source_explanation ?? 'This draft used related confirmed details.'} Nothing is treated
					as fact until you accept it.
				</p>
			{/if}

			<div class="mt-4 flex flex-wrap items-center gap-x-5 gap-y-2">
				{#if draft}
					<button
						type="button"
						onclick={save}
						class="text-xs font-semibold text-ink underline decoration-brand decoration-[1.5px] underline-offset-4"
						>Accept</button
					>
					<button
						type="button"
						onclick={() => (editingDraft = true)}
						class="text-xs text-ink-muted hover:text-ink">Edit</button
					>
					<button type="button" onclick={onreject} class="text-xs text-ink-muted hover:text-ink"
						>Reject</button
					>
				{:else}
					<button
						type="button"
						onclick={saveAndContinue}
						disabled={!answer.trim()}
						class="text-xs font-semibold text-ink underline decoration-brand decoration-[1.5px] underline-offset-4 disabled:no-underline disabled:opacity-45"
						>Save and continue</button
					>
				{/if}
				<span class="gutter" aria-live="polite">
					{saving ? 'saving' : saved ? 'saved' : answer ? 'autosaves' : ''}
				</span>
			</div>
		{:else}
			{#snippet summary()}
				<span class="ask block truncate text-sm text-ink sm:text-[0.95rem]">
					{#if !field.confirmed_at && !delegated}
						<span class="mark">{field.label}</span>
					{:else}
						{field.label}
					{/if}
				</span>
				<span
					class="mt-0.5 block truncate text-xs text-ink-muted {delegated ? 'font-mono' : ''} {draft
						? 'italic'
						: ''}"
				>
					{delegated ? 'awaiting participant' : value || 'No answer yet'}
				</span>
			{/snippet}

			{#if delegated}
				<div>{@render summary()}</div>
			{:else}
				<button
					type="button"
					onclick={onactivate}
					class="block w-full text-left hover:underline hover:decoration-line hover:underline-offset-4"
				>
					{@render summary()}
				</button>
			{/if}
		{/if}
	</div>

	<div class="text-right">
		{#if delegated}
			<span class="gutter">guarantor</span>
		{:else}
			<SourceBadge source={field.source} confidence={field.confidence} />
		{/if}
	</div>
</div>
