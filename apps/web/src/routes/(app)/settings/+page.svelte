<script lang="ts">
	import { Download, ExternalLink, Shield, Trash2 } from '@lucide/svelte';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { api } from '$lib/api/client';
	import { signOut } from '$lib/auth';
	import Button from '$lib/components/Button.svelte';

	let exporting = $state(false);
	let deleting = $state(false);
	let confirmation = $state('');
	let error = $state('');

	async function exportData() {
		exporting = true;
		error = '';
		try {
			const data = await api.exportAccount();
			const blob = new Blob([JSON.stringify(data, null, 2)], { type: 'application/json' });
			const url = URL.createObjectURL(blob);
			const anchor = document.createElement('a');
			anchor.href = url;
			anchor.download = `docufill-export-${new Date().toISOString().slice(0, 10)}.json`;
			anchor.click();
			URL.revokeObjectURL(url);
		} catch (cause) {
			error = cause instanceof Error ? cause.message : 'Your export could not be prepared.';
		} finally {
			exporting = false;
		}
	}

	async function deleteAccount() {
		if (confirmation !== 'DELETE') return;
		deleting = true;
		error = '';
		try {
			await api.deleteAccount();
			await signOut();
			await goto(resolve('/'), { replaceState: true });
		} catch (cause) {
			error =
				cause instanceof Error
					? `${cause.message}. For security, sign in again if your session is not recent.`
					: 'Your account could not be deleted.';
		} finally {
			deleting = false;
		}
	}
</script>

<svelte:head><title>Privacy and settings — Docufill</title></svelte:head>

<main class="mx-auto max-w-4xl px-5 py-8 sm:px-8 sm:py-12">
	<p class="eyebrow mb-2">Control centre</p>
	<h1 class="text-4xl font-extrabold tracking-[-0.05em] text-ink">Privacy and settings</h1>
	<p class="mt-3 max-w-2xl text-sm leading-6 text-ink-muted">
		Access, export, correct, or delete your information. Sensitive document content remains
		online-only.
	</p>

	{#if error}
		<p class="mt-6 rounded-xl bg-negative/10 p-4 text-sm font-semibold text-negative" role="alert">
			{error}
		</p>
	{/if}

	<div class="mt-8 grid gap-5">
		<section class="surface p-6 sm:p-8">
			<div class="flex items-start gap-4">
				<div
					class="grid size-11 shrink-0 place-items-center rounded-xl bg-brand-soft text-brand-strong"
				>
					<Download size={21} />
				</div>
				<div>
					<h2 class="text-xl font-extrabold text-ink">Export your information</h2>
					<p class="mt-2 text-sm leading-6 text-ink-muted">
						Download your profile facts, document metadata, participant records, and audit history
						as JSON. Encrypted files are downloaded separately from each workspace.
					</p>
					<Button variant="secondary" class="mt-5" onclick={exportData} loading={exporting}
						>Prepare export</Button
					>
				</div>
			</div>
		</section>

		<section class="surface p-6 sm:p-8">
			<div class="flex items-start gap-4">
				<div
					class="grid size-11 shrink-0 place-items-center rounded-xl bg-positive/10 text-positive"
				>
					<Shield size={21} />
				</div>
				<div>
					<h2 class="text-xl font-extrabold text-ink">How your information is handled</h2>
					<p class="mt-2 text-sm leading-6 text-ink-muted">
						Docufill uses data only to provide the document workflow you request. AI receives
						minimal relevant context and customer documents are not used to train models.
					</p>
					<div class="mt-4 flex flex-wrap gap-4 text-sm font-extrabold text-brand-strong">
						<a href={resolve('/privacy')} class="inline-flex min-h-11 items-center gap-1.5"
							>Privacy notice <ExternalLink size={14} /></a
						>
						<a href={resolve('/acceptable-use')} class="inline-flex min-h-11 items-center gap-1.5"
							>Excluded documents <ExternalLink size={14} /></a
						>
					</div>
				</div>
			</div>
		</section>

		<section class="rounded-card border border-negative/25 bg-surface p-6 shadow-card sm:p-8">
			<div class="flex items-start gap-4">
				<div
					class="grid size-11 shrink-0 place-items-center rounded-xl bg-negative/10 text-negative"
				>
					<Trash2 size={21} />
				</div>
				<div class="min-w-0 flex-1">
					<h2 class="text-xl font-extrabold text-ink">Delete account and data</h2>
					<p class="mt-2 text-sm leading-6 text-ink-muted">
						Permanently delete your identity, profile vault, signatures, documents, participant
						records, memory, and audit records. This cannot be undone.
					</p>
					<label
						for="delete-confirmation"
						class="mt-5 block text-xs font-extrabold tracking-wider text-ink-muted uppercase"
						>Type DELETE to confirm</label
					>
					<input
						id="delete-confirmation"
						bind:value={confirmation}
						autocomplete="off"
						class="mt-2 min-h-12 w-full max-w-sm rounded-control border-line bg-canvas text-ink"
						placeholder="DELETE"
					/>
					<div class="mt-4">
						<Button
							variant="danger"
							onclick={deleteAccount}
							loading={deleting}
							disabled={confirmation !== 'DELETE'}
						>
							Delete my account
						</Button>
					</div>
				</div>
			</div>
		</section>
	</div>
</main>
