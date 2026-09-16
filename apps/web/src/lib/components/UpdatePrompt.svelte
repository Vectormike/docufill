<script lang="ts">
	import { RefreshCw } from '@lucide/svelte';
	import { onMount } from 'svelte';
	import Button from './Button.svelte';

	let waiting = $state<ServiceWorker | null>(null);
	let updateRequested = false;

	onMount(() => {
		if (!('serviceWorker' in navigator)) return;
		let registration: ServiceWorkerRegistration | undefined;
		const watch = async () => {
			registration = await navigator.serviceWorker.getRegistration();
			if (!registration) return;
			waiting = registration.waiting;
			registration.addEventListener('updatefound', onUpdateFound);
		};
		const onUpdateFound = () => {
			const installing = registration?.installing;
			installing?.addEventListener('statechange', () => {
				if (installing.state === 'installed' && navigator.serviceWorker.controller) {
					waiting = registration?.waiting ?? null;
				}
			});
		};
		const onControllerChange = () => {
			if (updateRequested) window.location.reload();
		};
		navigator.serviceWorker.addEventListener('controllerchange', onControllerChange);
		void watch();
		return () => {
			registration?.removeEventListener('updatefound', onUpdateFound);
			navigator.serviceWorker.removeEventListener('controllerchange', onControllerChange);
		};
	});

	function update() {
		updateRequested = true;
		waiting?.postMessage({ type: 'SKIP_WAITING' });
	}
</script>

{#if waiting}
	<div
		class="fixed inset-x-3 bottom-4 z-50 mx-auto flex max-w-md items-center justify-between gap-3 rounded-2xl border border-line bg-surface-raised p-3 pl-4 shadow-card"
		role="status"
	>
		<p class="text-sm font-semibold text-ink">A safer, newer Docufill is ready.</p>
		<Button onclick={update}>
			<RefreshCw size={16} />
			Update
		</Button>
	</div>
{/if}
