<script lang="ts">
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { onMount } from 'svelte';
	import { api } from '$lib/api/client';
	import BrandMark from '$lib/components/BrandMark.svelte';
	import { getSupabase } from '$lib/supabase';

	let error = $state('');

	onMount(async () => {
		try {
			const params = new URL(window.location.href).searchParams;
			const code = params.get('code');
			if (code) {
				const flowId = params.get('sb_flow_id');
				const { error: exchangeError } = await getSupabase().auth.exchangeCodeForSession(
					code,
					flowId ? { flowId } : undefined
				);
				if (exchangeError) throw exchangeError;
			}
			const { data } = await getSupabase().auth.getSession();
			if (!data.session) throw new Error('The sign-in link is invalid or has expired.');
			const destination = await nextDestination();
			await goto(resolve(destination), { replaceState: true });
		} catch (cause) {
			error = cause instanceof Error ? cause.message : 'Sign-in could not be completed.';
		}
	});

	async function nextDestination() {
		try {
			const vault = await api.profile();
			return vault.profile.onboarding_completed ? '/documents' : '/onboarding';
		} catch {
			return '/onboarding';
		}
	}
</script>

<svelte:head><title>Signing in — Docufill</title></svelte:head>

<main class="grid min-h-screen place-items-center px-5">
	<div class="surface w-full max-w-sm p-8 text-center">
		<div class="mb-6 flex justify-center"><BrandMark /></div>
		{#if error}
			<h1 class="text-xl font-extrabold text-ink">We could not sign you in</h1>
			<p class="mt-3 text-sm leading-6 text-negative" role="alert">{error}</p>
			<a
				class="mt-6 inline-flex min-h-11 items-center font-bold text-brand-strong"
				href={resolve('/')}
			>
				Return to sign in
			</a>
		{:else}
			<div
				class="mx-auto size-8 animate-spin rounded-full border-3 border-brand border-r-transparent"
			></div>
			<h1 class="mt-5 text-xl font-extrabold text-ink">Opening your workspace…</h1>
			<p class="mt-2 text-sm text-ink-muted">Finishing your secure sign-in.</p>
		{/if}
	</div>
</main>
