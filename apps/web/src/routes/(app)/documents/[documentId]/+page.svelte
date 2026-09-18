<script lang="ts">
	import {
		ArrowLeft,
		CheckCircle2,
		Download,
		FileWarning,
		PenLine,
		Settings2,
		ShieldCheck,
		Trash2
	} from '@lucide/svelte';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { page } from '$app/state';
	import { onMount } from 'svelte';
	import { api, type DocumentDetail, type ProfileVault } from '$lib/api/client';
	import AnswerReview from '$lib/components/AnswerReview.svelte';
	import Button from '$lib/components/Button.svelte';
	import CopilotSummary from '$lib/components/CopilotSummary.svelte';
	import ParticipantManager from '$lib/components/ParticipantManager.svelte';
	import PdfAdjuster from '$lib/components/PdfAdjuster.svelte';
	import QuestionLine from '$lib/components/QuestionLine.svelte';

	type Mode = 'summary' | 'questions' | 'review' | 'preview' | 'adjust';

	const documentId = page.params.documentId ?? '';
	let detail = $state<DocumentDetail | null>(null);
	let vault = $state<ProfileVault | null>(null);
	let loading = $state(true);
	let error = $state('');
	let mode = $state<Mode>('summary');
	let activeFieldId = $state('');
	let previewUrl = $state('');
	let previewing = $state(false);
	let completing = $state(false);
	let deletingDocument = $state(false);
	let consent = $state(false);
	let memoryConsent = $state(false);
	let memorySaving = $state(false);
	let approvedMemoryFieldIds = $state<string[]>([]);

	/**
	 * Document order, not answered-first: the list is meant to read as the form
	 * itself. Unanswered asks are highlighted instead of hoisted.
	 */
	const questions = $derived(
		(detail?.fields ?? [])
			.filter((field) => field.kind !== 'signature')
			.sort((left, right) => left.sort_order - right.sort_order)
	);
	const answerable = $derived(questions.filter((field) => !field.participant_id));
	const remaining = $derived(answerable.filter((field) => !field.confirmed_at).length);
	const signatureRequired = $derived(
		Boolean(detail?.fields.some((field) => !field.participant_id && field.kind === 'signature'))
	);
	const memoryCandidates = $derived(
		(detail?.fields ?? []).filter(
			(field) =>
				!field.participant_id &&
				field.kind !== 'signature' &&
				field.kind !== 'declaration' &&
				Boolean(field.confirmed_at) &&
				!isSensitiveMemoryLabel(field.label)
		)
	);

	onMount(() => {
		void initialise();
	});

	async function initialise() {
		try {
			await Promise.all([load(), loadProfile()]);
			if (detail?.status === 'processing' || detail?.status === 'uploaded') {
				await waitFor((status) => !['processing', 'uploaded'].includes(status));
			}
		} catch (cause) {
			error = cause instanceof Error ? cause.message : 'The document could not be loaded.';
		}
	}

	async function load(showLoader = true) {
		if (showLoader) loading = true;
		error = '';
		try {
			detail = await api.document(documentId);
			memoryConsent = detail.memory_consent;
			if (memoryConsent && approvedMemoryFieldIds.length === 0) {
				approvedMemoryFieldIds = memoryCandidates.map((field) => field.id);
			}
		} catch (cause) {
			error = cause instanceof Error ? cause.message : 'The document could not be loaded.';
		} finally {
			loading = false;
		}
	}

	async function loadProfile() {
		vault = await api.profile().catch(() => null);
	}

	async function waitFor(done: (status: string) => boolean) {
		for (let attempt = 0; attempt < 60; attempt += 1) {
			await new Promise((resolve) => setTimeout(resolve, 1200));
			await load(false);
			if (!detail || done(detail.status)) return;
		}
		throw new Error('Processing is taking longer than expected. You can safely return later.');
	}

	function startQuestions() {
		activeFieldId = nextGapId() ?? answerable[0]?.id ?? '';
		mode = 'questions';
	}

	function editField(fieldId: string) {
		activeFieldId = fieldId;
		mode = 'questions';
	}

	function nextGapId(after = activeFieldId) {
		const start = answerable.findIndex((field) => field.id === after);
		const ordered = [...answerable.slice(start + 1), ...answerable.slice(0, start + 1)];
		return ordered.find((field) => !field.confirmed_at)?.id;
	}

	function advance() {
		const next = nextGapId();
		if (next) activeFieldId = next;
		else mode = 'review';
	}

	async function saveAnswer(fieldId: string, value: string) {
		if (!detail) return;
		const updated = await api.updateField(documentId, fieldId, {
			value,
			source: 'user',
			confirmed: true
		});
		detail = {
			...detail,
			fields: detail.fields.map((item) => (item.id === fieldId ? updated : item))
		};
	}

	async function rejectDraft(fieldId: string) {
		if (!detail) return;
		await api.clearField(documentId, fieldId);
		detail = {
			...detail,
			fields: detail.fields.map((item) =>
				item.id === fieldId
					? { ...item, value: null, value_preview: null, source: 'missing', confirmed_at: null }
					: item
			)
		};
	}

	async function generatePreview() {
		previewing = true;
		error = '';
		try {
			await api.requestPreview(documentId);
			await waitFor((status) => status === 'ready' || status === 'failed');
			if (detail?.status === 'failed') throw new Error('The PDF preview could not be rendered.');
			previewUrl = (await api.preview(documentId)).url;
			mode = 'preview';
		} catch (cause) {
			error = cause instanceof Error ? cause.message : 'The preview could not be generated.';
		} finally {
			previewing = false;
		}
	}

	async function saveLayout(
		fieldId: string,
		layout: {
			x: number;
			y: number;
			width: number;
			height: number;
			font_size: number | null;
			alignment: string;
		}
	) {
		await api.updateFieldLayout(documentId, fieldId, layout);
		if (detail) {
			detail = {
				...detail,
				fields: detail.fields.map((field) =>
					field.id === fieldId ? { ...field, ...layout } : field
				)
			};
		}
		previewUrl = '';
		mode = 'review';
	}

	async function complete() {
		if (!detail || !consent) return;
		if (signatureRequired && !vault?.signature) {
			error = 'Create a saved signature before signing this document.';
			return;
		}
		completing = true;
		error = '';
		try {
			await api.complete(documentId, {
				apply_signature_id: signatureRequired ? (vault?.signature?.id ?? null) : null,
				consent_text: 'I reviewed this completed PDF and intend to complete and sign it.'
			});
			await waitFor((status) => status === 'completed' || status === 'failed');
			if (detail?.status === 'failed') throw new Error('The final PDF could not be generated.');
		} catch (cause) {
			error = cause instanceof Error ? cause.message : 'The document could not be completed.';
		} finally {
			completing = false;
		}
	}

	async function download() {
		const { url } = await api.download(documentId);
		window.location.assign(url);
	}

	function openPreview() {
		if (previewUrl) window.open(previewUrl, '_blank', 'noopener');
	}

	async function deleteDocument() {
		if (
			!confirm('Permanently delete this document, its answers, participants, and generated files?')
		) {
			return;
		}
		deletingDocument = true;
		error = '';
		try {
			await api.deleteDocument(documentId);
			await goto(resolve('/documents'), { replaceState: true });
		} catch (cause) {
			error = cause instanceof Error ? cause.message : 'The document could not be deleted.';
			deletingDocument = false;
		}
	}

	function toggleMemoryCandidate(fieldId: string) {
		approvedMemoryFieldIds = approvedMemoryFieldIds.includes(fieldId)
			? approvedMemoryFieldIds.filter((id) => id !== fieldId)
			: [...approvedMemoryFieldIds, fieldId];
	}

	async function toggleMemory() {
		const enabled = !memoryConsent;
		memorySaving = true;
		error = '';
		try {
			await api.setMemory(documentId, enabled, enabled ? approvedMemoryFieldIds : []);
			memoryConsent = enabled;
		} catch (cause) {
			error = cause instanceof Error ? cause.message : 'Memory preference could not be saved.';
		} finally {
			memorySaving = false;
		}
	}

	function isSensitiveMemoryLabel(label: string) {
		const normalized = label.toLowerCase();
		return [
			'signature',
			'password',
			'passport',
			'nin',
			'bvn',
			'account number',
			'bank account',
			'tax id'
		].some((term) => normalized.includes(term));
	}

	function updateAssignment(fieldId: string, participantId: string | null) {
		if (!detail) return;
		detail = {
			...detail,
			fields: detail.fields.map((field) =>
				field.id === fieldId ? { ...field, participant_id: participantId } : field
			)
		};
	}
</script>

<svelte:head><title>{detail?.subject ?? 'Document'} — Docufill</title></svelte:head>

<main class="mx-auto max-w-6xl px-5 py-8 sm:px-8 sm:py-12">
	<a
		href={resolve('/documents')}
		class="inline-flex min-h-11 items-center gap-2 text-sm font-bold text-ink-muted hover:text-ink"
	>
		<ArrowLeft size={17} /> Documents
	</a>

	{#if loading}
		<div class="surface mt-5 h-80 p-6"><div class="skeleton h-full rounded-xl"></div></div>
	{:else if error && !detail}
		<div class="surface mt-5 p-8 text-center">
			<FileWarning size={30} class="mx-auto text-negative" />
			<p class="mt-4 font-bold text-negative" role="alert">{error}</p>
			<Button class="mt-5" onclick={() => load()}>Try again</Button>
		</div>
	{:else if detail}
		<header class="mt-5 flex flex-col justify-between gap-4 sm:flex-row sm:items-end">
			<div>
				<h1 class="ask text-xl leading-8 text-balance text-ink sm:text-2xl">
					{detail.subject}
				</h1>
				<p class="mt-1 text-xs text-ink-muted">{detail.original_name}</p>
			</div>
			<div class="flex items-center gap-2">
				<span
					class="w-fit rounded-control border border-line px-2.5 py-1 text-xs font-medium text-ink-muted"
				>
					{detail.status.replaceAll('_', ' ')}
				</span>
				<Button
					variant="ghost"
					onclick={deleteDocument}
					loading={deletingDocument}
					aria-label="Delete document"
					title="Delete document"
				>
					<Trash2 size={16} />
				</Button>
			</div>
		</header>

		{#if error}
			<p
				class="mt-5 rounded-xl bg-negative/10 p-4 text-sm font-semibold text-negative"
				role="alert"
			>
				{error}
			</p>
		{/if}

		{#if detail.status === 'processing' || detail.status === 'uploaded'}
			<section class="surface mt-8 overflow-hidden p-7 text-center sm:p-10">
				<div
					class="mx-auto size-12 animate-spin rounded-full border-4 border-brand border-r-transparent"
				></div>
				<h2 class="mt-6 text-2xl font-extrabold text-ink">Understanding your document</h2>
				<p class="mt-2 text-sm text-ink-muted">
					Extracting fields, grounding known answers, and checking layout.
				</p>
				<div class="mx-auto mt-6 h-2 max-w-md overflow-hidden rounded-full bg-line">
					<div
						class="h-full rounded-full bg-ink transition-all"
						style={`width:${detail.progress}%`}
					></div>
				</div>
				<p class="mt-3 text-xs font-bold text-ink-muted">{detail.progress}%</p>
			</section>
		{:else if detail.status === 'failed'}
			<section class="surface mt-8 p-8 text-center">
				<FileWarning size={32} class="mx-auto text-negative" />
				<h2 class="mt-4 text-2xl font-extrabold text-ink">This PDF needs attention</h2>
				<p class="mx-auto mt-2 max-w-lg text-sm leading-6 text-ink-muted">
					It may be encrypted, image-only, malformed, oversized, or require a PDF feature not
					supported in this release.
				</p>
				<div class="mt-6 flex flex-wrap justify-center gap-3">
					{#if detail.fields.length}
						<Button onclick={generatePreview} loading={previewing}>Retry PDF preview</Button>
					{/if}
					<a
						href={resolve('/documents/new')}
						class="inline-flex min-h-11 items-center rounded-control bg-ink px-5 text-sm font-semibold text-canvas"
					>
						Upload another PDF
					</a>
				</div>
			</section>
		{:else if detail.status === 'completed'}
			<section class="surface mt-8 p-7 text-center sm:p-10">
				<div
					class="mx-auto grid size-16 place-items-center rounded-xl bg-positive/10 text-positive"
				>
					<CheckCircle2 size={30} />
				</div>
				<h2 class="ask mt-5 text-xl leading-8 text-ink">Document completed</h2>
				<p class="mt-2 text-sm text-ink-muted">
					Your immutable original is preserved. This download link expires after five minutes.
				</p>
				<Button class="mt-6" onclick={download}
					><Download size={17} /> Download completed PDF</Button
				>
				<div class="mx-auto mt-7 max-w-md rounded-xl border border-line bg-canvas p-4 text-left">
					<p class="text-sm font-extrabold text-ink">Remember selected details</p>
					<p class="mt-1 text-xs leading-5 text-ink-muted">
						Only the confirmed, non-sensitive answers you approve below are added to your vault and
						can help with future documents. Participant answers stay isolated.
					</p>
					{#if memoryCandidates.length > 0}
						<div class="mt-4 max-h-48 space-y-2 overflow-y-auto">
							{#each memoryCandidates as field (field.id)}
								<label class="flex cursor-pointer items-start gap-2 text-sm text-ink">
									<input
										type="checkbox"
										checked={approvedMemoryFieldIds.includes(field.id)}
										disabled={memoryConsent || memorySaving}
										onchange={() => toggleMemoryCandidate(field.id)}
										class="mt-0.5 rounded border-line text-brand"
									/>
									<span>
										<span class="font-bold">{field.label}</span>
										<span class="block truncate text-xs text-ink-muted">{field.value_preview}</span>
									</span>
								</label>
							{/each}
						</div>
						<Button
							class="mt-4 w-full"
							variant={memoryConsent ? 'secondary' : 'primary'}
							disabled={memorySaving || (!memoryConsent && approvedMemoryFieldIds.length === 0)}
							onclick={toggleMemory}
						>
							{memorySaving
								? 'Saving…'
								: memoryConsent
									? 'Forget this document context'
									: 'Remember selected details'}
						</Button>
					{:else}
						<p class="mt-3 text-xs font-bold text-ink-muted">
							No eligible confirmed answers were found.
						</p>
					{/if}
				</div>
			</section>
		{:else}
			<div class="mt-8">
				{#if mode === 'summary'}
					<CopilotSummary
						document={detail}
						onstart={startQuestions}
						onreview={() => (mode = 'review')}
					/>
				{:else if mode === 'questions'}
					<section class="surface overflow-hidden">
						<div class="border-b border-line px-4 py-5 sm:px-7 sm:py-6">
							<p class="gutter">{questions.length} questions · {remaining} for you</p>
							<h2 class="ask mt-2 text-xl leading-7 text-balance text-ink sm:text-2xl sm:leading-8">
								{detail.subject}
							</h2>
							<div class="mt-4 flex items-center gap-1" aria-hidden="true">
								{#each answerable as field (field.id)}
									<span class="h-[3px] w-3 {field.confirmed_at ? 'bg-ink' : 'bg-line'}"></span>
								{/each}
							</div>
						</div>

						{#each questions as field, index (field.id)}
							<QuestionLine
								{field}
								number={index + 1}
								active={field.id === activeFieldId}
								onactivate={() => (activeFieldId = field.id)}
								onsave={(value) => saveAnswer(field.id, value)}
								onreject={() => rejectDraft(field.id)}
								onnext={advance}
							/>
						{/each}

						<div
							class="flex flex-wrap items-center justify-between gap-3 px-4 py-4 sm:px-7 sm:py-5"
						>
							<p class="gutter">autosaved · encrypted</p>
							<div class="flex items-center gap-4">
								{#if remaining > 0}
									<button
										type="button"
										onclick={() => (activeFieldId = nextGapId() ?? activeFieldId)}
										class="text-xs font-semibold text-ink-muted hover:text-ink"
										>Jump to next gap</button
									>
								{/if}
								<Button onclick={() => (mode = 'review')}>Review answers</Button>
							</div>
						</div>
					</section>
				{:else if mode === 'review'}
					<AnswerReview
						fields={detail.fields}
						onedit={editField}
						onpreview={generatePreview}
						{previewing}
					/>
					<div class="mt-5">
						<ParticipantManager
							{documentId}
							fields={detail.fields}
							onassignment={updateAssignment}
						/>
					</div>
				{:else if (mode === 'preview' || mode === 'adjust') && previewUrl}
					<section class="surface overflow-hidden">
						<div
							class="flex flex-col justify-between gap-4 border-b border-line p-5 sm:flex-row sm:items-center sm:p-6"
						>
							<div>
								<h2 class="text-xl font-extrabold text-ink">Review the actual completed PDF</h2>
								<p class="mt-1 text-xs text-ink-muted">
									Nothing is signed until you confirm below.
								</p>
							</div>
							<div class="flex gap-2">
								<Button variant="ghost" onclick={() => (mode = 'review')}>Back to answers</Button>
								<Button
									variant="secondary"
									onclick={() => (mode = mode === 'adjust' ? 'preview' : 'adjust')}
								>
									<Settings2 size={16} />
									{mode === 'adjust' ? 'Close adjuster' : 'Adjust document'}
								</Button>
							</div>
						</div>
						{#if mode === 'adjust'}
							<div class="p-4 sm:p-6">
								<PdfAdjuster url={previewUrl} fields={detail.fields} onsave={saveLayout} />
							</div>
						{:else}
							<iframe
								src={previewUrl}
								title={`Completed PDF preview for ${detail.subject}`}
								class="h-[65vh] min-h-120 w-full bg-stone-200"
							></iframe>
							<p class="border-t border-line px-5 py-3 text-xs text-ink-muted sm:px-6">
								Preview not showing? <button
									type="button"
									onclick={openPreview}
									class="font-semibold text-brand-strong underline underline-offset-2"
									>Open the PDF in a new tab</button
								>. Some in-app browsers cannot display PDFs.
							</p>
							<div class="border-t border-line p-5 sm:p-6">
								{#if signatureRequired}
									<div class="mb-5 rounded-xl border border-line bg-canvas p-4">
										<p class="flex items-center gap-2 text-sm font-extrabold text-ink">
											<PenLine size={17} /> Signature required
										</p>
										{#if vault?.signature}
											<p class="mt-1 text-xs text-ink-muted">
												Saved signature version {vault.signature.version} will be applied only after confirmation.
											</p>
										{:else}
											<p class="mt-2 text-sm text-negative">
												Create a saved signature in <a
													href={resolve('/profile')}
													class="font-extrabold underline">My details</a
												> first.
											</p>
										{/if}
									</div>
								{/if}
								<label class="flex cursor-pointer items-start gap-3">
									<input
										type="checkbox"
										bind:checked={consent}
										class="mt-1 rounded border-line text-brand"
									/>
									<span>
										<span class="text-sm font-extrabold text-ink"
											>I reviewed this PDF and intend to complete {signatureRequired
												? 'and sign '
												: ''}it.</span
										>
										<span class="mt-1 block text-xs leading-5 text-ink-muted"
											>This records consent, identity method, timestamp, signature version, and the
											final document hash.</span
										>
									</span>
								</label>
								<div class="mt-5 flex flex-col justify-between gap-3 sm:flex-row sm:items-center">
									<p class="flex items-center gap-2 text-xs font-semibold text-positive">
										<ShieldCheck size={15} /> Explicit confirmation is required every time
									</p>
									<Button
										onclick={complete}
										loading={completing}
										disabled={!consent || (signatureRequired && !vault?.signature)}
									>
										{signatureRequired ? 'Confirm and sign' : 'Complete document'}
									</Button>
								</div>
							</div>
						{/if}
					</section>
				{/if}
			</div>
		{/if}
	{/if}
</main>
