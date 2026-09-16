<script lang="ts">
	import { ArrowLeft, ArrowRight, Move, Save } from '@lucide/svelte';
	import { onMount } from 'svelte';
	import type { PDFDocumentProxy, PageViewport } from 'pdfjs-dist';
	import pdfWorker from 'pdfjs-dist/build/pdf.worker.min.mjs?url';
	import type { DocumentField } from '$lib/api/client';
	import Button from './Button.svelte';

	let {
		url,
		fields,
		onsave
	}: {
		url: string;
		fields: DocumentField[];
		onsave: (
			fieldId: string,
			layout: {
				x: number;
				y: number;
				width: number;
				height: number;
				font_size: number | null;
				alignment: string;
			}
		) => Promise<void>;
	} = $props();

	let container = $state<HTMLDivElement>();
	let canvas = $state<HTMLCanvasElement>();
	let document = $state<PDFDocumentProxy>();
	let viewport = $state<PageViewport>();
	let pageNumber = $state(1);
	let selectedId = $state('');
	let draft = $state({
		x: 0,
		y: 0,
		width: 120,
		height: 20,
		font_size: 12 as number | null,
		alignment: 'left'
	});
	let loading = $state(true);
	let saving = $state(false);
	let error = $state('');

	const pageFields = $derived(fields.filter((field) => field.page_number === pageNumber));

	onMount(() => {
		void load();
		const observer = new ResizeObserver(() => void renderPage());
		if (container) observer.observe(container);
		return () => {
			observer.disconnect();
			void document?.cleanup();
		};
	});

	async function load() {
		loading = true;
		error = '';
		try {
			const pdfjs = await import('pdfjs-dist');
			pdfjs.GlobalWorkerOptions.workerSrc = pdfWorker;
			document = await pdfjs.getDocument({ url, withCredentials: false }).promise;
			await renderPage();
		} catch (cause) {
			error = cause instanceof Error ? cause.message : 'The PDF preview could not be opened.';
		} finally {
			loading = false;
		}
	}

	async function renderPage() {
		if (!document || !canvas || !container) return;
		const page = await document.getPage(pageNumber);
		const base = page.getViewport({ scale: 1 });
		const scale = Math.min(2, Math.max(0.4, (container.clientWidth - 2) / base.width));
		viewport = page.getViewport({ scale });
		const ratio = Math.min(window.devicePixelRatio || 1, 2);
		canvas.width = viewport.width * ratio;
		canvas.height = viewport.height * ratio;
		canvas.style.width = `${viewport.width}px`;
		canvas.style.height = `${viewport.height}px`;
		const context = canvas.getContext('2d');
		if (!context) return;
		await page.render({
			canvas,
			canvasContext: context,
			viewport,
			transform: ratio === 1 ? undefined : [ratio, 0, 0, ratio, 0, 0]
		}).promise;
	}

	function select(field: DocumentField) {
		selectedId = field.id;
		draft = {
			x: field.x,
			y: field.y,
			width: field.width,
			height: field.height,
			font_size: field.font_size ?? 12,
			alignment: field.alignment
		};
	}

	function rectangle(field: DocumentField) {
		if (!viewport) return 'display: none';
		const source = field.id === selectedId ? draft : field;
		const [x1, y1] = viewport.convertToViewportPoint(source.x, source.y);
		const [x2, y2] = viewport.convertToViewportPoint(
			source.x + source.width,
			source.y + source.height
		);
		return `left:${Math.min(x1, x2)}px;top:${Math.min(y1, y2)}px;width:${Math.abs(x2 - x1)}px;height:${Math.abs(y2 - y1)}px`;
	}

	async function save() {
		if (!selectedId) return;
		saving = true;
		try {
			await onsave(selectedId, draft);
		} finally {
			saving = false;
		}
	}

	async function changePage(next: number) {
		if (!document || next < 1 || next > document.numPages) return;
		pageNumber = next;
		selectedId = '';
		await renderPage();
	}
</script>

<div class="grid gap-5 xl:grid-cols-[minmax(0,1fr)_18rem]">
	<div>
		<div class="mb-3 flex items-center justify-between">
			<Button
				variant="ghost"
				onclick={() => changePage(pageNumber - 1)}
				disabled={pageNumber === 1}
			>
				<ArrowLeft size={16} /> Previous
			</Button>
			<p class="text-xs font-extrabold text-ink-muted">
				Page {pageNumber} of {document?.numPages ?? '…'}
			</p>
			<Button
				variant="ghost"
				onclick={() => changePage(pageNumber + 1)}
				disabled={!document || pageNumber === document.numPages}
			>
				Next <ArrowRight size={16} />
			</Button>
		</div>
		<div
			bind:this={container}
			class="relative mx-auto overflow-auto rounded-xl border border-line bg-stone-200"
		>
			{#if loading}
				<div class="grid aspect-[3/4] place-items-center">
					<div
						class="size-8 animate-spin rounded-full border-3 border-brand border-r-transparent"
					></div>
				</div>
			{/if}
			<canvas bind:this={canvas} class:invisible={loading} class="mx-auto block"></canvas>
			{#if !loading}
				<div
					class="pointer-events-none absolute inset-0 mx-auto"
					style={`width:${viewport?.width ?? 0}px;height:${viewport?.height ?? 0}px`}
				>
					{#each pageFields as field (field.id)}
						<button
							type="button"
							onclick={() => select(field)}
							style={rectangle(field)}
							class="pointer-events-auto absolute overflow-hidden border-2 text-left text-[9px] font-bold transition {field.id ===
							selectedId
								? 'z-10 border-brand bg-brand/25 text-[#211d14]'
								: 'border-positive/70 bg-positive/10 text-positive hover:bg-positive/20'}"
							title={field.label}
						>
							<span class="line-clamp-2 px-1">{field.label}</span>
						</button>
					{/each}
				</div>
			{/if}
		</div>
		{#if error}<p class="mt-3 text-sm font-semibold text-negative" role="alert">{error}</p>{/if}
	</div>

	<aside class="rounded-xl border border-line bg-canvas p-4">
		<div class="flex items-center gap-2">
			<Move size={17} class="text-brand-strong" />
			<h3 class="font-extrabold text-ink">Field placement</h3>
		</div>
		{#if selectedId}
			<div class="mt-4 grid grid-cols-2 gap-3">
				{#each [{ key: 'x', label: 'X' }, { key: 'y', label: 'Y' }, { key: 'width', label: 'Width' }, { key: 'height', label: 'Height' }] as input (input.key)}
					<label class="text-[11px] font-extrabold tracking-wider text-ink-muted uppercase">
						{input.label}
						<input
							type="number"
							min="0"
							step="1"
							value={draft[input.key as keyof typeof draft] as number}
							oninput={(event) => {
								draft = { ...draft, [input.key]: Number(event.currentTarget.value) };
							}}
							class="mt-1 min-h-10 w-full rounded-lg border-line bg-surface-raised text-sm text-ink"
						/>
					</label>
				{/each}
				<label class="text-[11px] font-extrabold tracking-wider text-ink-muted uppercase">
					Font size
					<input
						type="number"
						min="6"
						max="24"
						step=".5"
						value={draft.font_size ?? 12}
						oninput={(event) =>
							(draft = { ...draft, font_size: Number(event.currentTarget.value) })}
						class="mt-1 min-h-10 w-full rounded-lg border-line bg-surface-raised text-sm text-ink"
					/>
				</label>
				<label class="text-[11px] font-extrabold tracking-wider text-ink-muted uppercase">
					Alignment
					<select
						bind:value={draft.alignment}
						class="mt-1 min-h-10 w-full rounded-lg border-line bg-surface-raised text-sm text-ink"
					>
						<option value="left">Left</option>
						<option value="center">Centre</option>
						<option value="right">Right</option>
					</select>
				</label>
			</div>
			<Button class="mt-4 w-full" onclick={save} loading={saving}
				><Save size={16} /> Save placement</Button
			>
			<p class="mt-3 text-xs leading-5 text-ink-muted">
				Generate a new preview after saving corrections.
			</p>
		{:else}
			<p class="mt-4 text-sm leading-6 text-ink-muted">
				Select a highlighted field on the page to adjust its exact position and fit.
			</p>
		{/if}
	</aside>
</div>
