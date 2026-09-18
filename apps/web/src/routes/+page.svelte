<script lang="ts">
	import {
		AppWindow,
		ArrowRight,
		CheckCircle2,
		ChevronDown,
		Compass,
		Download,
		FileCheck,
		FileSpreadsheet,
		Globe,
		LockKeyhole,
		Layers,
		ShieldCheck,
		Sparkles,
		UserCheck,
		Zap
	} from '@lucide/svelte';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { onMount } from 'svelte';
	import gsap from 'gsap';
	import { currentSession, signInWithEmail, signInWithGoogle } from '$lib/auth';
	import BrandMark from '$lib/components/BrandMark.svelte';
	import Button from '$lib/components/Button.svelte';
	import { isSupabaseConfigured } from '$lib/supabase';

	let signedIn = $state(false);
	let email = $state('');
	let loading = $state(false);
	let message = $state('');
	let error = $state('');

	// Interactive Simulator State
	let activeDocType = $state<'tenancy' | 'employment' | 'guarantor'>('tenancy');
	let isSimulating = $state(false);

	const docTypeData = {
		tenancy: [
			{ label: 'Prospective Tenant Name', value: 'Victor Jonah', source: 'Profile Vault', confidence: '100%' },
			{ label: 'Phone Number', value: '+234 802 345 6789', source: 'Profile Vault', confidence: '100%' },
			{ label: 'Current Address', value: '14 Admiralty Way, Lekki Phase 1, Lagos', source: 'Profile Vault', confidence: '100%' },
			{ label: 'Monthly Income', value: '₦1,850,000', source: 'Confirmed Fact', confidence: '99%' }
		],
		employment: [
			{ label: 'Employee Full Name', value: 'Victor Jonah', source: 'Profile Vault', confidence: '100%' },
			{ label: 'Official Email', value: 'victor@docufill.app', source: 'Profile Vault', confidence: '100%' },
			{ label: 'Job Role / Designation', value: 'Principal Software Engineer', source: 'Confirmed Fact', confidence: '96%' },
			{ label: 'Date of Birth', value: '14th October 1994', source: 'Profile Vault', confidence: '100%' }
		],
		guarantor: [
			{ label: 'Guarantor Full Name', value: 'Dr. Emmanuel Adebayo', source: 'Invited Participant', confidence: 'Assigned' },
			{ label: 'Guarantor Phone', value: '+234 803 999 1234', source: 'Invited Participant', confidence: 'Assigned' },
			{ label: 'Relationship', value: 'Senior Mentor / Supervisor', source: 'Invited Participant', confidence: 'Assigned' }
		]
	};

	let simulatedFields = $state(docTypeData.tenancy);

	// FAQ State
	let openFaq = $state<number | null>(0);

	const faqs = [
		{
			q: 'How does the Docufill Browser Extension work?',
			a: 'The extension sits in your browser and automatically detects form inputs when you visit tenancy portals, job applications, or banking forms. With one click, it matches fields to your encrypted profile vault and fills them safely without storing unencrypted data on external servers.'
		},
		{
			q: 'How does Docufill keep my personal documents safe?',
			a: 'All facts, answers, text context, signatures, and document excerpts are encrypted using AES-256-GCM before reaching database or cloud storage. Storage buckets are private with short 5-minute pre-signed download URLs.'
		},
		{
			q: 'Does Docufill use my private data to train AI models?',
			a: 'Never. Customer content is strictly excluded from model training. Our AI mapping engine runs with zero retention policies and receives at most 6 relevant profile facts per question, never your full document vault.'
		},
		{
			q: 'How do guarantor and participant invitations work?',
			a: 'You can assign specific fields (like Guarantor signature or income verification) to third parties. They receive a secure 256-bit link and 6-digit OTP code sent to their email. They only see and answer fields assigned to them.'
		},
		{
			q: 'What document formats are supported?',
			a: 'Docufill native engine handles digital PDF documents up to 25MB and 100 pages. It automatically detects AcroForm interactive fields as well as flat text field locations.'
		}
	];

	// Element references for GSAP lazy scrolling
	let heroContainerEl: HTMLElement;
	let extensionSectionEl: HTMLElement;
	let useCasesSectionEl: HTMLElement;
	let securitySectionEl: HTMLElement;

	onMount(() => {
		if (isSupabaseConfigured()) {
			currentSession()
				.then((session) => {
					signedIn = Boolean(session);
				})
				.catch(() => null);
		}

		// GSAP Hero Entrance Timeline
		const ctx = gsap.context(() => {
			gsap.from('#hero-content > *', {
				opacity: 0,
				y: 35,
				duration: 0.8,
				stagger: 0.15,
				ease: 'power3.out'
			});

			gsap.from('#hero-card', {
				opacity: 0,
				scale: 0.94,
				y: 45,
				duration: 1,
				delay: 0.4,
				ease: 'power3.out'
			});

			// Floating animation for hero card
			gsap.to('#hero-card', {
				y: '-=12',
				duration: 3.5,
				repeat: -1,
				yoyo: true,
				ease: 'sine.inOut'
			});

			// Lazy Scroll Observer for Lazy Reveals
			const observerOptions = { threshold: 0.15 };
			const lazyObserver = new IntersectionObserver((entries) => {
				entries.forEach((entry) => {
					if (entry.isIntersecting) {
						gsap.fromTo(
							entry.target,
							{ opacity: 0, y: 40 },
							{ opacity: 1, y: 0, duration: 0.8, ease: 'power3.out' }
						);
						lazyObserver.unobserve(entry.target);
					}
				});
			}, observerOptions);

			document.querySelectorAll('.lazy-reveal').forEach((el) => lazyObserver.observe(el));
		});

		return () => ctx.revert();
	});

	async function handleGoogleSignIn() {
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

	async function handleEmailSignIn(event: SubmitEvent) {
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

	function switchDocType(type: 'tenancy' | 'employment' | 'guarantor') {
		activeDocType = type;
		isSimulating = true;

		gsap.fromTo(
			'#sim-container',
			{ opacity: 0.4, y: 8 },
			{ opacity: 1, y: 0, duration: 0.45, ease: 'power2.out', onComplete: () => (isSimulating = false) }
		);

		simulatedFields = docTypeData[type];
	}

	function toggleFaq(index: number) {
		openFaq = openFaq === index ? null : index;
	}

	// 3D Tilt Effect on mousemove
	function handleCardMouseMove(e: MouseEvent, card: HTMLElement) {
		const rect = card.getBoundingClientRect();
		const x = e.clientX - rect.left - rect.width / 2;
		const y = e.clientY - rect.top - rect.height / 2;
		gsap.to(card, {
			rotationY: x / 30,
			rotationX: -y / 30,
			transformPerspective: 1000,
			duration: 0.3,
			ease: 'power1.out'
		});
	}

	function handleCardMouseLeave(card: HTMLElement) {
		gsap.to(card, { rotationY: 0, rotationX: 0, duration: 0.5, ease: 'power2.out' });
	}
</script>

<svelte:head>
	<title>Docufill — Privacy-First Document & Web Form Agent</title>
	<meta
		name="description"
		content="Fill forms once. Never fill them again. PDF processing, browser extension auto-fill, and zero-trust security."
	/>
</svelte:head>

<div class="min-h-screen bg-canvas text-ink font-sans overflow-x-hidden selection:bg-brand selection:text-black">
	<!-- Glassmorphic White Navigation Bar -->
	<header class="sticky top-0 z-50 border-b border-line bg-white/90 backdrop-blur-2xl transition-all">
		<div class="mx-auto flex h-20 max-w-7xl items-center justify-between px-5 sm:px-8">
			<a href={resolve('/')} class="flex items-center gap-3 group" aria-label="Docufill Home">
				<BrandMark size="sm" />
			</a>

			<!-- Center Links -->
			<nav class="hidden md:flex items-center gap-1 rounded-full border border-line bg-surface-raised px-4 py-1.5 text-sm font-bold">
				<a href="#extension" class="rounded-full px-4 py-1.5 text-ink-muted transition hover:text-ink hover:bg-white">Extension</a>
				<a href="#use-cases" class="rounded-full px-4 py-1.5 text-ink-muted transition hover:text-ink hover:bg-white">Use Cases</a>
				<a href="#demo" class="rounded-full px-4 py-1.5 text-ink-muted transition hover:text-ink hover:bg-white">Live Simulator</a>
				<a href="#security" class="rounded-full px-4 py-1.5 text-ink-muted transition hover:text-ink hover:bg-white">Security</a>
				<a href="#faq" class="rounded-full px-4 py-1.5 text-ink-muted transition hover:text-ink hover:bg-white">FAQ</a>
			</nav>

			<!-- Right Google Auth Button -->
			<div class="flex items-center gap-3">
				{#if signedIn}
					<a
						href={resolve('/documents')}
						class="inline-flex min-h-11 items-center gap-2 rounded-control bg-brand px-5 py-2.5 text-sm font-extrabold text-black shadow-sm transition hover:bg-brand-strong active:scale-98"
					>
						My documents <ArrowRight size={17} />
					</a>
				{:else}
					<button
						type="button"
						onclick={handleGoogleSignIn}
						disabled={loading}
						class="inline-flex min-h-11 items-center gap-2.5 rounded-control bg-brand px-4.5 py-2 text-sm font-extrabold text-black shadow-sm transition hover:bg-brand-strong active:scale-98 cursor-pointer"
					>
						<svg class="size-4.5" viewBox="0 0 24 24">
							<path
								fill="#000000"
								d="M22.56 12.25c0-.78-.07-1.53-.2-2.25H12v4.26h5.92c-.26 1.37-1.04 2.53-2.21 3.31v2.77h3.57c2.08-1.92 3.28-4.74 3.28-8.09z"
							/>
							<path
								fill="#000000"
								d="M12 23c2.97 0 5.46-.98 7.28-2.66l-3.57-2.77c-.98.66-2.23 1.06-3.71 1.06-2.86 0-5.29-1.93-6.16-4.53H2.18v2.84C3.99 20.53 7.7 23 12 23z"
							/>
							<path
								fill="#000000"
								d="M5.84 14.09c-.22-.66-.35-1.36-.35-2.09s.13-1.43.35-2.09V7.06H2.18C1.43 8.55 1 10.22 1 12s.43 3.45 1.18 4.94l2.85-2.22.81-.63z"
							/>
							<path
								fill="#000000"
								d="M12 5.38c1.62 0 3.06.56 4.21 1.64l3.15-3.15C17.45 2.09 14.97 1 12 1 7.7 1 3.99 3.47 2.18 7.06l3.66 2.84c.87-2.6 3.3-4.52 6.16-4.52z"
							/>
						</svg>
						<span>Sign in with Google</span>
					</button>
				{/if}
			</div>
		</div>
	</header>

	<!-- Centered Hero Section -->
	<section bind:this={heroContainerEl} class="relative mx-auto max-w-5xl px-5 pt-16 pb-24 text-center sm:px-8 lg:pt-24 lg:pb-32">
		<!-- Background Soft Glow -->
		<div class="pointer-events-none absolute top-1/3 left-1/2 -translate-x-1/2 size-96 rounded-full bg-brand/20 blur-3xl"></div>

		<div id="hero-content" class="flex flex-col items-center">
			<!-- Badge -->
			<div class="mb-6 inline-flex items-center gap-2 rounded-full border border-brand/40 bg-brand-soft px-4 py-1.5 text-xs font-black tracking-widest text-ink uppercase">
				<Sparkles size={14} class="text-ink" />
				Privacy-First Personal Document Agent
			</div>
			<!-- Centered Hero Title with Rebond Grotesque & LockSerif Accent -->
			<h1 class="text-4xl sm:text-6xl lg:text-7xl font-extrabold tracking-tight text-ink leading-[1.06] max-w-4xl text-balance">
				Fill forms once. <br />
				<span class="font-display italic font-normal text-ink-muted">Never fill them again.</span>
			</h1>

			<!-- Centered Subtitle -->
			<p class="mt-7 max-w-2xl text-lg sm:text-xl leading-relaxed text-ink-muted font-medium text-balance">
				Upload any PDF form or auto-fill web applications in 1-click. Docufill maps your confirmed details from your encrypted vault and leaves only missing fields for review.
			</p>

			<!-- Centered Hero CTA Action Buttons -->
			<div class="mt-10 flex flex-wrap items-center justify-center gap-4">
				{#if !signedIn}
					<button
						type="button"
						onclick={handleGoogleSignIn}
						disabled={loading}
						class="inline-flex min-h-13 items-center gap-3 rounded-control bg-brand px-7 py-3.5 text-base font-extrabold text-black shadow-md transition hover:bg-brand-strong active:scale-98 cursor-pointer"
					>
						<svg class="size-5" viewBox="0 0 24 24">
							<path
								fill="#000000"
								d="M22.56 12.25c0-.78-.07-1.53-.2-2.25H12v4.26h5.92c-.26 1.37-1.04 2.53-2.21 3.31v2.77h3.57c2.08-1.92 3.28-4.74 3.28-8.09z"
							/>
							<path
								fill="#000000"
								d="M12 23c2.97 0 5.46-.98 7.28-2.66l-3.57-2.77c-.98.66-2.23 1.06-3.71 1.06-2.86 0-5.29-1.93-6.16-4.53H2.18v2.84C3.99 20.53 7.7 23 12 23z"
							/>
							<path
								fill="#000000"
								d="M5.84 14.09c-.22-.66-.35-1.36-.35-2.09s.13-1.43.35-2.09V7.06H2.18C1.43 8.55 1 10.22 1 12s.43 3.45 1.18 4.94l2.85-2.22.81-.63z"
							/>
							<path
								fill="#000000"
								d="M12 5.38c1.62 0 3.06.56 4.21 1.64l3.15-3.15C17.45 2.09 14.97 1 12 1 7.7 1 3.99 3.47 2.18 7.06l3.66 2.84c.87-2.6 3.3-4.52 6.16-4.52z"
							/>
						</svg>
						<span>Start Free with Google</span>
						<ArrowRight size={18} />
					</button>
				{:else}
					<a
						href={resolve('/documents')}
						class="inline-flex min-h-13 items-center gap-3 rounded-control bg-brand px-7 py-3.5 text-base font-extrabold text-black shadow-md transition hover:bg-brand-strong active:scale-98"
					>
						<span>Go to My Documents</span>
						<ArrowRight size={18} />
					</a>
				{/if}

				<a
					href="#extension"
					class="inline-flex min-h-13 items-center gap-2.5 rounded-control border border-line bg-white px-6 py-3.5 text-sm font-bold text-ink transition hover:bg-surface-raised"
				>
					<AppWindow size={18} class="text-ink" />
					<span>Browser Extension</span>
				</a>
			</div>

			<!-- Trust Badges Bar -->
			<div class="mt-12 flex flex-wrap items-center justify-center gap-8 border-t border-line pt-8 text-xs font-bold text-ink-muted">
				<span class="flex items-center gap-2">
					<LockKeyhole size={15} class="text-positive" /> AES-256-GCM Encrypted
				</span>
				<span class="flex items-center gap-2">
					<ShieldCheck size={15} class="text-positive" /> Zero Model Training
				</span>
				<span class="flex items-center gap-2">
					<FileCheck size={15} class="text-positive" /> Native PDFium Engine
				</span>
			</div>
		<!-- Centered Floating Preview Card -->
		<div id="hero-card" class="mt-16 mx-auto max-w-3xl surface overflow-hidden p-6 sm:p-8 border border-line shadow-card backdrop-blur-2xl text-left bg-white">
			<div class="flex items-center justify-between border-b border-line pb-4 mb-6">
				<div class="flex items-center gap-3">
					<div class="grid size-11 place-items-center rounded-control bg-brand-soft text-ink font-black text-sm">
						PDF
					</div>
					<div>
						<h3 class="text-base font-bold text-ink">Tenancy_Agreement_Application.pdf</h3>
						<p class="text-xs text-ink-muted">14 Fields Detected • 3 Auto-Filled from Vault</p>
					</div>
				</div>
				<span class="rounded-full bg-brand-soft px-3.5 py-1 text-xs font-bold text-ink border border-brand/40">
					100% Grounded
				</span>
			</div>

			<!-- Sample Auto-Filled Fields List -->
			<div class="space-y-3.5">
				<div class="rounded-control border border-line bg-surface-raised p-4 transition hover:border-brand-strong">
					<div class="flex items-center justify-between text-xs text-ink-muted mb-1">
						<span>Applicant Full Name</span>
						<span class="text-positive font-bold flex items-center gap-1"><CheckCircle2 size={13} /> Vault Fact</span>
					</div>
					<p class="text-base font-bold text-ink">Victor Jonah</p>
				</div>

				<div class="rounded-control border border-line bg-surface-raised p-4 transition hover:border-brand-strong">
					<div class="flex items-center justify-between text-xs text-ink-muted mb-1">
						<span>Telephone Contact</span>
						<span class="text-positive font-bold flex items-center gap-1"><CheckCircle2 size={12} /> Vault Fact</span>
					</div>
					<p class="text-base font-bold text-ink">+234 802 345 6789</p>
				</div>

				<div class="rounded-control border border-brand-strong/40 bg-brand-soft p-4">
					<div class="flex items-center justify-between text-xs text-ink font-bold mb-1">
						<span>Guarantor Field</span>
						<span class="rounded bg-brand/30 px-2 py-0.5 text-[10px] text-ink font-bold">Participant Assigned</span>
					</div>
					<p class="text-xs text-ink-muted italic">Assigned to Dr. Emmanuel Adebayo via 6-digit OTP</p>
				</div>
			</div>
		</div>
	</section>

	<!-- Dedicated Chrome / Browser Extension Spotlight Section -->
	<section id="extension" bind:this={extensionSectionEl} class="lazy-reveal border-t border-line bg-surface-raised/60 py-24">
		<div class="mx-auto max-w-7xl px-5 sm:px-8">
			<div class="grid items-center gap-12 lg:grid-cols-2 lg:gap-16">
				<!-- Left Text Description -->
				<div>
					<div class="mb-4 inline-flex items-center gap-2 rounded-full border border-brand/40 bg-brand-soft px-3.5 py-1 text-xs font-bold text-ink uppercase tracking-wider">
						<AppWindow size={14} class="text-ink" />
						Browser Extension
					</div>
					<h2 class="text-3xl sm:text-5xl font-extrabold text-ink tracking-tight leading-[1.1]">
						Auto-fill forms directly on any website.
					</h2>
					<p class="mt-5 text-base sm:text-lg text-ink-muted leading-relaxed font-medium">
						Stop retyping your address, employer info, and references on every portal. The Docufill Chrome extension detects inputs on web pages and populates them instantly from your encrypted vault.
					</p>

					<!-- Features Bullets -->
					<div class="mt-8 space-y-4 text-sm font-semibold text-ink">
						<div class="flex items-start gap-3">
							<div class="mt-0.5 grid size-6 place-items-center rounded-full bg-brand text-black font-black text-xs">✓</div>
							<div>
								<h4 class="font-bold text-ink">1-Click Web Form Auto-Fill</h4>
								<p class="text-xs text-ink-muted mt-0.5">Detects form inputs on tenancy portals, job application, and bank websites.</p>
							</div>
						</div>

						<div class="flex items-start gap-3">
							<div class="mt-0.5 grid size-6 place-items-center rounded-full bg-brand text-black font-black text-xs">✓</div>
							<div>
								<h4 class="font-bold text-ink">Side Panel Vault Access</h4>
								<p class="text-xs text-ink-muted mt-0.5">Manage your confirmed facts and generate single-use participant links right in your browser side panel.</p>
							</div>
						</div>

						<div class="flex items-start gap-3">
							<div class="mt-0.5 grid size-6 place-items-center rounded-full bg-brand text-black font-black text-xs">✓</div>
							<div>
								<h4 class="font-bold text-ink">Zero Server Exposure</h4>
								<p class="text-xs text-ink-muted mt-0.5">Data is decrypted locally in your browser session. Unencrypted values are never sent to external servers.</p>
							</div>
						</div>
					</div>

					<div class="mt-9">
						<button
							type="button"
							onclick={handleGoogleSignIn}
							class="inline-flex min-h-12 items-center gap-3 rounded-control bg-brand px-6 py-3 text-sm font-extrabold text-black transition hover:bg-brand-strong active:scale-98 cursor-pointer"
						>
							<AppWindow size={18} />
							<span>Get Extension with Google</span>
							<ArrowRight size={16} />
						</button>
					</div>
				</div>

				<!-- Right Extension Mockup Box -->
				<div class="surface p-6 sm:p-8 border border-line shadow-card bg-white relative overflow-hidden">
					<div class="flex items-center justify-between border-b border-line pb-4 mb-6">
						<div class="flex items-center gap-2">
							<span class="size-3 rounded-full bg-red-400"></span>
							<span class="size-3 rounded-full bg-yellow-400"></span>
							<span class="size-3 rounded-full bg-green-400"></span>
							<span class="ml-2 text-xs text-ink-muted font-mono">tenancy-portal.com/apply</span>
						</div>
						<span class="rounded bg-brand-soft px-2 py-0.5 text-[11px] font-bold text-ink">Extension Active</span>
					</div>

					<!-- Extension Popup Simulator Overlay -->
					<div class="rounded-control border border-line bg-surface-raised p-5 shadow-md">
						<div class="flex items-center justify-between border-b border-line pb-3 mb-4">
							<div class="flex items-center gap-2">
								<BrandMark size="sm" showWordmark={false} />
								<span class="text-xs font-bold text-ink">Docufill Assistant</span>
							</div>
							<span class="text-[11px] font-bold text-ink">4 Fields Found</span>
						</div>

						<div class="space-y-2.5 text-xs">
							<div class="flex items-center justify-between rounded bg-white p-2.5 border border-line">
								<span class="text-ink-muted">Full Name</span>
								<span class="font-bold text-ink">Victor Jonah</span>
							</div>
							<div class="flex items-center justify-between rounded bg-white p-2.5 border border-line">
								<span class="text-ink-muted">Phone Number</span>
								<span class="font-bold text-ink">+234 802 345 6789</span>
							</div>
							<div class="flex items-center justify-between rounded bg-white p-2.5 border border-line">
								<span class="text-ink-muted">Email Address</span>
								<span class="font-bold text-ink">victor@docufill.app</span>
							</div>
						</div>

						<button
							type="button"
							class="mt-4 w-full rounded-control bg-brand py-2.5 text-xs font-extrabold text-black transition hover:bg-brand-strong"
						>
							⚡ Auto-Fill Web Form (1-Click)
						</button>
					</div>
				</div>
			</div>
		</div>
	</section>

	<!-- 4 Core Use Cases Grid Section -->
	<section id="use-cases" bind:this={useCasesSectionEl} class="lazy-reveal py-24 mx-auto max-w-7xl px-5 sm:px-8">
		<div class="text-center max-w-3xl mx-auto mb-16">
			<p class="eyebrow mb-2">Built for Real Life</p>
			<h2 class="text-3xl sm:text-5xl font-extrabold text-ink tracking-tight">
				Where Docufill saves you hours.
			</h2>
			<p class="mt-4 text-base sm:text-lg text-ink-muted font-medium">
				From tenant onboarding to guarantor signatures, Docufill handles repetitive forms seamlessly.
			</p>
		</div>

		<div class="grid gap-6 sm:grid-cols-2 lg:grid-cols-4">
			<!-- Use Case 1 -->
			<div class="surface p-7 border border-line hover:border-brand-strong transition bg-white">
				<div class="grid size-12 place-items-center rounded-control bg-brand-soft text-ink mb-5">
					<FileSpreadsheet size={22} />
				</div>
				<h3 class="text-lg font-bold text-ink mb-2">Residential Tenancy</h3>
				<p class="text-sm leading-relaxed text-ink-muted">
					Complete landlord applications, lease agreements, and reference forms with confirmed personal facts.
				</p>
			</div>

			<!-- Use Case 2 -->
			<div class="surface p-7 border border-line hover:border-brand-strong transition bg-white">
				<div class="grid size-12 place-items-center rounded-control bg-brand-soft text-ink mb-5">
					<UserCheck size={22} />
				</div>
				<h3 class="text-lg font-bold text-ink mb-2">Job Onboarding</h3>
				<p class="text-sm leading-relaxed text-ink-muted">
					Fill employment verification, emergency contacts, tax declarations, and direct deposit details.
				</p>
			</div>

			<!-- Use Case 3 -->
			<div class="surface p-7 border border-line hover:border-brand-strong transition bg-white">
				<div class="grid size-12 place-items-center rounded-control bg-brand-soft text-ink mb-5">
					<LockKeyhole size={22} />
				</div>
				<h3 class="text-lg font-bold text-ink mb-2">Financial Disclosures</h3>
				<p class="text-sm leading-relaxed text-ink-muted">
					Fill mortgage applications and bank questionnaires without risking unencrypted credential exposure.
				</p>
			</div>

			<!-- Use Case 4 -->
			<div class="surface p-7 border border-line hover:border-brand-strong transition bg-white">
				<div class="grid size-12 place-items-center rounded-control bg-brand-soft text-ink mb-5">
					<Globe size={22} />
				</div>
				<h3 class="text-lg font-bold text-ink mb-2">Visa & Travel Approvals</h3>
				<p class="text-sm leading-relaxed text-ink-muted">
					Populate embassy forms and travel declarations with zero identity hallucination.
				</p>
			</div>
		</div>
	</section>

	<!-- Interactive Live Simulator Section -->
	<section id="demo" class="lazy-reveal border-t border-line bg-surface-raised/50 py-24">
		<div class="mx-auto max-w-7xl px-5 sm:px-8">
			<div class="text-center max-w-3xl mx-auto mb-14">
				<p class="eyebrow mb-2">Interactive Demo</p>
				<h2 class="text-3xl sm:text-5xl font-extrabold text-ink tracking-tight">
					Try the live auto-fill simulator.
				</h2>
				<p class="mt-4 text-base sm:text-lg text-ink-muted font-medium">
					Select a document template below to test how confirmed details populate fields automatically.
				</p>
			</div>

			<!-- Document Type Selector Tabs -->
			<div class="flex flex-wrap items-center justify-center gap-3 mb-10">
				<button
					type="button"
					onclick={() => switchDocType('tenancy')}
					class="flex items-center gap-2 rounded-control px-5 py-3 text-sm font-bold transition cursor-pointer {activeDocType === 'tenancy' ? 'bg-brand text-black shadow-sm font-extrabold' : 'bg-white border border-line text-ink-muted hover:text-ink'}"
				>
					<FileSpreadsheet size={18} /> Residential Tenancy
				</button>

				<button
					type="button"
					onclick={() => switchDocType('employment')}
					class="flex items-center gap-2 rounded-control px-5 py-3 text-sm font-bold transition cursor-pointer {activeDocType === 'employment' ? 'bg-brand text-black shadow-sm font-extrabold' : 'bg-white border border-line text-ink-muted hover:text-ink'}"
				>
					<UserCheck size={18} /> Employment Verification
				</button>

				<button
					type="button"
					onclick={() => switchDocType('guarantor')}
					class="flex items-center gap-2 rounded-control px-5 py-3 text-sm font-bold transition cursor-pointer {activeDocType === 'guarantor' ? 'bg-brand text-black shadow-sm font-extrabold' : 'bg-white border border-line text-ink-muted hover:text-ink'}"
				>
					<Zap size={18} /> Guarantor Assignment
				</button>
			</div>

			<!-- Interactive Display Container -->
			<div id="sim-container" class="surface max-w-4xl mx-auto p-6 sm:p-10 border border-line shadow-card bg-white">
				<div class="flex items-center justify-between border-b border-line pb-4 mb-6">
					<div>
						<span class="text-xs font-bold text-ink-muted uppercase">Template Document</span>
						<h3 class="text-xl font-extrabold text-ink capitalize">{activeDocType} Form Template</h3>
					</div>
					<div class="flex items-center gap-2">
						<span class="size-2.5 rounded-full bg-positive animate-pulse"></span>
						<span class="text-xs font-bold text-positive">Auto-Fill Active</span>
					</div>
				</div>

				<div class="grid gap-4 sm:grid-cols-2">
					{#each simulatedFields as item (item.label)}
						<div class="rounded-control border border-line bg-surface-raised p-4 transition hover:border-brand-strong">
							<div class="flex items-center justify-between text-xs text-ink-muted mb-1.5">
								<span class="font-bold">{item.label}</span>
								<span class="rounded bg-brand-soft px-2 py-0.5 text-[11px] font-bold text-ink">{item.confidence}</span>
							</div>
							<p class="text-base font-bold text-ink">{item.value}</p>
							<div class="mt-2 text-[11px] text-ink-muted flex items-center gap-1">
								<ShieldCheck size={13} class="text-positive" /> Source: {item.source}
							</div>
						</div>
					{/each}
				</div>
			</div>
		</div>
	</section>

	<!-- Zero-Knowledge Security Section -->
	<section id="security" bind:this={securitySectionEl} class="lazy-reveal py-24 mx-auto max-w-7xl px-5 sm:px-8">
		<div class="text-center max-w-3xl mx-auto mb-16">
			<p class="eyebrow mb-2">Zero-Trust Architecture</p>
			<h2 class="text-3xl sm:text-5xl font-extrabold text-ink tracking-tight">
				Your privacy is non-negotiable.
			</h2>
			<p class="mt-4 text-base sm:text-lg text-ink-muted font-medium">
				Every profile fact, answer, signature, and text context is encrypted before saving.
			</p>
		</div>

		<div class="grid gap-6 sm:grid-cols-2 lg:grid-cols-4">
			<div
				role="region"
				aria-label="AES-256 Encryption Security feature"
				onmousemove={(e) => handleCardMouseMove(e, e.currentTarget)}
				onmouseleave={(e) => handleCardMouseLeave(e.currentTarget)}
				class="surface p-7 border border-line transition-transform duration-200 bg-white"
			>
				<div class="grid size-12 place-items-center rounded-control bg-brand-soft text-ink mb-5">
					<LockKeyhole size={24} />
				</div>
				<h3 class="text-lg font-bold text-ink mb-2">AES-256 Encryption</h3>
				<p class="text-sm leading-relaxed text-ink-muted">
					Facts, signatures, answers, and context are encrypted with unique random nonces before saving.
				</p>
			</div>

			<div
				role="region"
				aria-label="Zero LLM Training Privacy feature"
				onmousemove={(e) => handleCardMouseMove(e, e.currentTarget)}
				onmouseleave={(e) => handleCardMouseLeave(e.currentTarget)}
				class="surface p-7 border border-line transition-transform duration-200 bg-white"
			>
				<div class="grid size-12 place-items-center rounded-control bg-brand-soft text-ink mb-5">
					<ShieldCheck size={24} />
				</div>
				<h3 class="text-lg font-bold text-ink mb-2">Zero LLM Training</h3>
				<p class="text-sm leading-relaxed text-ink-muted">
					Customer data is never used to train public models. AI receives minimal grounded facts per question.
				</p>
			</div>

			<div
				role="region"
				aria-label="Scoped Guarantors Collaboration feature"
				onmousemove={(e) => handleCardMouseMove(e, e.currentTarget)}
				onmouseleave={(e) => handleCardMouseLeave(e.currentTarget)}
				class="surface p-7 border border-line transition-transform duration-200 bg-white"
			>
				<div class="grid size-12 place-items-center rounded-control bg-brand-soft text-ink mb-5">
					<UserCheck size={24} />
				</div>
				<h3 class="text-lg font-bold text-ink mb-2">Scoped Guarantors</h3>
				<p class="text-sm leading-relaxed text-ink-muted">
					Participants receive 6-digit OTP verification codes and access only assigned fields for 2 hours.
				</p>
			</div>

			<div
				role="region"
				aria-label="Instant Self-Deletion Data ownership feature"
				onmousemove={(e) => handleCardMouseMove(e, e.currentTarget)}
				onmouseleave={(e) => handleCardMouseLeave(e.currentTarget)}
				class="surface p-7 border border-line transition-transform duration-200 bg-white"
			>
				<div class="grid size-12 place-items-center rounded-control bg-brand-soft text-ink mb-5">
					<FileCheck size={24} />
				</div>
				<h3 class="text-lg font-bold text-ink mb-2">Instant Self-Deletion</h3>
				<p class="text-sm leading-relaxed text-ink-muted">
					Deleting a document or your account immediately purges every original, preview, and vector chunk.
				</p>
			</div>
		</div>
	</section>

	<!-- FAQ Section -->
	<section id="faq" class="lazy-reveal border-t border-line bg-surface-raised/50 py-24">
		<div class="mx-auto max-w-4xl px-5 sm:px-8">
			<div class="text-center mb-14">
				<p class="eyebrow mb-2">Questions & Answers</p>
				<h2 class="text-3xl sm:text-5xl font-extrabold text-ink tracking-tight">
					Frequently asked questions.
				</h2>
			</div>

			<div class="space-y-4">
				{#each faqs as faq, index}
					<div class="surface overflow-hidden border border-line transition bg-white">
						<button
							type="button"
							onclick={() => toggleFaq(index)}
							class="flex w-full items-center justify-between p-6 text-left font-extrabold text-ink text-lg cursor-pointer"
						>
							<span>{faq.q}</span>
							<ChevronDown size={20} class="transition-transform duration-300 {openFaq === index ? 'rotate-180 text-brand-strong' : 'text-ink-muted'}" />
						</button>
						{#if openFaq === index}
							<div class="px-6 pb-6 text-sm leading-relaxed text-ink-muted border-t border-line/50 pt-4 font-medium">
								{faq.a}
							</div>
						{/if}
					</div>
				{/each}
			</div>
		</div>
	</section>

	<!-- Final CTA Sign-In Section -->
	<section class="lazy-reveal py-24 mx-auto max-w-5xl px-5 sm:px-8">
		<div class="surface overflow-hidden p-8 sm:p-14 text-center border border-line bg-gradient-to-b from-white to-brand-soft/30 shadow-card">
			<h2 class="text-3xl sm:text-5xl font-extrabold text-ink tracking-tight">
				Ready to simplify form filling forever?
			</h2>
			<p class="mt-4 text-base sm:text-lg text-ink-muted max-w-2xl mx-auto font-medium">
				Sign in with Google to create your encrypted profile vault and fill your first document in seconds.
			</p>

			<div class="mt-8 flex flex-col items-center justify-center gap-4 max-w-md mx-auto">
				{#if signedIn}
					<Button class="w-full text-base py-3 font-extrabold" onclick={() => goto(resolve('/documents'))}>
						Continue to My Documents <ArrowRight size={18} />
					</Button>
				{:else}
					<button
						type="button"
						onclick={handleGoogleSignIn}
						disabled={loading}
						class="w-full inline-flex min-h-13 items-center justify-center gap-3 rounded-control bg-brand px-6 py-3.5 text-base font-extrabold text-black shadow-sm transition hover:bg-brand-strong cursor-pointer"
					>
						<svg class="size-5" viewBox="0 0 24 24">
							<path
								fill="#000000"
								d="M22.56 12.25c0-.78-.07-1.53-.2-2.25H12v4.26h5.92c-.26 1.37-1.04 2.53-2.21 3.31v2.77h3.57c2.08-1.92 3.28-4.74 3.28-8.09z"
							/>
							<path
								fill="#000000"
								d="M12 23c2.97 0 5.46-.98 7.28-2.66l-3.57-2.77c-.98.66-2.23 1.06-3.71 1.06-2.86 0-5.29-1.93-6.16-4.53H2.18v2.84C3.99 20.53 7.7 23 12 23z"
							/>
							<path
								fill="#000000"
								d="M5.84 14.09c-.22-.66-.35-1.36-.35-2.09s.13-1.43.35-2.09V7.06H2.18C1.43 8.55 1 10.22 1 12s.43 3.45 1.18 4.94l2.85-2.22.81-.63z"
							/>
							<path
								fill="#000000"
								d="M12 5.38c1.62 0 3.06.56 4.21 1.64l3.15-3.15C17.45 2.09 14.97 1 12 1 7.7 1 3.99 3.47 2.18 7.06l3.66 2.84c.87-2.6 3.3-4.52 6.16-4.52z"
							/>
						</svg>
						<span>Continue with Google</span>
					</button>

					<div class="my-2 flex w-full items-center gap-3 text-xs font-bold text-ink-muted uppercase tracking-wider">
						<span class="h-px flex-1 bg-line"></span> or email magic link <span class="h-px flex-1 bg-line"></span>
					</div>

					<form onsubmit={handleEmailSignIn} class="w-full flex gap-2">
						<input
							type="email"
							required
							bind:value={email}
							placeholder="you@example.com"
							class="min-h-11 flex-1 rounded-control border border-line bg-white px-4 text-sm text-ink placeholder:text-ink-muted focus:border-brand-strong"
						/>
						<Button type="submit" variant="secondary" {loading}>Send</Button>
					</form>
				{/if}

				{#if message}<p class="text-xs font-bold text-positive">{message}</p>{/if}
				{#if error}<p class="text-xs font-bold text-negative">{error}</p>{/if}
			</div>
		</div>
	</section>

	<!-- Footer -->
	<footer class="border-t border-line bg-white py-12">
		<div class="mx-auto flex max-w-7xl flex-col items-center justify-between gap-6 px-5 sm:px-8 md:flex-row">
			<BrandMark size="sm" />
			<div class="flex flex-wrap items-center gap-6 text-sm text-ink-muted">
				<a href={resolve('/privacy')} class="hover:text-ink">Privacy Policy</a>
				<a href={resolve('/acceptable-use')} class="hover:text-ink">Acceptable Use</a>
				<span>© {new Date().getFullYear()} Docufill Agent. All rights reserved.</span>
			</div>
		</div>
	</footer>
</div>
