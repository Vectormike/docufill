<script lang="ts">
	import { FilePlus2, Files, LogOut, Settings, UserRound } from '@lucide/svelte';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { page } from '$app/state';
	import { onMount, type Snippet } from 'svelte';
	import { signOut } from '$lib/auth';
	import BrandMark from '$lib/components/BrandMark.svelte';
	import Button from '$lib/components/Button.svelte';
	import InstallPrompt from '$lib/components/InstallPrompt.svelte';
	import { getSupabase, isSupabaseConfigured } from '$lib/supabase';

	let { children }: { children: Snippet } = $props();
	let ready = $state(false);
	let email = $state('');
	let setupError = $state('');

	const navigation = [
		{ href: '/documents', label: 'Documents', icon: Files },
		{ href: '/profile', label: 'My details', icon: UserRound },
		{ href: '/settings', label: 'Settings', icon: Settings }
	] as const;

	onMount(async () => {
		if (!isSupabaseConfigured()) {
			setupError = 'Supabase public keys are missing. Copy apps/web/.env.example to .env.';
			ready = true;
			return;
		}
		const { data } = await getSupabase().auth.getSession();
		if (!data.session) {
			await goto(resolve('/'), { replaceState: true });
			return;
		}
		email = data.session.user.email ?? '';
		ready = true;
	});

	async function logout() {
		await signOut();
		await goto(resolve('/'), { replaceState: true });
	}
</script>

{#if !ready}
	<main class="grid min-h-screen place-items-center">
		<div
			class="size-9 animate-spin rounded-full border-3 border-brand border-r-transparent"
			aria-label="Loading"
		></div>
	</main>
{:else if setupError}
	<main class="grid min-h-screen place-items-center px-5">
		<section class="surface max-w-lg p-8 text-center">
			<BrandMark />
			<h1 class="mt-6 text-2xl font-extrabold text-ink">Connect Docufill</h1>
			<p class="mt-3 text-sm leading-6 text-ink-muted">{setupError}</p>
			<a
				href={resolve('/')}
				class="mt-6 inline-flex min-h-11 items-center font-bold text-brand-strong">Return home</a
			>
		</section>
	</main>
{:else}
	<div class="min-h-dvh pb-24 lg:pb-0">
		<header class="sticky top-0 z-30 border-b border-line bg-canvas/90 backdrop-blur-xl">
			<div class="mx-auto flex h-18 max-w-7xl items-center justify-between px-5 sm:px-8">
				<a href={resolve('/documents')} aria-label="Docufill documents"><BrandMark size="sm" /></a>
				<nav
					class="hidden items-center gap-1 rounded-xl border border-line bg-surface p-1 lg:flex"
					aria-label="Main navigation"
				>
					{#each navigation as item (item.href)}
						<a
							href={resolve(item.href)}
							class="flex min-h-10 items-center gap-2 rounded-control px-3.5 text-sm font-medium transition {page.url.pathname.startsWith(
								item.href
							)
								? 'bg-canvas text-ink'
								: 'text-ink-muted hover:text-ink'}"
							aria-current={page.url.pathname.startsWith(item.href) ? 'page' : undefined}
						>
							<item.icon size={17} />
							{item.label}
						</a>
					{/each}
				</nav>
				<div class="flex items-center gap-1.5">
					<InstallPrompt />
					<Button
						variant="ghost"
						onclick={logout}
						aria-label={`Sign out ${email}`}
						title="Sign out"
					>
						<LogOut size={18} />
						<span class="hidden xl:inline">Sign out</span>
					</Button>
				</div>
			</div>
		</header>

		{@render children()}

		<div
			class="pointer-events-none fixed inset-x-0 bottom-0 z-30 px-3 lg:hidden"
			style="padding-bottom: max(0.75rem, env(safe-area-inset-bottom))"
		>
			<nav
				class="pointer-events-auto grid grid-cols-4 gap-1 rounded-xl border border-line bg-surface-raised/95 p-1.5 shadow-card backdrop-blur-xl"
				aria-label="Mobile navigation"
			>
				{#each navigation as item (item.href)}
					<a
						href={resolve(item.href)}
						class="flex min-h-13 flex-col items-center justify-center gap-1 rounded-control text-[11px] font-medium {page.url.pathname.startsWith(
							item.href
						)
							? 'bg-canvas text-ink'
							: 'text-ink-muted'}"
						aria-current={page.url.pathname.startsWith(item.href) ? 'page' : undefined}
					>
						<item.icon size={19} />
						{item.label}
					</a>
				{/each}
				<a
					href={resolve('/documents/new')}
					class="flex min-h-13 flex-col items-center justify-center gap-1 rounded-control bg-brand text-[11px] font-semibold text-[#211d14]"
				>
					<FilePlus2 size={19} /> New document
				</a>
			</nav>
		</div>
	</div>
{/if}
