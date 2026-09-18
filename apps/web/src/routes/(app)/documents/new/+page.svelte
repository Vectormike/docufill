<script lang="ts">
	import { ArrowLeft, FileText, LockKeyhole, Sparkles, UploadCloud, X } from '@lucide/svelte';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { api } from '$lib/api/client';
	import Button from '$lib/components/Button.svelte';
	import { subjectFromFilename, uploadDocument, validatePdf } from '$lib/documents';

	let file = $state<File | null>(null);
	let subject = $state('');
	let naming = $state(false);
	let context = $state('');
	let acknowledged = $state(false);
	let dragging = $state(false);
	let uploading = $state(false);
	let progress = $state(0);
	let stage = $state('');
	let error = $state('');
	let namingVersion = 0;

	function choose(candidate?: File) {
		if (!candidate) return;
		error = '';
		try {
			validatePdf(candidate);
			file = candidate;
			void typeSubject(subjectFromFilename(candidate.name));
		} catch (cause) {
			error = cause instanceof Error ? cause.message : 'Choose a valid PDF.';
		}
	}

	async function typeSubject(nextSubject: string) {
		const version = ++namingVersion;
		subject = '';
		if (window.matchMedia('(prefers-reduced-motion: reduce)').matches) {
			subject = nextSubject;
			return;
		}
		naming = true;
		for (const character of nextSubject) {
			await new Promise((resolveDelay) => window.setTimeout(resolveDelay, 24));
			if (version !== namingVersion) return;
			subject += character;
		}
		naming = false;
	}

	function removeFile() {
		namingVersion += 1;
		naming = false;
		subject = '';
		file = null;
	}

	function drop(event: DragEvent) {
		event.preventDefault();
		dragging = false;
		choose(event.dataTransfer?.files[0]);
	}

	async function submit(event: SubmitEvent) {
		event.preventDefault();
		if (!file || !acknowledged) return;
		error = '';
		uploading = true;
		try {
			const document = await uploadDocument(file, (nextProgress, nextStage) => {
				progress = nextProgress;
				stage = nextStage;
			});
			if (context.trim()) await api.addContext(document.id, context.trim());
			await goto(resolve('/(app)/documents/[documentId]', { documentId: document.id }));
		} catch (cause) {
			error = cause instanceof Error ? cause.message : 'The document could not be uploaded.';
			uploading = false;
		}
	}
</script>

<svelte:head><title>New document — Docufill</title></svelte:head>

<main class="mx-auto max-w-3xl px-5 py-8 sm:px-8 sm:py-12">
	<a
		href={resolve('/documents')}
		class="inline-flex min-h-11 items-center gap-2 text-sm font-bold text-ink-muted hover:text-ink"
	>
		<ArrowLeft size={17} /> Documents
	</a>
	<div class="mt-5">
		<h1 class="ask text-2xl leading-8 text-ink">New document</h1>
		<p class="mt-1 text-sm leading-6 text-ink-muted">
			Upload a fillable or text-based PDF. Your original remains unchanged in private storage.
		</p>
	</div>

	<form class="mt-8 space-y-5" onsubmit={submit}>
		<section class="surface p-5 sm:p-7">
			<label for="subject" class="flex items-center gap-2 text-sm font-extrabold text-ink">
				<Sparkles size={17} class="text-brand-strong" /> Document name
			</label>
			<p class="mt-1 text-xs text-ink-muted">Choose a PDF and Docufill will name it for you.</p>
			<input
				id="subject"
				readonly
				value={subject}
				placeholder="Waiting for a PDF…"
				class="mt-3 min-h-12 w-full rounded-control border-line bg-canvas text-ink placeholder:text-ink-muted/60"
			/>
			{#if naming}<p class="mt-2 text-xs font-semibold text-brand-strong">Naming document…</p>{/if}

			<label
				class="mt-6 grid min-h-64 cursor-pointer place-items-center rounded-xl border-2 border-dashed p-6 text-center transition {dragging
					? 'border-brand bg-brand-soft'
					: 'border-line bg-canvas hover:border-brand'}"
				ondragover={(event) => {
					event.preventDefault();
					dragging = true;
				}}
				ondragleave={() => (dragging = false)}
				ondrop={drop}
			>
				<input
					type="file"
					accept="application/pdf,.pdf"
					class="sr-only"
					onchange={(event) => choose(event.currentTarget.files?.[0])}
				/>
				{#if file}
					<div>
						<div class="mx-auto grid size-14 place-items-center rounded-xl bg-brand text-[#211d14]">
							<FileText size={25} />
						</div>
						<p class="mt-4 font-extrabold text-ink">{file.name}</p>
						<p class="mt-1 text-xs text-ink-muted">
							{(file.size / 1024 / 1024).toFixed(2)} MB · PDF
						</p>
						<button
							type="button"
							onclick={(event) => {
								event.preventDefault();
								removeFile();
							}}
							class="mt-4 inline-flex min-h-11 items-center gap-2 rounded-control px-3 text-sm font-bold text-negative"
						>
							<X size={16} /> Remove
						</button>
					</div>
				{:else}
					<div>
						<div
							class="mx-auto grid size-14 place-items-center rounded-xl bg-brand-soft text-brand-strong"
						>
							<UploadCloud size={26} />
						</div>
						<p class="mt-4 font-extrabold text-ink">Drop your PDF here</p>
						<p class="mt-1 text-sm text-ink-muted">or tap to choose a file · maximum 25 MB</p>
						<p class="mt-3 text-xs font-semibold text-warning">
							Scans and image-only PDFs are not supported yet.
						</p>
					</div>
				{/if}
			</label>
		</section>

		<section class="surface p-5 sm:p-7">
			<label for="context" class="text-sm font-extrabold text-ink"
				>Helpful context <span class="font-medium text-ink-muted">(optional)</span></label
			>
			<p class="mt-1 text-xs leading-5 text-ink-muted">
				Paste only text relevant to this document. Direct Gmail access is not part of this release.
			</p>
			<textarea
				id="context"
				rows="4"
				maxlength="50000"
				bind:value={context}
				placeholder="Paste instructions or selected email context…"
				class="mt-3 w-full rounded-control border-line bg-canvas text-sm text-ink placeholder:text-ink-muted/60"
			></textarea>
		</section>

		<label class="surface flex cursor-pointer items-start gap-3 p-5">
			<input
				type="checkbox"
				required
				bind:checked={acknowledged}
				class="mt-1 rounded border-line text-brand focus:ring-brand"
			/>
			<span>
				<span class="block text-sm font-extrabold text-ink"
					>This is a supported, non-regulated document</span
				>
				<span class="mt-1 block text-xs leading-5 text-ink-muted">
					Do not upload wills, certificates, court or family-law documents, or anything requiring a
					regulated digital signature.
				</span>
			</span>
		</label>

		{#if uploading}
			<div class="surface p-5" role="status" aria-live="polite">
				<div class="flex items-center justify-between text-sm font-bold text-ink">
					<span>{stage}</span><span>{progress}%</span>
				</div>
				<div class="mt-3 h-2 overflow-hidden rounded-full bg-line">
					<div
						class="h-full rounded-full bg-ink transition-all duration-300"
						style={`width: ${progress}%`}
					></div>
				</div>
			</div>
		{/if}
		{#if error}<p
				class="rounded-xl bg-negative/10 p-4 text-sm font-semibold text-negative"
				role="alert"
			>
				{error}
			</p>{/if}

		<div class="flex flex-col-reverse justify-between gap-3 sm:flex-row sm:items-center">
			<p class="flex items-center gap-2 text-xs font-semibold text-positive">
				<LockKeyhole size={15} /> Direct upload to private storage
			</p>
			<Button
				type="submit"
				loading={uploading}
				disabled={!file || !acknowledged}
				class="sm:min-w-44"
			>
				Upload and analyse
			</Button>
		</div>
	</form>
</main>
