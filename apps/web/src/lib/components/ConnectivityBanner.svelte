<script lang="ts">
	import { WifiOff } from '@lucide/svelte';
	import { onMount } from 'svelte';

	let online = $state(true);

	onMount(() => {
		const update = () => (online = navigator.onLine);
		update();
		window.addEventListener('online', update);
		window.addEventListener('offline', update);
		return () => {
			window.removeEventListener('online', update);
			window.removeEventListener('offline', update);
		};
	});
</script>

{#if !online}
	<div
		class="fixed inset-x-3 top-3 z-50 mx-auto flex max-w-md items-center gap-3 rounded-xl border border-warning/30 bg-surface-raised px-4 py-3 text-sm font-semibold text-ink shadow-card"
		role="status"
	>
		<WifiOff size={18} class="shrink-0 text-warning" />
		<span>You’re offline. Private documents stay online-only and have not been cached.</span>
	</div>
{/if}
