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

	const overview = $derived.by(() => {
		if (loading) return 'Loading your documents…';
		if (documents.length === 0) return 'Upload a form to get started.';
		const pending = documents.filter((document) => document.status === 'needs_input').length;
		const total = `${documents.length} ${documents.length === 1 ? 'document' : 'documents'}`;
		return pending === 0 ? total : `${total} · ${pending} need your input`;
	});

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

	function statusDot(status: string) {
		if (status === 'completed' || status === 'ready') return 'bg-positive';
		if (status === 'failed') return 'bg-negative';
		if (status === 'needs_input') return 'bg-brand';
		return 'bg-ink-muted/40';
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

<main class="mx-auto max-w-5xl px-5 py-8 sm:px-8 sm:py-12">
	<div class="flex flex-col justify-between gap-4 sm:flex-row sm:items-center">
		<div>
			<h1 class="ask text-2xl leading-8 text-ink">Documents</h1>
			<p class="mt-1 text-sm text-ink-muted">{overview}</p>
		</div>
		<a
			href={resolve('/documents/new')}
			class="inline-flex min-h-11 w-fit items-center justify-center gap-2 rounded-control border border-ink bg-ink px-4 py-2.5 text-sm font-semibold text-canvas transition hover:opacity-90"
		>
			<FilePlus2 size={17} /> New document
		</a>
	</div>

	<section class="mt-8" aria-labelledby="history-title">
		<div class="mb-3 flex items-center justify-between">
			<h2 id="history-title" class="text-sm font-semibold text-ink-muted">Recent activity</h2>
			<Button variant="ghost" onclick={() => load()} {loading}>
				<RefreshCw size={15} /> Refresh
			</Button>
		</div>

		{#if loading}
			<div class="surface divide-y divide-line overflow-hidden" aria-label="Loading documents">
				{#each [1, 2, 3, 4, 5, 6] as item (item)}
					<div class="px-4 py-3.5 sm:px-5"><div class="skeleton h-12 rounded-control"></div></div>
				{/each}
			</div>
		{:else if error}
			<div class="surface p-8 text-center">
				<p class="font-bold text-negative" role="alert">{error}</p>
				<Button class="mt-4" onclick={() => load()}>Try again</Button>
			</div>
		{:else if documents.length === 0}
			<div class="surface px-6 py-12 text-center">
				<div
					class="mx-auto grid size-11 place-items-center rounded-control border border-line text-ink-muted"
				>
					<Files size={20} />
				</div>
				<h3 class="mt-4 text-base font-semibold text-ink">No documents yet</h3>
				<p class="mx-auto mt-1.5 max-w-sm text-sm leading-6 text-ink-muted">
					Upload a fillable or text-based PDF. Scanned image PDFs are not supported in this first
					release.
				</p>
				<a
					href={resolve('/documents/new')}
					class="mt-5 inline-flex min-h-11 items-center gap-2 rounded-control border border-ink bg-ink px-4 py-2.5 text-sm font-semibold text-canvas"
				>
					Upload a PDF <ArrowRight size={16} />
				</a>
				<p class="mt-5 flex items-center justify-center gap-2 text-xs text-ink-muted">
					<ShieldCheck size={14} /> Originals stay private and unchanged
				</p>
			</div>
		{:else}
			<div class="surface divide-y divide-line overflow-hidden">
				{#each documents.slice(0, visibleCount) as document (document.id)}
					<a
						href={resolve('/(app)/documents/[documentId]', { documentId: document.id })}
						class="group flex min-w-0 items-center gap-4 px-4 py-3.5 transition duration-[var(--motion-fast)] hover:bg-canvas sm:px-5"
					>
						<div class="min-w-0 flex-1">
							<h3 class="truncate text-sm font-semibold text-ink">{document.subject}</h3>
							<p class="mt-0.5 truncate text-xs text-ink-muted" title={document.original_name}>
								{document.original_name}
							</p>
							<p class="mt-2 flex flex-wrap items-center gap-x-2.5 gap-y-1 text-xs text-ink-muted">
								<span class="inline-flex items-center gap-1.5">
									<span class="size-1.5 rounded-full {statusDot(document.status)}"></span>
									{statusLabel(document.status)}
								</span>
								<span aria-hidden="true">·</span>
								<span>{formatDate(document.updated_at)}</span>
							</p>
							{#if document.status === 'processing'}
								<div class="mt-2.5 h-1 overflow-hidden rounded-full bg-line">
									<div
										class="h-full rounded-full bg-ink transition-all"
										style={`width: ${document.progress}%`}
									></div>
								</div>
							{/if}
						</div>
						<ArrowRight
							size={16}
							class="shrink-0 text-ink-muted/60 transition group-hover:text-ink"
						/>
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
