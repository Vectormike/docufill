<script lang="ts">
	import {
		CirclePlus,
		FileClock,
		LockKeyhole,
		PenLine,
		ShieldCheck,
		Trash2,
		UserRound
	} from '@lucide/svelte';
	import { onMount } from 'svelte';
	import { api, type ProfileFact, type ProfileVault } from '$lib/api/client';
	import Button from '$lib/components/Button.svelte';
	import FactEditor from '$lib/components/FactEditor.svelte';
	import SignatureCapture from '$lib/components/SignatureCapture.svelte';

	const sectionNames: Record<string, string> = {
		identity: 'Identity',
		contact: 'Contact',
		address: 'Address',
		employment: 'Employment',
		education: 'Education',
		contacts: 'Next of kin & contacts',
		financial: 'Financial',
		identification: 'Identification',
		custom: 'Reusable answers'
	};

	let vault = $state<ProfileVault | null>(null);
	let loading = $state(true);
	let saving = $state(false);
	let showFactEditor = $state(false);
	let showSignature = $state(false);
	let displayName = $state('');
	let error = $state('');
	let notice = $state('');

	const groupedFacts = $derived.by(() => {
		const groups: Record<string, ProfileFact[]> = {};
		for (const fact of vault?.facts ?? []) {
			groups[fact.namespace] = [...(groups[fact.namespace] ?? []), fact];
		}
		return Object.entries(groups);
	});

	onMount(() => void load());

	async function load() {
		loading = true;
		error = '';
		try {
			vault = await api.profile();
			displayName = vault.profile.display_name ?? '';
		} catch (cause) {
			error = cause instanceof Error ? cause.message : 'Your details could not be loaded.';
		} finally {
			loading = false;
		}
	}

	async function saveName(event: SubmitEvent) {
		event.preventDefault();
		saving = true;
		try {
			const profile = await api.updateProfile({ display_name: displayName.trim() });
			if (vault) vault = { ...vault, profile };
			notice = 'Profile name updated.';
		} catch (cause) {
			error = cause instanceof Error ? cause.message : 'Your name could not be updated.';
		} finally {
			saving = false;
		}
	}

	async function saveFact(input: Parameters<typeof api.saveFact>[0]) {
		saving = true;
		error = '';
		try {
			const fact = await api.saveFact(input);
			if (vault) {
				vault = {
					...vault,
					facts: [
						...vault.facts.filter(
							(item) => !(item.namespace === fact.namespace && item.fact_key === fact.fact_key)
						),
						fact
					]
				};
			}
			showFactEditor = false;
			notice = 'Confirmed detail saved.';
		} catch (cause) {
			error = cause instanceof Error ? cause.message : 'The detail could not be saved.';
		} finally {
			saving = false;
		}
	}

	async function deleteFact(fact: ProfileFact) {
		if (!window.confirm(`Delete ${labelFor(fact.fact_key)} from your reusable details?`)) return;
		await api.deleteFact(fact.id);
		if (vault) vault = { ...vault, facts: vault.facts.filter((item) => item.id !== fact.id) };
	}

	async function saveSignature(kind: string, imageDataUrl: string) {
		saving = true;
		error = '';
		try {
			const signature = await api.saveSignature({ kind, image_data_url: imageDataUrl });
			if (vault) vault = { ...vault, signature };
			showSignature = false;
			notice = 'Signature encrypted and saved. It has not been applied to a document.';
		} catch (cause) {
			error =
				cause instanceof Error
					? `${cause.message} If your session is older than 15 minutes, sign in again first.`
					: 'The signature could not be saved.';
		} finally {
			saving = false;
		}
	}

	async function deleteSignature() {
		if (!window.confirm('Permanently delete your saved signature?')) return;
		await api.deleteSignature();
		if (vault) vault = { ...vault, signature: null };
		notice = 'Saved signature deleted.';
	}

	function labelFor(value: string) {
		return value.replaceAll('_', ' ').replace(/\b\w/g, (letter) => letter.toUpperCase());
	}

	function formatDate(value: string) {
		return new Intl.DateTimeFormat('en-NG', {
			day: 'numeric',
			month: 'short',
			year: 'numeric'
		}).format(new Date(value));
	}
</script>

<svelte:head><title>My details — Docufill</title></svelte:head>

<main class="mx-auto max-w-5xl px-5 py-8 sm:px-8 sm:py-12">
	<div>
		<p class="eyebrow mb-2">Personal knowledge vault</p>
		<h1 class="text-4xl font-extrabold tracking-[-0.055em] text-ink">Your confirmed details</h1>
		<p class="mt-2 max-w-2xl text-sm leading-6 text-ink-muted">
			Add information as documents need it. Exact facts stay separate from calculations and AI
			drafts.
		</p>
	</div>

	{#if notice}
		<p class="mt-6 rounded-xl bg-positive/10 p-4 text-sm font-semibold text-positive" role="status">
			{notice}
		</p>
	{/if}
	{#if error}
		<p class="mt-6 rounded-xl bg-negative/10 p-4 text-sm font-semibold text-negative" role="alert">
			{error}
		</p>
	{/if}

	{#if loading}
		<div class="mt-8 grid gap-5">
			<div class="surface h-40 p-5"><div class="skeleton h-full rounded-xl"></div></div>
			<div class="surface h-64 p-5"><div class="skeleton h-full rounded-xl"></div></div>
		</div>
	{:else if vault}
		<section class="surface mt-8 p-5 sm:p-7">
			<div class="flex flex-col gap-5 sm:flex-row sm:items-center">
				{#if vault.profile.avatar_url}
					<img
						src={vault.profile.avatar_url}
						alt=""
						class="size-16 rounded-2xl border border-line object-cover"
						referrerpolicy="no-referrer"
					/>
				{:else}
					<div class="grid size-16 place-items-center rounded-2xl bg-brand-soft text-brand-strong">
						<UserRound size={27} />
					</div>
				{/if}
				<form class="flex flex-1 flex-col gap-3 sm:flex-row sm:items-end" onsubmit={saveName}>
					<label class="flex-1 text-xs font-extrabold tracking-wider text-ink-muted uppercase">
						Display name
						<input
							required
							minlength="2"
							maxlength="100"
							bind:value={displayName}
							class="mt-2 min-h-11 w-full rounded-control border-line bg-canvas text-sm text-ink"
						/>
					</label>
					<Button type="submit" variant="secondary" loading={saving}>Save name</Button>
				</form>
			</div>
			<p class="mt-4 flex items-center gap-2 text-xs font-semibold text-positive">
				<ShieldCheck size={15} /> Email and identity begin with your authenticated account
			</p>
		</section>

		<section class="surface mt-5 p-5 sm:p-7">
			<div class="flex flex-col justify-between gap-4 sm:flex-row sm:items-center">
				<div>
					<h2 class="text-xl font-extrabold text-ink">Reusable information</h2>
					<p class="mt-1 text-xs text-ink-muted">
						Sensitive values are encrypted and masked in previews.
					</p>
				</div>
				<Button onclick={() => (showFactEditor = true)} disabled={showFactEditor}>
					<CirclePlus size={17} /> Add a detail
				</Button>
			</div>

			{#if showFactEditor}
				<div class="mt-5">
					<FactEditor onsave={saveFact} oncancel={() => (showFactEditor = false)} {saving} />
				</div>
			{/if}

			{#if groupedFacts.length === 0 && !showFactEditor}
				<div class="mt-6 rounded-2xl border border-dashed border-line bg-canvas p-8 text-center">
					<p class="font-bold text-ink">No reusable details yet</p>
					<p class="mt-1 text-sm text-ink-muted">
						Docufill can also suggest details as your first document asks for them.
					</p>
				</div>
			{:else}
				<div class="mt-6 space-y-7">
					{#each groupedFacts as [section, facts] (section)}
						<div>
							<h3 class="mb-2 text-xs font-extrabold tracking-[0.14em] text-brand-strong uppercase">
								{sectionNames[section] ?? labelFor(section)}
							</h3>
							<div class="divide-y divide-line rounded-xl border border-line">
								{#each facts as fact (fact.id)}
									<div class="flex items-start gap-4 p-4">
										<div class="min-w-0 flex-1">
											<p class="text-sm font-extrabold text-ink">{labelFor(fact.fact_key)}</p>
											<p class="mt-1 truncate text-sm text-ink-muted">
												{fact.value_preview ?? 'Encrypted value'}
											</p>
											<p
												class="mt-2 flex flex-wrap gap-x-4 gap-y-1 text-[11px] font-semibold text-ink-muted"
											>
												<span class="flex items-center gap-1"
													><FileClock size={13} /> Confirmed {formatDate(fact.confirmed_at)}</span
												>
												<span
													>Used in {fact.usage_count}
													{fact.usage_count === 1 ? 'field' : 'fields'}</span
												>
												<span>Source: {fact.source_type}</span>
											</p>
										</div>
										<Button
											variant="ghost"
											onclick={() => deleteFact(fact)}
											aria-label={`Delete ${labelFor(fact.fact_key)}`}
										>
											<Trash2 size={16} />
										</Button>
									</div>
								{/each}
							</div>
						</div>
					{/each}
				</div>
			{/if}
		</section>

		<section class="surface mt-5 p-5 sm:p-7">
			<div class="flex flex-col justify-between gap-4 sm:flex-row sm:items-center">
				<div>
					<div class="flex items-center gap-2">
						<LockKeyhole size={18} class="text-brand-strong" />
						<h2 class="text-xl font-extrabold text-ink">Signature vault</h2>
					</div>
					<p class="mt-1 text-xs text-ink-muted">
						Separately encrypted and never sent to AI or document memory.
					</p>
				</div>
				{#if vault.signature}
					<div class="flex gap-2">
						<Button variant="secondary" onclick={() => (showSignature = !showSignature)}
							><PenLine size={16} /> Replace</Button
						>
						<Button variant="danger" onclick={deleteSignature}><Trash2 size={16} /> Delete</Button>
					</div>
				{:else}
					<Button onclick={() => (showSignature = !showSignature)}
						><PenLine size={16} /> Create signature</Button
					>
				{/if}
			</div>
			{#if vault.signature && !showSignature}
				<div class="mt-5 rounded-xl border border-line bg-canvas p-4">
					<p class="text-sm font-bold text-ink">
						Saved {vault.signature.kind} signature · version {vault.signature.version}
					</p>
					<p class="mt-1 text-xs text-ink-muted">
						Created {formatDate(vault.signature.created_at)} · explicit confirmation required per document
					</p>
				</div>
			{/if}
			{#if showSignature}
				<div class="mt-5"><SignatureCapture onsave={saveSignature} {saving} /></div>
			{/if}
		</section>
	{/if}
</main>
