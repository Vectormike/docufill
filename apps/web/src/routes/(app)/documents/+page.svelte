<script lang="ts">
	import { ArrowRight, FilePlus2, Files, RefreshCw, ShieldCheck } from '@lucide/svelte';
	import { resolve } from '$app/paths';
	import { onMount } from 'svelte';
	import { api, type DocumentSummary } from '$lib/api/client';
	import Button from '$lib/components/Button.svelte';
	import { getSupabase } from '$lib/supabase';

	let documents = $state<DocumentSummary[]>([]);
	let loading = $state(true);
	let error = $state('');
	let visibleCount = $state(12);

	onMount(() => {
		void load();
		const channel = getSupabase()
			.channel('document-status')
			.on('postgres_changes', { event: '*', schema: 'public', table: 'documents' }, () => {
				void load(false);
			})
			.subscribe();
		return () => {
			void getSupabase().removeChannel(channel);
		};
	});

	async function load(showLoader = true) {
		if (showLoader) loading = true;
		error = '';
		try {
			documents = await api.documents();
		} catch (cause) {
			error = cause instanceof Error ? cause.message : 'Documents could not be loaded.';
		} finally {
			loading = false;
		}
	}

	function statusLabel(status: string) {
		return (
			{
				uploaded: 'Queued',
				processing: 'Processing',
				needs_input: 'Needs your input',
				ready: 'Ready to review',
				completed: 'Completed',
				failed: 'Needs attention'
			}[status] ?? status
		);
	}

	function statusClass(status: string) {
		if (status === 'completed' || status === 'ready') return 'bg-positive/10 text-positive';
		if (status === 'failed') return 'bg-negative/10 text-negative';
		if (status === 'needs_input') return 'bg-brand-soft text-brand-strong';
		return 'bg-canvas text-ink-muted';
	}

	function formatDate(value: string) {
		return new Intl.DateTimeFormat('en-NG', {
			day: 'numeric',
			month: 'short',
			year: 'numeric'
		}).format(new Date(value));
	}
</script>

<svelte:head><title>Documents — Docufill</title></svelte:head>

<main class="mx-auto max-w-7xl px-5 py-8 sm:px-8 sm:py-12">
	<div class="flex flex-col justify-between gap-5 sm:flex-row sm:items-end">
		<div>
			<p class="eyebrow mb-2">Document workspace</p>
			<h1 class="text-4xl font-extrabold tracking-[-0.055em] text-ink">Your documents</h1>
			<p class="mt-2 text-sm text-ink-muted">
				Upload a form and answer only what Docufill cannot confirm.
			</p>
		</div>
		<a
			href={resolve('/documents/new')}
			class="inline-flex min-h-12 items-center justify-center gap-2 rounded-control border border-brand bg-brand px-5 py-3 text-sm font-extrabold text-[#211d14] shadow-sm transition hover:bg-brand-strong"
		>
			<FilePlus2 size={18} /> New document
		</a>
	</div>

	<div class="mt-8 grid gap-4 sm:grid-cols-3">
		<div class="surface p-5">
			<p class="text-xs font-bold tracking-wider text-ink-muted uppercase">All documents</p>
			<p class="mt-2 text-3xl font-extrabold tracking-tight text-ink">{documents.length}</p>
		</div>
		<div class="surface p-5">
			<p class="text-xs font-bold tracking-wider text-ink-muted uppercase">Need your input</p>
			<p class="mt-2 text-3xl font-extrabold tracking-tight text-ink">
				{documents.filter((document) => document.status === 'needs_input').length}
			</p>
		</div>
		<div class="surface p-5">
			<p class="text-xs font-bold tracking-wider text-ink-muted uppercase">Completed</p>
			<p class="mt-2 text-3xl font-extrabold tracking-tight text-ink">
				{documents.filter((document) => document.status === 'completed').length}
			</p>
		</div>
	</div>

	<section class="mt-8" aria-labelledby="history-title">
		<div class="mb-4 flex items-center justify-between">
			<h2 id="history-title" class="text-lg font-extrabold text-ink">Recent activity</h2>
			<Button variant="ghost" onclick={() => load()} {loading}>
				<RefreshCw size={16} /> Refresh
			</Button>
		</div>

		{#if loading}
			<div class="grid gap-4 md:grid-cols-2 xl:grid-cols-3" aria-label="Loading documents">
				{#each [1, 2, 3, 4, 5, 6] as item (item)}
					<div class="surface h-44 p-5"><div class="skeleton h-full rounded-xl"></div></div>
				{/each}
			</div>
		{:else if error}
			<div class="surface p-8 text-center">
				<p class="font-bold text-negative" role="alert">{error}</p>
				<Button class="mt-4" onclick={() => load()}>Try again</Button>
			</div>
		{:else if documents.length === 0}
			<div class="surface px-6 py-14 text-center">
				<div
					class="mx-auto grid size-16 place-items-center rounded-2xl bg-brand-soft text-brand-strong"
				>
					<Files size={28} />
				</div>
				<h3 class="mt-5 text-2xl font-extrabold tracking-tight text-ink">
					Your first form starts here
				</h3>
				<p class="mx-auto mt-2 max-w-md text-sm leading-6 text-ink-muted">
					Upload a fillable or text-based PDF. Scanned image PDFs are not supported in this first
					release.
				</p>
				<a
					href={resolve('/documents/new')}
					class="mt-6 inline-flex min-h-11 items-center gap-2 rounded-control bg-brand px-5 py-2.5 text-sm font-extrabold text-[#211d14]"
				>
					Upload a PDF <ArrowRight size={16} />
				</a>
				<p class="mt-5 flex items-center justify-center gap-2 text-xs font-semibold text-positive">
					<ShieldCheck size={15} /> Originals stay private and unchanged
				</p>
			</div>
		{:else}
			<div class="grid gap-4 md:grid-cols-2 xl:grid-cols-3">
				{#each documents.slice(0, visibleCount) as document (document.id)}
					<a
						href={resolve('/(app)/documents/[documentId]', { documentId: document.id })}
						class="surface group flex min-h-44 flex-col p-5 transition duration-[var(--motion-base)] hover:-translate-y-0.5 hover:border-brand"
					>
						<div class="flex items-start justify-between gap-3">
							<div
								class="grid size-11 shrink-0 place-items-center rounded-xl bg-brand-soft text-brand-strong"
							>
								<Files size={20} />
							</div>
							<span
								class="rounded-full px-2.5 py-1 text-[11px] font-extrabold {statusClass(
									document.status
								)}"
							>
								{statusLabel(document.status)}
							</span>
						</div>
						<h3 class="mt-4 line-clamp-2 leading-5 font-extrabold text-ink">{document.subject}</h3>
						<p class="mt-1 truncate text-xs text-ink-muted">{document.original_name}</p>
						<div class="mt-auto flex items-center justify-between pt-4 text-xs text-ink-muted">
							<span>{formatDate(document.updated_at)}</span>
							<ArrowRight size={16} class="transition group-hover:translate-x-1" />
						</div>
						{#if document.status === 'processing'}
							<div class="mt-3 h-1.5 overflow-hidden rounded-full bg-line">
								<div
									class="h-full rounded-full bg-brand transition-all"
									style={`width: ${document.progress}%`}
								></div>
							</div>
						{/if}
					</a>
				{/each}
			</div>
			{#if visibleCount < documents.length}
				<div class="mt-6 text-center">
					<Button variant="secondary" onclick={() => (visibleCount += 12)}>Load more</Button>
				</div>
			{/if}
		{/if}
	</section>
</main>
