<script lang="ts">
	import { ArrowRight, Check, LockKeyhole } from '@lucide/svelte';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { onMount } from 'svelte';
	import { currentSession, signInWithEmail, signInWithGoogle } from '$lib/auth';
	import BrandMark from '$lib/components/BrandMark.svelte';
	import Button from '$lib/components/Button.svelte';
	import FeatureIllustration from '$lib/components/FeatureIllustration.svelte';
	import { isSupabaseConfigured } from '$lib/supabase';

	const features = [
		{
			key: 'vault' as const,
			title: 'Fill once, reuse your details',
			body: 'Docufill remembers only the information you confirm, so every form gets easier.'
		},
		{
			key: 'questions' as const,
			title: 'Answer questions, not pages',
			body: 'We turn dense PDFs into a short, guided checklist and show where every answer came from.'
		},
		{
			key: 'invite' as const,
			title: 'Invite others and sign securely',
			body: 'Guarantors see only their assigned questions, while you stay in control of the final document.'
		}
	];

	let current = $state(0);
	let email = $state('');
	let loading = $state(false);
	let message = $state('');
	let error = $state('');
	let signedIn = $state(false);
	let animateLogo = $state(false);
	let touchStart = 0;

	onMount(async () => {
		animateLogo = sessionStorage.getItem('docufill-logo-seen') !== 'true';
		sessionStorage.setItem('docufill-logo-seen', 'true');
		if (isSupabaseConfigured()) {
			signedIn = Boolean(await currentSession().catch(() => null));
		}
	});

	async function googleSignIn() {
		error = '';
		if (!isSupabaseConfigured()) {
			error = 'Add your Supabase public keys to apps/web/.env before signing in.';
			return;
		}
		loading = true;
		try {
			await signInWithGoogle();
		} catch (cause) {
			error = cause instanceof Error ? cause.message : 'Google sign-in could not start.';
			loading = false;
		}
	}

	async function emailSignIn(event: SubmitEvent) {
		event.preventDefault();
		error = '';
		message = '';
		if (!isSupabaseConfigured()) {
			error = 'Add your Supabase public keys to apps/web/.env before signing in.';
			return;
		}
		loading = true;
		try {
			await signInWithEmail(email.trim());
			message = 'Check your email for a secure sign-in link.';
		} catch (cause) {
			error = cause instanceof Error ? cause.message : 'We could not send the sign-in link.';
		} finally {
			loading = false;
		}
	}

	function finishSwipe(end: number) {
		const distance = end - touchStart;
		if (distance < -50) current = Math.min(features.length - 1, current + 1);
		if (distance > 50) current = Math.max(0, current - 1);
	}
</script>

<svelte:head>
	<title>Docufill — Fill forms once. Never fill them again.</title>
	<meta
		name="description"
		content="A private personal document agent for guided PDF filling and secure participant workflows."
	/>
</svelte:head>

<main class="min-h-screen">
	<header class="mx-auto flex max-w-6xl items-center justify-between px-5 py-5 sm:px-8">
		<BrandMark size="sm" />
		{#if signedIn}
			<a
				href={resolve('/documents')}
				class="hidden min-h-11 items-center gap-2 rounded-control px-4 py-2.5 text-sm font-bold text-ink sm:inline-flex"
			>
				My documents <ArrowRight size={16} />
			</a>
		{/if}
	</header>

	<section
		class="mx-auto grid max-w-6xl items-center gap-12 px-5 pt-5 pb-12 sm:px-8 lg:grid-cols-[1.05fr_0.95fr] lg:gap-18 lg:py-18"
	>
		<div class="max-w-xl">
			<div class:logo-drop={animateLogo} class="mb-8 inline-flex">
				<BrandMark size="lg" showWordmark={false} />
			</div>
			<p class="eyebrow mb-3">Your personal document agent</p>
			<h1
				class="text-4xl leading-[1.05] font-semibold tracking-tight text-balance text-ink sm:text-5xl"
			>
				Fill forms once.
				<span class="text-ink-muted">Never fill them again.</span>
			</h1>
			<p class="mt-6 max-w-lg text-lg leading-8 text-balance text-ink-muted">
				Upload a digital PDF. Docufill finds the questions, reuses your confirmed details, and asks
				only what is missing.
			</p>
			<div class="mt-7 flex flex-wrap gap-x-5 gap-y-3 text-sm font-semibold text-ink-muted">
				<span class="flex items-center gap-2"
					><Check size={16} class="text-positive" /> Source-aware answers</span
				>
				<span class="flex items-center gap-2"
					><LockKeyhole size={16} class="text-positive" /> Private by default</span
				>
			</div>
		</div>

		<div class="surface overflow-hidden p-3 sm:p-4">
			<div
				role="region"
				aria-label="Docufill features"
				ontouchstart={(event) => (touchStart = event.touches[0]?.clientX ?? 0)}
				ontouchend={(event) => finishSwipe(event.changedTouches[0]?.clientX ?? touchStart)}
			>
				<FeatureIllustration feature={features[current].key} />
				<div class="px-3 pt-6 pb-3 sm:px-5">
					<p class="mb-1.5 text-xs text-ink-muted">
						{current + 1} of {features.length}
					</p>
					<h2 class="text-xl font-semibold tracking-tight text-ink">
						{features[current].title}
					</h2>
					<p class="mt-2 min-h-14 text-sm leading-6 text-ink-muted">{features[current].body}</p>
					<div class="mt-5 flex items-center justify-between">
						<div class="flex gap-1.5" aria-label="Feature progress">
							{#each features as feature, index (feature.key)}
								<button
									type="button"
									onclick={() => (current = index)}
									class="h-2 rounded-full transition-all {index === current
										? 'w-7 bg-brand'
										: 'w-2 bg-line'}"
									aria-label={`Show feature ${index + 1}`}
									aria-current={index === current ? 'step' : undefined}
								></button>
							{/each}
						</div>
						{#if current < features.length - 1}
							<div class="flex items-center gap-1">
								<Button variant="ghost" onclick={() => (current = features.length - 1)}>Skip</Button
								>
								<Button onclick={() => (current += 1)}>Next <ArrowRight size={16} /></Button>
							</div>
						{:else}
							<Button onclick={() => document.getElementById('sign-in')?.scrollIntoView()}>
								Get started <ArrowRight size={16} />
							</Button>
						{/if}
					</div>
				</div>
			</div>
		</div>
	</section>

	<section id="sign-in" class="mx-auto max-w-6xl px-5 pb-14 sm:px-8">
		<div class="surface grid gap-7 p-6 sm:p-8 lg:grid-cols-[1fr_1.05fr] lg:items-center">
			<div>
				<h2 class="text-2xl font-semibold tracking-tight text-ink">Your profile grows with you.</h2>
				<p class="mt-3 max-w-md text-sm leading-6 text-ink-muted">
					We begin with your verified name and email. No long onboarding form and no speculative
					personal data collection.
				</p>
			</div>
			<div class="rounded-control border border-line bg-canvas p-4 sm:p-5">
				{#if signedIn}
					<Button class="w-full" onclick={() => goto(resolve('/documents'))}>
						Continue to my documents <ArrowRight size={17} />
					</Button>
				{:else}
					<Button class="w-full" {loading} onclick={googleSignIn}>
						<span
							class="grid size-5 place-items-center rounded-full bg-white font-black text-[#4285f4]"
							>G</span
						>
						Continue with Google
					</Button>
					<div
						class="my-4 flex items-center gap-3 text-[11px] font-bold tracking-wider text-ink-muted uppercase"
					>
						<span class="h-px flex-1 bg-line"></span> or use email
						<span class="h-px flex-1 bg-line"></span>
					</div>
					<form class="flex flex-col gap-3 sm:flex-row" onsubmit={emailSignIn}>
						<label class="sr-only" for="email">Email address</label>
						<input
							id="email"
							type="email"
							required
							autocomplete="email"
							bind:value={email}
							placeholder="you@example.com"
							class="min-h-11 flex-1 rounded-control border-line bg-surface-raised text-sm text-ink placeholder:text-ink-muted/70"
						/>
						<Button type="submit" variant="secondary" {loading}>Email me a link</Button>
					</form>
				{/if}
				{#if message}<p class="mt-3 text-sm font-semibold text-positive" role="status">
						{message}
					</p>{/if}
				{#if error}<p class="mt-3 text-sm font-semibold text-negative" role="alert">{error}</p>{/if}
				<p class="mt-4 text-xs leading-5 text-ink-muted">
					By continuing, you acknowledge our <a
						href={resolve('/privacy')}
						class="font-semibold text-brand-strong underline underline-offset-2">privacy notice</a
					>
					and agree to use Docufill only for
					<a
						href={resolve('/acceptable-use')}
						class="font-semibold text-brand-strong underline underline-offset-2"
						>supported, non-regulated documents</a
					>.
				</p>
			</div>
		</div>
	</section>
</main>
