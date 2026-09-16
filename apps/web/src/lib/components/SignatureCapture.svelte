<script lang="ts">
	import { Eraser, PenLine, Type, Upload } from '@lucide/svelte';
	import { onMount } from 'svelte';
	import Button from './Button.svelte';

	let {
		onsave,
		saving = false
	}: { onsave: (kind: string, dataUrl: string) => Promise<void>; saving?: boolean } = $props();

	let mode = $state<'drawn' | 'typed' | 'uploaded'>('drawn');
	let canvas = $state<HTMLCanvasElement>();
	let drawing = false;
	let hasDrawing = $state(false);
	let typedName = $state('');
	let uploadedData = $state('');

	onMount(() => {
		resizeCanvas();
		window.addEventListener('resize', resizeCanvas);
		return () => window.removeEventListener('resize', resizeCanvas);
	});

	function resizeCanvas() {
		if (!canvas) return;
		const rect = canvas.getBoundingClientRect();
		const ratio = Math.min(window.devicePixelRatio || 1, 2);
		canvas.width = Math.max(1, rect.width * ratio);
		canvas.height = Math.max(1, rect.height * ratio);
		const context = canvas.getContext('2d');
		context?.scale(ratio, ratio);
		if (context) {
			context.strokeStyle = '#211d14';
			context.lineWidth = 2.25;
			context.lineCap = 'round';
			context.lineJoin = 'round';
		}
		hasDrawing = false;
	}

	function point(event: PointerEvent) {
		const rect = canvas?.getBoundingClientRect();
		return { x: event.clientX - (rect?.left ?? 0), y: event.clientY - (rect?.top ?? 0) };
	}

	function start(event: PointerEvent) {
		if (!canvas) return;
		drawing = true;
		canvas.setPointerCapture(event.pointerId);
		const context = canvas.getContext('2d');
		const position = point(event);
		context?.beginPath();
		context?.moveTo(position.x, position.y);
	}

	function move(event: PointerEvent) {
		if (!drawing || !canvas) return;
		const position = point(event);
		const context = canvas.getContext('2d');
		context?.lineTo(position.x, position.y);
		context?.stroke();
		hasDrawing = true;
	}

	function stop() {
		drawing = false;
	}

	function clear() {
		if (!canvas) return;
		const context = canvas.getContext('2d');
		context?.clearRect(0, 0, canvas.width, canvas.height);
		hasDrawing = false;
	}

	function readUpload(file?: File) {
		if (!file) return;
		if (!['image/png', 'image/jpeg'].includes(file.type) || file.size > 2 * 1024 * 1024) {
			uploadedData = '';
			return;
		}
		const reader = new FileReader();
		reader.onload = () => (uploadedData = String(reader.result));
		reader.readAsDataURL(file);
	}

	function typedDataUrl() {
		const output = document.createElement('canvas');
		output.width = 1200;
		output.height = 360;
		const context = output.getContext('2d');
		if (!context) return '';
		context.fillStyle = '#211d14';
		context.textAlign = 'center';
		context.textBaseline = 'middle';
		context.font = 'italic 112px cursive';
		context.fillText(typedName.trim(), 600, 180, 1080);
		return output.toDataURL('image/png');
	}

	async function save() {
		const data =
			mode === 'drawn'
				? (canvas?.toDataURL('image/png') ?? '')
				: mode === 'typed'
					? typedDataUrl()
					: uploadedData;
		if (data) await onsave(mode, data);
	}

	const canSave = $derived(
		(mode === 'drawn' && hasDrawing) ||
			(mode === 'typed' && typedName.trim().length >= 2) ||
			(mode === 'uploaded' && Boolean(uploadedData))
	);
</script>

<div>
	<div class="grid grid-cols-3 gap-1 rounded-xl bg-canvas p-1">
		{#each [{ key: 'drawn' as const, label: 'Draw', icon: PenLine }, { key: 'typed' as const, label: 'Type', icon: Type }, { key: 'uploaded' as const, label: 'Upload', icon: Upload }] as option (option.key)}
			<button
				type="button"
				onclick={() => (mode = option.key)}
				class="flex min-h-11 items-center justify-center gap-2 rounded-lg text-xs font-extrabold transition {mode ===
				option.key
					? 'bg-surface-raised text-ink shadow-sm'
					: 'text-ink-muted'}"
			>
				<option.icon size={16} />
				{option.label}
			</button>
		{/each}
	</div>

	<div class="mt-4">
		{#if mode === 'drawn'}
			<div class="relative">
				<canvas
					bind:this={canvas}
					onpointerdown={start}
					onpointermove={move}
					onpointerup={stop}
					onpointercancel={stop}
					class="h-44 w-full touch-none rounded-xl border border-line bg-white"
					aria-label="Draw your signature"
				></canvas>
				<button
					type="button"
					onclick={clear}
					class="absolute top-3 right-3 grid size-10 place-items-center rounded-lg border border-line bg-white text-stone-700"
					aria-label="Clear signature"
				>
					<Eraser size={17} />
				</button>
			</div>
		{:else if mode === 'typed'}
			<label class="text-sm font-bold text-ink" for="typed-signature">Type your full name</label>
			<input
				id="typed-signature"
				bind:value={typedName}
				maxlength="100"
				placeholder="Your full name"
				class="mt-2 min-h-12 w-full rounded-control border-line bg-canvas text-ink"
			/>
			<div
				class="mt-3 grid h-28 place-items-center rounded-xl border border-line bg-white px-4 text-center font-serif text-3xl text-stone-900 italic"
			>
				{typedName || 'Your signature'}
			</div>
		{:else}
			<label
				class="grid min-h-40 cursor-pointer place-items-center rounded-xl border-2 border-dashed border-line bg-canvas p-5 text-center"
			>
				<input
					type="file"
					accept="image/png,image/jpeg"
					class="sr-only"
					onchange={(event) => readUpload(event.currentTarget.files?.[0])}
				/>
				{#if uploadedData}
					<img
						src={uploadedData}
						alt="Signature preview"
						class="max-h-28 max-w-full object-contain"
					/>
				{:else}
					<span>
						<Upload size={24} class="mx-auto text-brand-strong" />
						<span class="mt-2 block text-sm font-bold text-ink">Choose PNG or JPEG</span>
						<span class="mt-1 block text-xs text-ink-muted">Maximum 2 MB</span>
					</span>
				{/if}
			</label>
		{/if}
	</div>

	<Button class="mt-4 w-full" onclick={save} disabled={!canSave} loading={saving}>
		Save signature securely
	</Button>
	<p class="mt-3 text-xs leading-5 text-ink-muted">
		Saving does not sign a document. Docufill asks again before every use.
	</p>
</div>
