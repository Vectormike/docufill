<script lang="ts">
	import { Check, ChevronDown, Mail, Search } from '@lucide/svelte';
	import type { DocumentField } from '$lib/api/client';
	import Button from './Button.svelte';

	let {
		fields,
		loading = false,
		oninvite,
		oncancel
	}: {
		fields: DocumentField[];
		loading?: boolean;
		oninvite: (input: {
			display_name: string;
			contact: string;
			role: string;
			field_ids: string[];
		}) => Promise<void>;
		oncancel: () => void;
	} = $props();

	const roles = [
		['guarantor', 'Guarantor'],
		['co_applicant', 'Co-applicant'],
		['other', 'Other']
	];

	let name = $state('');
	let email = $state('');
	let role = $state('guarantor');
	let selectedFieldIds = $state<string[]>([]);
	let showQuestions = $state(false);
	let search = $state('');

	const availableFields = $derived(fields.filter((field) => !field.participant_id));
	const filteredFields = $derived(
		availableFields.filter((field) =>
			field.label.toLowerCase().includes(search.trim().toLowerCase())
		)
	);

	async function submit(event: SubmitEvent) {
		event.preventDefault();
		await oninvite({
			display_name: name.trim(),
			contact: email.trim(),
			role,
			field_ids: selectedFieldIds
		});
	}

	function toggleField(fieldId: string, checked: boolean) {
		selectedFieldIds = checked
			? [...selectedFieldIds, fieldId]
			: selectedFieldIds.filter((id) => id !== fieldId);
	}
</script>

<form class="mt-5 min-w-0 rounded-xl border border-line bg-canvas p-4 sm:p-5" onsubmit={submit}>
	<div>
		<h3 class="text-base font-extrabold text-ink">Invite a participant</h3>
		<p class="mt-1 text-xs leading-5 text-ink-muted">
			They’ll receive a private link to answer only the questions you select.
		</p>
	</div>

	<div class="mt-5 grid min-w-0 gap-4 md:grid-cols-2">
		<label class="min-w-0 text-xs font-extrabold tracking-wider text-ink-muted uppercase">
			Name
			<input
				required
				minlength="2"
				maxlength="100"
				bind:value={name}
				placeholder="e.g. John Doe"
				class="mt-2 min-h-11 w-full min-w-0 rounded-control border-line bg-surface-raised text-sm text-ink"
			/>
		</label>
		<label class="min-w-0 text-xs font-extrabold tracking-wider text-ink-muted uppercase">
			Email
			<input
				required
				type="email"
				bind:value={email}
				placeholder="john@example.com"
				class="mt-2 min-h-11 w-full min-w-0 rounded-control border-line bg-surface-raised text-sm text-ink"
			/>
		</label>
	</div>

	<fieldset class="mt-5 min-w-0">
		<legend class="text-xs font-extrabold tracking-wider text-ink-muted uppercase">Role</legend>
		<div class="mt-2 grid gap-2 sm:grid-cols-3">
			{#each roles as option (option[0])}
				<button
					type="button"
					aria-pressed={role === option[0]}
					onclick={() => (role = option[0])}
					class="flex min-h-11 items-center justify-center gap-2 rounded-xl border px-3 text-sm font-bold transition {role ===
					option[0]
						? 'border-brand bg-brand-soft text-ink'
						: 'border-line bg-surface-raised text-ink-muted hover:border-brand'}"
				>
					{#if role === option[0]}<Check size={15} class="text-brand-strong" />{/if}
					{option[1]}
				</button>
			{/each}
		</div>
	</fieldset>

	<div class="mt-5 min-w-0 rounded-xl border border-line bg-surface-raised">
		<div class="flex flex-wrap items-center gap-3 p-4">
			<div class="min-w-0 flex-1">
				<p class="text-sm font-extrabold text-ink">Assigned questions</p>
				<p class="mt-0.5 text-xs text-ink-muted">
					{selectedFieldIds.length === 0
						? 'None selected yet'
						: `${selectedFieldIds.length} selected`}
				</p>
			</div>
			<Button type="button" variant="secondary" onclick={() => (showQuestions = !showQuestions)}>
				{showQuestions ? 'Done' : 'Choose'}
				<ChevronDown
					size={15}
					class={`transition-transform ${showQuestions ? 'rotate-180' : ''}`}
				/>
			</Button>
		</div>

		{#if showQuestions}
			<div class="border-t border-line p-3 sm:p-4">
				<label class="relative block">
					<span class="sr-only">Search questions</span>
					<Search
						size={16}
						class="pointer-events-none absolute top-1/2 left-3 -translate-y-1/2 text-ink-muted"
					/>
					<input
						type="search"
						bind:value={search}
						placeholder="Search questions"
						class="min-h-11 w-full rounded-control border-line bg-canvas pr-3 pl-9 text-sm text-ink"
					/>
				</label>
				<div class="mt-3 max-h-72 space-y-1 overflow-y-auto">
					{#each filteredFields as field (field.id)}
						<label
							class="flex min-w-0 cursor-pointer items-start gap-3 rounded-lg p-3 hover:bg-brand-soft"
						>
							<input
								type="checkbox"
								checked={selectedFieldIds.includes(field.id)}
								onchange={(event) => toggleField(field.id, event.currentTarget.checked)}
								class="mt-0.5 shrink-0 rounded border-line text-brand"
							/>
							<span class="min-w-0">
								<span class="block text-sm leading-5 font-semibold text-ink">{field.label}</span>
								<span class="mt-0.5 block text-[10px] text-ink-muted">
									Page {field.page_number} · {field.kind}
								</span>
							</span>
						</label>
					{:else}
						<p class="p-4 text-center text-sm text-ink-muted">No questions match your search.</p>
					{/each}
				</div>
				{#if selectedFieldIds.length > 0}
					<button
						type="button"
						onclick={() => (selectedFieldIds = [])}
						class="mt-3 min-h-10 text-xs font-bold text-negative"
					>
						Clear selection
					</button>
				{/if}
			</div>
		{/if}
	</div>

	<div class="mt-5 flex flex-col-reverse gap-2 sm:flex-row sm:justify-end">
		<Button type="button" variant="ghost" onclick={oncancel}>Cancel</Button>
		<Button type="submit" {loading} disabled={selectedFieldIds.length === 0}>
			<Mail size={16} /> Send invite
		</Button>
	</div>
</form>
