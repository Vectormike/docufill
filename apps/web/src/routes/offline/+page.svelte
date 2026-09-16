<script lang="ts">
	import { RefreshCw, ShieldCheck, WifiOff } from '@lucide/svelte';
	import BrandMark from '$lib/components/BrandMark.svelte';
	import Button from '$lib/components/Button.svelte';
</script>

<svelte:head><title>Offline — Docufill</title></svelte:head>

<main class="grid min-h-screen place-items-center px-5">
	<section class="surface w-full max-w-lg p-7 text-center sm:p-10">
		<div class="mb-8 flex justify-center"><BrandMark /></div>
		<div class="mx-auto grid size-16 place-items-center rounded-xl bg-brand-soft text-brand-strong">
			<WifiOff size={28} />
		</div>
		<h1 class="mt-6 text-3xl font-semibold tracking-tight text-ink">You’re offline</h1>
		<p class="mx-auto mt-3 max-w-sm text-sm leading-6 text-ink-muted">
			Reconnect to open profiles, documents, signatures, or participant questions. Sensitive
			information is deliberately never saved for offline use.
		</p>
		<div class="mt-5 flex items-center justify-center gap-2 text-xs font-bold text-positive">
			<ShieldCheck size={16} /> Private data stayed out of the browser cache
		</div>
		<Button class="mt-7" onclick={() => window.location.reload()}>
			<RefreshCw size={16} /> Try again
		</Button>
	</section>
</main>
