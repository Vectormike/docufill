<script lang="ts">
	import { Download, Share } from '@lucide/svelte';
	import { onMount } from 'svelte';
	import Button from './Button.svelte';

	interface InstallEvent extends Event {
		prompt(): Promise<void>;
		userChoice: Promise<{ outcome: 'accepted' | 'dismissed' }>;
	}

	let installEvent = $state<InstallEvent | null>(null);
	let showIosHelp = $state(false);
	let installed = $state(false);

	onMount(() => {
		installed = window.matchMedia('(display-mode: standalone)').matches;
		const listener = (event: Event) => {
			event.preventDefault();
			installEvent = event as InstallEvent;
		};
		window.addEventListener('beforeinstallprompt', listener);
		return () => window.removeEventListener('beforeinstallprompt', listener);
	});

	async function install() {
		if (installEvent) {
			await installEvent.prompt();
			const choice = await installEvent.userChoice;
			installed = choice.outcome === 'accepted';
			installEvent = null;
			return;
		}
		showIosHelp = !showIosHelp;
	}
</script>

{#if !installed}
	<div class="relative">
		<Button variant="ghost" onclick={install} aria-expanded={showIosHelp}>
			<Download size={17} />
			Install app
		</Button>
		{#if showIosHelp}
			<div
				class="absolute top-13 right-0 z-20 w-64 rounded-xl border border-line bg-surface-raised p-4 text-xs leading-5 text-ink-muted shadow-card"
				role="status"
			>
				<span class="mb-1 flex items-center gap-2 font-bold text-ink">
					<Share size={15} /> On iPhone or iPad
				</span>
				Tap Share, then “Add to Home Screen”. On Android, use your browser’s install option.
			</div>
		{/if}
	</div>
{/if}
