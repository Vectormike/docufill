<script lang="ts">
	import { CheckCircle2, FileCheck2, LockKeyhole, ShieldCheck } from '@lucide/svelte';
	import { resolve } from '$app/paths';
	import { page } from '$app/state';
	import { onMount } from 'svelte';
	import {
		api,
		type DocumentField,
		type ParticipantAssignment,
		type ParticipantLanding,
		type ParticipantReceipt
	} from '$lib/api/client';
	import BrandMark from '$lib/components/BrandMark.svelte';
	import Button from '$lib/components/Button.svelte';
	import QuestionCard from '$lib/components/QuestionCard.svelte';
	import SignatureCapture from '$lib/components/SignatureCapture.svelte';

	type Step = 'landing' | 'verify' | 'questions' | 'signature' | 'review' | 'receipt';

	const token = page.params.token ?? '';
	const sessionKey = `docufill-participant:${token}`;
	let landing = $state<ParticipantLanding | null>(null);
	let assignment = $state<ParticipantAssignment | null>(null);
	let receipt = $state<ParticipantReceipt | null>(null);
	let sessionToken = $state('');
	let code = $state('');
	let step = $state<Step>('landing');
	let current = $state(0);
	let loading = $state(true);
	let working = $state(false);
	let error = $state('');
	let codeSent = $state(false);

	const questions = $derived(
		(assignment?.fields ?? []).filter((field) => field.kind !== 'signature')
	);
	const signatureField = $derived(
		assignment?.fields.find((field) => field.kind === 'signature') ?? null
	);
	const completedCount = $derived(
		(assignment?.fields ?? []).filter((field) => field.confirmed_at).length
	);

	onMount(() => {
		void loadLanding();
	});

	async function loadLanding() {
		try {
			landing = await api.shareLanding(token);
			sessionToken = sessionStorage.getItem(sessionKey) ?? '';
			if (landing?.status === 'completed') {
				step = 'receipt';
			} else if (sessionToken) {
				await loadAssignment();
			}
		} catch (cause) {
			error = cause instanceof Error ? cause.message : 'This invitation is unavailable.';
		} finally {
			loading = false;
		}
	}

	async function verify() {
		if (!/^\d{6}$/.test(code)) return;
		working = true;
		error = '';
		try {
			const session = await api.verifyShare(token, code);
			sessionToken = session.session_token;
			sessionStorage.setItem(sessionKey, sessionToken);
			await loadAssignment();
		} catch (cause) {
			error = cause instanceof Error ? cause.message : 'The verification code was not accepted.';
		} finally {
			working = false;
		}
	}

	async function requestCode() {
		working = true;
		error = '';
		try {
			await api.requestShareVerificationCode(token);
			codeSent = true;
		} catch (cause) {
			error = cause instanceof Error ? cause.message : 'A new code could not be sent.';
		} finally {
			working = false;
		}
	}

	async function loadAssignment() {
		try {
			assignment = await api.assignment(token, sessionToken);
			current = Math.max(
				0,
				questions.findIndex((field) => !field.confirmed_at)
			);
			step = questions.length ? 'questions' : signatureField ? 'signature' : 'review';
		} catch (cause) {
			sessionStorage.removeItem(sessionKey);
			sessionToken = '';
			step = 'verify';
			error = cause instanceof Error ? cause.message : 'Please verify your identity again.';
		}
	}

	async function saveAnswer(value: string) {
		const field = questions[current];
		if (!field || !assignment) return;
		const updated = await api.answerAssignedField(token, field.id, value, sessionToken);
		replaceField(updated);
	}

	async function saveSignature(_kind: string, imageDataUrl: string) {
		if (!signatureField) return;
		working = true;
		error = '';
		try {
			const updated = await api.answerAssignedField(
				token,
				signatureField.id,
				imageDataUrl,
				sessionToken
			);
			replaceField(updated);
			step = 'review';
		} catch (cause) {
			error = cause instanceof Error ? cause.message : 'The signature could not be saved.';
		} finally {
			working = false;
		}
	}

	function replaceField(updated: DocumentField) {
		if (!assignment) return;
		assignment = {
			...assignment,
			fields: assignment.fields.map((field) => (field.id === updated.id ? updated : field))
		};
	}

	function nextQuestion() {
		if (current < questions.length - 1) current += 1;
		else step = signatureField && !signatureField.confirmed_at ? 'signature' : 'review';
	}

	async function submit() {
		working = true;
		error = '';
		try {
			receipt = await api.submitAssignment(token, sessionToken);
			sessionStorage.removeItem(sessionKey);
			step = 'receipt';
		} catch (cause) {
			error = cause instanceof Error ? cause.message : 'Your answers could not be submitted.';
		} finally {
			working = false;
		}
	}
</script>

<svelte:head>
	<title>{landing?.document_subject ?? 'Secure document request'} — Docufill</title>
	<meta name="robots" content="noindex, nofollow" />
</svelte:head>

<header class="border-b border-line bg-surface">
	<div class="mx-auto flex min-h-18 max-w-3xl items-center justify-between px-5">
		<BrandMark size="sm" />
		<span class="inline-flex items-center gap-1.5 text-xs font-bold text-positive"
			><LockKeyhole size={14} /> Secure request</span
		>
	</div>
</header>

<main class="mx-auto max-w-3xl px-5 py-8 sm:py-12">
	{#if loading}
		<div class="surface h-80 p-6"><div class="skeleton h-full rounded-xl"></div></div>
	{:else if error && !landing}
		<section class="surface p-8 text-center">
			<LockKeyhole size={30} class="mx-auto text-negative" />
			<h1 class="mt-4 text-2xl font-extrabold text-ink">Invitation unavailable</h1>
			<p class="mt-2 text-sm text-negative" role="alert">{error}</p>
		</section>
	{:else if landing}
		<div class="mb-7">
			<p class="eyebrow mb-2">Private participant workspace</p>
			<h1 class="text-3xl font-extrabold tracking-[-0.045em] text-balance text-ink">
				{landing.document_subject}
			</h1>
			<p class="mt-2 text-sm text-ink-muted">
				{landing.requester_name} asked you to complete the
				<strong class="text-ink">{landing.role}</strong> section.
			</p>
		</div>

		{#if error}
			<p
				class="mb-5 rounded-xl bg-negative/10 p-4 text-sm font-semibold text-negative"
				role="alert"
			>
				{error}
			</p>
		{/if}

		{#if step === 'landing'}
			<section class="surface overflow-hidden p-6 sm:p-8">
				<div class="grid size-12 place-items-center rounded-xl bg-brand-soft text-brand-strong">
					<FileCheck2 size={24} />
				</div>
				<h2 class="mt-5 text-2xl font-extrabold text-ink">Hi {landing.participant_name}</h2>
				<p class="mt-2 text-sm leading-6 text-ink-muted">
					You will only see the questions assigned to you—not the PDF or anyone else's information.
				</p>
				<ul class="mt-6 grid gap-3 text-sm font-semibold text-ink-muted sm:grid-cols-2">
					<li class="rounded-xl border border-line bg-canvas p-4">Email verification required</li>
					<li class="rounded-xl border border-line bg-canvas p-4">
						Link and session expire automatically
					</li>
				</ul>
				<Button class="mt-6 w-full sm:w-auto" onclick={() => (step = 'verify')}
					>Verify and continue</Button
				>
			</section>
		{:else if step === 'verify'}
			<section class="surface p-6 sm:p-8">
				<h2 class="text-2xl font-extrabold text-ink">Check your email</h2>
				<p class="mt-2 text-sm leading-6 text-ink-muted">
					Enter the six-digit code sent with your private invitation.
				</p>
				<form
					class="mt-6"
					onsubmit={(event) => {
						event.preventDefault();
						void verify();
					}}
				>
					<label
						for="verification-code"
						class="text-xs font-extrabold tracking-wider text-ink-muted uppercase"
						>Verification code</label
					>
					<input
						id="verification-code"
						bind:value={code}
						inputmode="numeric"
						autocomplete="one-time-code"
						pattern="[0-9]{6}"
						maxlength="6"
						class="mt-2 min-h-14 w-full rounded-control border-line bg-canvas text-center text-2xl font-extrabold tracking-[0.35em] text-ink"
						placeholder="000000"
						required
					/>
					<Button
						type="submit"
						class="mt-4 w-full"
						loading={working}
						disabled={!/^\d{6}$/.test(code)}>Continue securely</Button
					>
				</form>
				<div class="mt-4 text-center">
					{#if codeSent}
						<p class="text-xs font-bold text-positive" role="status">
							A new code was sent. It expires in 15 minutes.
						</p>
					{:else}
						<button
							class="min-h-11 text-xs font-extrabold text-brand-strong"
							onclick={requestCode}
							disabled={working}
						>
							Send me a new code
						</button>
					{/if}
				</div>
			</section>
		{:else if step === 'questions' && questions[current]}
			<QuestionCard
				field={questions[current]}
				index={current}
				total={questions.length}
				onsave={saveAnswer}
				onreject={async () => {}}
				onback={() => (current = Math.max(0, current - 1))}
				onnext={nextQuestion}
			/>
		{:else if step === 'signature' && signatureField}
			<section class="surface p-6 sm:p-8">
				<p class="eyebrow mb-2">Assigned signature</p>
				<h2 class="text-2xl font-extrabold text-ink">{signatureField.label}</h2>
				<p class="mt-2 text-sm leading-6 text-ink-muted">
					Create a signature for this request. It is isolated from the requester's signature and is
					not saved to a reusable account.
				</p>
				<div class="mt-6"><SignatureCapture onsave={saveSignature} /></div>
			</section>
		{:else if step === 'review' && assignment}
			<section class="surface overflow-hidden">
				<div class="border-b border-line p-6 sm:p-8">
					<p class="eyebrow mb-2">Review your section</p>
					<h2 class="text-2xl font-extrabold text-ink">
						{completedCount} of {assignment.fields.length} answers ready
					</h2>
				</div>
				<div class="divide-y divide-line">
					{#each assignment.fields as field (field.id)}
						<div class="flex items-start justify-between gap-4 p-5 sm:px-8">
							<div>
								<p class="text-sm font-extrabold text-ink">{field.label}</p>
								<p class="mt-1 text-sm text-ink-muted">{field.value_preview ?? 'Missing'}</p>
							</div>
							<button
								class="min-h-11 px-2 text-xs font-extrabold text-brand-strong"
								onclick={() => {
									if (field.kind === 'signature') step = 'signature';
									else {
										current = questions.findIndex((item) => item.id === field.id);
										step = 'questions';
									}
								}}>Edit</button
							>
						</div>
					{/each}
				</div>
				<div class="border-t border-line bg-canvas/60 p-6 sm:p-8">
					<p class="flex items-start gap-2 text-xs leading-5 text-ink-muted">
						<ShieldCheck size={16} class="mt-0.5 shrink-0 text-positive" /> Submitting locks your section
						and notifies the requester. You cannot see or finalize the full document.
					</p>
					<Button
						class="mt-5 w-full"
						onclick={submit}
						loading={working}
						disabled={completedCount !== assignment.fields.length}>Submit my section</Button
					>
				</div>
			</section>
		{:else if step === 'receipt'}
			<section class="surface p-8 text-center sm:p-10">
				<div
					class="mx-auto grid size-16 place-items-center rounded-2xl bg-positive/10 text-positive"
				>
					<CheckCircle2 size={30} />
				</div>
				<h2 class="mt-5 text-3xl font-extrabold tracking-tight text-ink">
					Your section is submitted
				</h2>
				<p class="mt-2 text-sm text-ink-muted">
					The requester can now continue their document workflow.
				</p>
				{#if receipt}
					<p
						class="mx-auto mt-6 max-w-sm rounded-xl border border-line bg-canvas p-4 font-mono text-xs text-ink-muted"
					>
						Receipt {receipt.receipt_code}<br />{new Date(receipt.submitted_at).toLocaleString(
							'en-NG'
						)}
					</p>
				{/if}
				<a
					href={resolve('/')}
					class="mt-7 inline-flex min-h-11 items-center rounded-control bg-brand px-5 text-sm font-extrabold text-[#211d14]"
				>
					Create your own Docufill account
				</a>
			</section>
		{/if}
	{/if}
</main>
