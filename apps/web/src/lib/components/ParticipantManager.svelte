<script lang="ts">
	import { Check, Copy, Mail, Trash2, UserPlus, UsersRound } from '@lucide/svelte';
	import { onMount } from 'svelte';
	import {
		api,
		type DocumentField,
		type ParticipantInvitation,
		type ParticipantSummary
	} from '$lib/api/client';
	import Button from './Button.svelte';

	let {
		documentId,
		fields,
		onassignment
	}: {
		documentId: string;
		fields: DocumentField[];
		onassignment: (fieldId: string, participantId: string | null) => void;
	} = $props();

	let participants = $state<ParticipantSummary[]>([]);
	let showForm = $state(false);
	let name = $state('');
	let email = $state('');
	let role = $state('guarantor');
	let loading = $state(false);
	let error = $state('');
	let invitation = $state<ParticipantInvitation | null>(null);
	let copied = $state(false);
	let selectedFieldIds = $state<string[]>([]);

	onMount(() => void load());

	async function load() {
		try {
			participants = await api.participants(documentId);
		} catch (cause) {
			error = cause instanceof Error ? cause.message : 'Participants could not be loaded.';
		}
	}

	async function invite(event: SubmitEvent) {
		event.preventDefault();
		loading = true;
		error = '';
		try {
			invitation = await api.inviteParticipant(documentId, {
				display_name: name.trim(),
				contact: email.trim(),
				role,
				field_ids: selectedFieldIds,
				send_invitation: true
			});
			participants = [...participants, invitation.participant];
			for (const fieldId of selectedFieldIds) onassignment(fieldId, invitation.participant.id);
			name = '';
			email = '';
			selectedFieldIds = [];
			showForm = false;
		} catch (cause) {
			error = cause instanceof Error ? cause.message : 'The invitation could not be created.';
		} finally {
			loading = false;
		}
	}

	async function copyLink() {
		if (!invitation) return;
		await navigator.clipboard.writeText(invitation.share_url);
		copied = true;
		setTimeout(() => (copied = false), 1800);
	}

	async function assign(fieldId: string, participantId: string | null) {
		loading = true;
		try {
			await api.assignField(documentId, fieldId, participantId);
			onassignment(fieldId, participantId);
		} catch (cause) {
			error = cause instanceof Error ? cause.message : 'The field could not be assigned.';
		} finally {
			loading = false;
		}
	}

	async function revoke(participant: ParticipantSummary) {
		if (!window.confirm(`Revoke ${participant.display_name}’s access?`)) return;
		await api.revokeParticipant(documentId, participant.id);
		participants = participants.map((item) =>
			item.id === participant.id ? { ...item, status: 'revoked' } : item
		);
	}
</script>

<section class="surface p-5 sm:p-7">
	<div class="flex flex-col justify-between gap-4 sm:flex-row sm:items-center">
		<div>
			<div class="flex items-center gap-2">
				<UsersRound size={19} class="text-brand-strong" />
				<h2 class="text-xl font-extrabold text-ink">Other participants</h2>
			</div>
			<p class="mt-1 text-xs leading-5 text-ink-muted">
				Each person gets a unique expiring link and sees only assigned questions.
			</p>
		</div>
		<Button variant="secondary" onclick={() => (showForm = !showForm)}>
			<UserPlus size={16} /> Invite someone
		</Button>
	</div>

	{#if error}<p
			class="mt-4 rounded-xl bg-negative/10 p-3 text-sm font-semibold text-negative"
			role="alert"
		>
			{error}
		</p>{/if}
	{#if showForm}
		<form
			class="mt-5 grid gap-3 rounded-2xl border border-line bg-canvas p-4 sm:grid-cols-2"
			onsubmit={invite}
		>
			<label class="text-xs font-extrabold tracking-wider text-ink-muted uppercase">
				Name
				<input
					required
					minlength="2"
					maxlength="100"
					bind:value={name}
					class="mt-2 min-h-11 w-full rounded-control border-line bg-surface-raised text-sm text-ink"
				/>
			</label>
			<label class="text-xs font-extrabold tracking-wider text-ink-muted uppercase">
				Email
				<input
					required
					type="email"
					bind:value={email}
					class="mt-2 min-h-11 w-full rounded-control border-line bg-surface-raised text-sm text-ink"
				/>
			</label>
			<label class="text-xs font-extrabold tracking-wider text-ink-muted uppercase">
				Role
				<select
					bind:value={role}
					class="mt-2 min-h-11 w-full rounded-control border-line bg-surface-raised text-sm text-ink"
				>
					<option value="guarantor">Guarantor</option>
					<option value="co_applicant">Co-applicant</option>
					<option value="other">Other participant</option>
				</select>
			</label>
			<fieldset class="sm:col-span-2">
				<legend class="text-xs font-extrabold tracking-wider text-ink-muted uppercase"
					>Questions to assign</legend
				>
				<div
					class="mt-2 grid max-h-48 gap-1 overflow-y-auto rounded-xl border border-line bg-surface-raised p-2 sm:grid-cols-2"
				>
					{#each fields as field (field.id)}
						<label
							class="flex min-h-11 cursor-pointer items-center gap-3 rounded-lg px-3 text-sm font-semibold text-ink hover:bg-brand-soft"
						>
							<input
								type="checkbox"
								checked={selectedFieldIds.includes(field.id)}
								onchange={(event) => {
									selectedFieldIds = event.currentTarget.checked
										? [...selectedFieldIds, field.id]
										: selectedFieldIds.filter((id) => id !== field.id);
								}}
								class="rounded border-line text-brand"
							/>
							<span class="min-w-0">
								<span class="block truncate">{field.label}</span>
								<span class="block text-[10px] text-ink-muted"
									>Page {field.page_number} · {field.kind}</span
								>
							</span>
						</label>
					{/each}
				</div>
			</fieldset>
			<div class="flex items-end justify-end gap-2">
				<Button type="button" variant="ghost" onclick={() => (showForm = false)}>Cancel</Button>
				<Button type="submit" {loading} disabled={selectedFieldIds.length === 0}
					><Mail size={16} /> Send invite</Button
				>
			</div>
		</form>
	{/if}

	{#if invitation}
		<div class="mt-5 rounded-2xl border border-positive/25 bg-positive/8 p-4">
			<p class="flex items-center gap-2 text-sm font-extrabold text-positive">
				<Check size={16} /> Invitation created
			</p>
			<p class="mt-1 text-xs leading-5 text-ink-muted">
				{invitation.invitation_sent
					? 'A verification code and secure link were emailed.'
					: 'Email delivery is not configured. Copy the unique link after configuring delivery.'}
			</p>
			<div class="mt-3 flex gap-2">
				<input
					readonly
					value={invitation.share_url}
					class="min-h-11 min-w-0 flex-1 rounded-control border-line bg-surface-raised text-xs text-ink"
				/>
				<Button variant="secondary" onclick={copyLink}>
					{#if copied}<Check size={16} /> Copied{:else}<Copy size={16} /> Copy{/if}
				</Button>
			</div>
		</div>
	{/if}

	{#if participants.length > 0}
		<div class="mt-5 divide-y divide-line rounded-xl border border-line">
			{#each participants as participant (participant.id)}
				<div class="flex items-center gap-3 p-4">
					<div
						class="grid size-10 shrink-0 place-items-center rounded-xl bg-brand-soft text-sm font-extrabold text-brand-strong"
					>
						{participant.display_name.slice(0, 1).toUpperCase()}
					</div>
					<div class="min-w-0 flex-1">
						<p class="truncate text-sm font-extrabold text-ink">{participant.display_name}</p>
						<p class="truncate text-xs text-ink-muted">
							{participant.contact_hint} · {participant.status.replaceAll('_', ' ')}
						</p>
					</div>
					{#if participant.status !== 'revoked' && participant.status !== 'completed'}
						<Button
							variant="ghost"
							onclick={() => revoke(participant)}
							aria-label={`Revoke ${participant.display_name}`}
						>
							<Trash2 size={16} />
						</Button>
					{/if}
				</div>
			{/each}
		</div>

		<div class="mt-6">
			<h3 class="text-sm font-extrabold text-ink">Assign questions</h3>
			<p class="mt-1 text-xs text-ink-muted">
				Confirm exactly which questions each person can access.
			</p>
			<div class="mt-3 max-h-72 divide-y divide-line overflow-y-auto rounded-xl border border-line">
				{#each fields as field (field.id)}
					<label class="flex items-center gap-3 p-3">
						<span class="min-w-0 flex-1 truncate text-sm font-semibold text-ink">{field.label}</span
						>
						<select
							value={field.participant_id ?? ''}
							onchange={(event) => assign(field.id, event.currentTarget.value || null)}
							disabled={loading}
							class="min-h-10 max-w-40 rounded-lg border-line bg-canvas text-xs text-ink"
							aria-label={`Assign ${field.label}`}
						>
							<option value="">Me</option>
							{#each participants.filter((participant) => participant.status !== 'revoked' && (participant.status !== 'completed' || participant.id === field.participant_id)) as participant (participant.id)}
								<option value={participant.id}>{participant.display_name}</option>
							{/each}
						</select>
					</label>
				{/each}
			</div>
		</div>
	{:else if !showForm}
		<div
			class="mt-5 rounded-xl border border-dashed border-line bg-canvas p-6 text-center text-sm text-ink-muted"
		>
			No participants have been invited.
		</div>
	{/if}
</section>
