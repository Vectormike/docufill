<script lang="ts">
	import {
		ArrowRight,
		CheckCircle2,
		ChevronDown,
		Upload,
		Sparkles,
		LockKeyhole,
		ShieldCheck,
		FileCheck,
		Briefcase,
		GraduationCap,
		Building2,
		Zap,
		X,
		Mail,
		EyeOff
	} from '@lucide/svelte';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { onMount } from 'svelte';
	import gsap from 'gsap';
	import { ScrollTrigger } from 'gsap/ScrollTrigger';
	import { currentSession, signInWithEmail, signInWithGoogle } from '$lib/auth';
	import BrandMark from '$lib/components/BrandMark.svelte';
	import Button from '$lib/components/Button.svelte';
	import { isSupabaseConfigured } from '$lib/supabase';

	gsap.registerPlugin(ScrollTrigger);

	let signedIn = $state(false);
	let email = $state('');
	let loading = $state(false);
	let message = $state('');
	let error = $state('');

	// Upload trial state
	let isDragging = $state(false);
	let uploadedFile = $state<File | null>(null);
	let trialStep = $state<'idle' | 'processing' | 'result'>('idle');
	let trialProgress = $state(0);

	// FAQ State
	let openFaq = $state<number | null>(0);

	// Compounding progress bar state
	let compoundingProgress = $state([0, 0, 0]);
	let compoundingSectionEl: HTMLElement;

	// Scroll text highlight state
	let scrollSectionEl: HTMLElement;

	const faqs = [
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
			a: 'You can assign specific fields (like Guarantor signature or income verification) to third parties. They receive a secure 256-bit link and 6-digit OTP code sent to their email. They only see and answer fields assigned to them — no account needed.'
		},
		{
			q: 'What document formats are supported?',
			a: 'Docufill handles digital PDF documents up to 25MB and 100 pages. It detects AcroForm fields, flat text blanks, and printed scans of forms. Handwritten-only pages may still need a review.'
		},
		{
			q: 'Does Docufill read my Gmail inbox?',
			a: 'No. We use your Google account for sign-in and profile seeding only — your name, email, and photo. We never access, read, or import anything from your Gmail inbox or Google Drive.'
		}
	];

	// Sample trial data for scripted demo
	const sampleResults = [
		{
			label: 'Applicant Full Name',
			value: 'Chinedu Okonkwo',
			source: 'Gmail Profile',
			filled: true
		},
		{
			label: 'Email Address',
			value: 'chinedu.okonkwo@gmail.com',
			source: 'Gmail Profile',
			filled: true
		},
		{ label: 'Phone Number', value: '+234 802 345 6789', source: 'Tenancy Pack', filled: true },
		{
			label: 'Current Address',
			value: '14 Admiralty Way, Lekki Phase 1, Lagos',
			source: 'Tenancy Pack',
			filled: true
		},
		{ label: 'Monthly Income', value: '₦1,850,000', source: 'Tenancy Pack', filled: true },
		{ label: 'Guarantor Name', value: '', source: 'Sent to Adaeze — email + link', filled: false },
		{ label: 'Next of Kin', value: '', source: 'Needs you', filled: false }
	];

	const HEADLINE =
		'FILL FORMS ONCE AND NEVER FILL AGAIN. EVERY FORM TAKES LESS TIME THAN THE PREVIOUS. NEVER TYPE THE SAME FACT TWICE.';
	const headlineWords = HEADLINE.split(' ');
	const WORD_HIGHLIGHT = '#FACC15';
	const COMPOUNDING_FILL = [30, 75, 95];

	/**
	 * The headline words and the progress bars only reach their readable state
	 * through a tween, so skipping the animations means applying that state
	 * directly. Everything else animates with `from`, so its resting DOM state
	 * is already the finished one.
	 */
	function settleWithoutMotion() {
		compoundingProgress = [...COMPOUNDING_FILL];
		scrollSectionEl?.querySelectorAll<HTMLElement>('.aboutSection__word').forEach((word) => {
			word.style.color = WORD_HIGHLIGHT;
		});
	}

	onMount(() => {
		if (isSupabaseConfigured()) {
			currentSession()
				.then((session) => {
					signedIn = Boolean(session);
				})
				.catch(() => null);
		}

		if (window.matchMedia('(prefers-reduced-motion: reduce)').matches) {
			settleWithoutMotion();
			return;
		}

		const ctx = gsap.context(() => {
			// Hero entrance
			gsap.from('#hero-content > *', {
				opacity: 0,
				y: 35,
				duration: 0.8,
				stagger: 0.15,
				ease: 'power3.out'
			});

			// Pinned Headline ScrollTrigger word highlight animation
			if (scrollSectionEl) {
				const wordEls = scrollSectionEl.querySelectorAll('.aboutSection__word');

				// Wave fade-in from below when section enters viewport
				gsap.fromTo(
					wordEls,
					{ opacity: 0, y: 28 },
					{
						opacity: 1,
						y: 0,
						duration: 0.55,
						ease: 'power3.out',
						stagger: 0.045,
						scrollTrigger: {
							trigger: scrollSectionEl,
							start: 'top 78%',
							once: true
						}
					}
				);

				// Pin section & scrub word color transition to yellow
				const tl = gsap.timeline({
					scrollTrigger: {
						trigger: scrollSectionEl,
						start: 'top top',
						end: `+=${window.innerHeight * 1.8}`,
						pin: true,
						scrub: 0.6,
						anticipatePin: 1,
						invalidateOnRefresh: true
					}
				});

				wordEls.forEach((word, i) => {
					tl.to(
						word,
						{
							color: WORD_HIGHLIGHT,
							duration: 0.3
						},
						i * 0.3
					);
				});
			}

			// Compounding memory section observer for progress bar fill & card fade-in
			if (compoundingSectionEl) {
				const observer = new IntersectionObserver(
					(entries) => {
						entries.forEach((entry) => {
							if (entry.isIntersecting) {
								gsap.fromTo(
									'.compounding-card',
									{ opacity: 0, y: 30 },
									{ opacity: 1, y: 0, duration: 0.7, stagger: 0.2, ease: 'power3.out' }
								);

								// Animate progress bars
								setTimeout(() => (compoundingProgress[0] = COMPOUNDING_FILL[0]), 200);
								setTimeout(() => (compoundingProgress[1] = COMPOUNDING_FILL[1]), 500);
								setTimeout(() => (compoundingProgress[2] = COMPOUNDING_FILL[2]), 800);

								observer.unobserve(entry.target);
							}
						});
					},
					{ threshold: 0.2 }
				);
				observer.observe(compoundingSectionEl);
			}

			// Lazy scroll reveals for general sections
			const observerOptions = { threshold: 0.12 };
			const lazyObserver = new IntersectionObserver((entries) => {
				entries.forEach((entry) => {
					if (entry.isIntersecting) {
						gsap.fromTo(
							entry.target,
							{ opacity: 0, y: 35 },
							{ opacity: 1, y: 0, duration: 0.8, ease: 'power3.out' }
						);
						lazyObserver.unobserve(entry.target);
					}
				});
			}, observerOptions);

			document.querySelectorAll('.lazy-reveal').forEach((el) => lazyObserver.observe(el));
		});

		return () => {
			ctx.revert();
		};
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

	function toggleFaq(index: number) {
		openFaq = openFaq === index ? null : index;
	}

	// Drop zone handlers
	function handleDragOver(e: DragEvent) {
		e.preventDefault();
		isDragging = true;
	}

	function handleDragLeave() {
		isDragging = false;
	}

	function handleDrop(e: DragEvent) {
		e.preventDefault();
		isDragging = false;
		const file = e.dataTransfer?.files[0];
		if (file && file.type === 'application/pdf') {
			startTrial(file);
		}
	}

	function handleFileInput(e: Event) {
		const input = e.target as HTMLInputElement;
		const file = input.files?.[0];
		if (file) {
			startTrial(file);
		}
	}

	function startSampleTrial(sampleName: string) {
		const mockFile = new File([''], `${sampleName}.pdf`, { type: 'application/pdf' });
		startTrial(mockFile);
	}

	function startTrial(file: File) {
		uploadedFile = file;
		trialStep = 'processing';
		trialProgress = 0;

		const steps = [
			{ progress: 15, delay: 400 },
			{ progress: 35, delay: 800 },
			{ progress: 60, delay: 1200 },
			{ progress: 85, delay: 1800 },
			{ progress: 100, delay: 2200 }
		];

		steps.forEach(({ progress, delay }) => {
			setTimeout(() => {
				trialProgress = progress;
			}, delay);
		});

		setTimeout(() => {
			trialStep = 'result';
			gsap.from('.result-field', {
				opacity: 0,
				y: 15,
				duration: 0.4,
				stagger: 0.08,
				ease: 'power2.out'
			});
		}, 2600);
	}

	function resetTrial() {
		trialStep = 'idle';
		uploadedFile = null;
		trialProgress = 0;
	}
</script>

<svelte:head>
	<title>Docufill — Fill forms once. Never fill them again.</title>
	<meta
		name="description"
		content="Start from your Gmail profile. Upload any PDF form, and Docufill maps your confirmed details from your encrypted vault. Every next form takes less time."
	/>
</svelte:head>

<div
	class="min-h-screen overflow-x-hidden bg-white font-sans text-ink selection:bg-brand selection:text-black"
>
	<!-- ═══════════════ NAVIGATION ═══════════════ -->
	<header
		class="sticky top-0 z-50 border-b border-line bg-white/90 backdrop-blur-2xl transition-all"
	>
		<div class="mx-auto flex h-20 max-w-7xl items-center justify-between px-5 sm:px-8">
			<a href={resolve('/')} class="group flex items-center gap-3" aria-label="Docufill Home">
				<BrandMark size="sm" />
			</a>

			<!-- Center Links -->
			<nav
				class="hidden items-center gap-1 rounded-full border border-line bg-surface-raised px-4 py-1.5 text-sm font-bold md:flex"
			>
				<a
					href="#how-it-works"
					class="rounded-full px-4 py-1.5 text-ink-muted transition hover:bg-white hover:text-ink"
					>How it Works</a
				>
				<a
					href="#invite-section"
					class="rounded-full px-4 py-1.5 text-ink-muted transition hover:bg-white hover:text-ink"
					>Invites</a
				>
				<a
					href="#try-it"
					class="rounded-full px-4 py-1.5 text-ink-muted transition hover:bg-white hover:text-ink"
					>Try It</a
				>
				<a
					href="#security"
					class="rounded-full px-4 py-1.5 text-ink-muted transition hover:bg-white hover:text-ink"
					>Security</a
				>
				<a
					href="#faq"
					class="rounded-full px-4 py-1.5 text-ink-muted transition hover:bg-white hover:text-ink"
					>FAQ</a
				>
			</nav>

			<!-- Right Auth -->
			<div class="flex items-center gap-3">
				{#if signedIn}
					<a
						href={resolve('/documents')}
						class="inline-flex min-h-11 items-center gap-2 rounded-full bg-brand px-5 py-2.5 text-sm font-extrabold text-black shadow-sm transition hover:bg-brand-strong active:scale-98"
					>
						My documents <ArrowRight size={17} />
					</a>
				{:else}
					<button
						type="button"
						onclick={handleGoogleSignIn}
						disabled={loading}
						class="inline-flex min-h-11 cursor-pointer items-center gap-2.5 rounded-full bg-ink px-5 py-2.5 text-sm font-extrabold text-white shadow-sm transition hover:bg-zinc-800 active:scale-98"
					>
						<svg class="size-4.5" viewBox="0 0 24 24">
							<path
								fill="#facc15"
								d="M22.56 12.25c0-.78-.07-1.53-.2-2.25H12v4.26h5.92c-.26 1.37-1.04 2.53-2.21 3.31v2.77h3.57c2.08-1.92 3.28-4.74 3.28-8.09z"
							/>
							<path
								fill="#facc15"
								d="M12 23c2.97 0 5.46-.98 7.28-2.66l-3.57-2.77c-.98.66-2.23 1.06-3.71 1.06-2.86 0-5.29-1.93-6.16-4.53H2.18v2.84C3.99 20.53 7.7 23 12 23z"
							/>
							<path
								fill="#facc15"
								d="M5.84 14.09c-.22-.66-.35-1.36-.35-2.09s.13-1.43.35-2.09V7.06H2.18C1.43 8.55 1 10.22 1 12s.43 3.45 1.18 4.94l2.85-2.22.81-.63z"
							/>
							<path
								fill="#facc15"
								d="M12 5.38c1.62 0 3.06.56 4.21 1.64l3.15-3.15C17.45 2.09 14.97 1 12 1 7.7 1 3.99 3.47 2.18 7.06l3.66 2.84c.87-2.6 3.3-4.52 6.16-4.52z"
							/>
						</svg>
						<span>Sign in with Google</span>
					</button>
				{/if}
			</div>
		</div>
	</header>

	<!-- ═══════════════ REWORKED HERO (CENTRALIZED TEXT, NO MOCKUP) ═══════════════ -->
	<section
		class="relative mx-auto max-w-5xl px-5 pt-20 pb-24 text-center sm:px-8 lg:pt-28 lg:pb-32"
	>
		<!-- Subtle background radial glow -->
		<div
			class="pointer-events-none absolute top-1/2 left-1/2 size-[32rem] -translate-x-1/2 -translate-y-1/2 rounded-full bg-brand/20 blur-3xl"
		></div>

		<div id="hero-content" class="relative z-10 flex flex-col items-center">
			<!-- Centered Badge -->
			<div
				class="mb-6 inline-flex items-center gap-2 rounded-full border border-brand/40 bg-brand-soft px-4.5 py-1.5 text-xs font-black tracking-widest text-ink uppercase shadow-xs"
			>
				<Sparkles size={14} class="text-ink" />
				Privacy-First Document Agent
			</div>

			<!-- Centered Main Headline with Accent Typography -->
			<h1
				class="max-w-4xl text-4xl leading-[1.06] font-extrabold tracking-tight text-balance text-ink sm:text-6xl lg:text-7xl"
			>
				Fill forms once. <br />
				<span class="font-display font-normal text-ink-muted italic">Never fill them again.</span>
			</h1>

			<!-- Centered Subtitle -->
			<p
				class="mt-7 max-w-2xl text-lg leading-relaxed font-medium text-balance text-ink-muted sm:text-xl"
			>
				Start from your Gmail profile. Upload any PDF form, and Docufill maps your confirmed details
				from your encrypted vault. Every next form takes less time.
			</p>

			<!-- Centered Action Buttons (Rounded-Full) -->
			<div class="mt-10 flex flex-wrap items-center justify-center gap-4">
				{#if !signedIn}
					<button
						type="button"
						onclick={handleGoogleSignIn}
						disabled={loading}
						class="inline-flex min-h-13 cursor-pointer items-center gap-3 rounded-full bg-brand px-8 py-3.5 text-base font-extrabold text-black shadow-md transition hover:bg-brand-strong active:scale-98"
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
						class="inline-flex min-h-13 items-center gap-3 rounded-full bg-brand px-8 py-3.5 text-base font-extrabold text-black shadow-md transition hover:bg-brand-strong active:scale-98"
					>
						<span>Go to My Documents</span>
						<ArrowRight size={18} />
					</a>
				{/if}

				<a
					href="#try-it"
					class="inline-flex min-h-13 items-center gap-2.5 rounded-full border-2 border-ink bg-white px-7 py-3.5 text-sm font-bold text-ink transition hover:bg-ink hover:text-white"
				>
					<Upload size={18} />
					<span>Try with a PDF</span>
				</a>
			</div>

			<!-- Centered Trust Badges Bar -->
			<div
				class="mt-14 flex flex-wrap items-center justify-center gap-8 border-t border-line pt-8 text-xs font-bold text-ink-muted"
			>
				<span class="flex items-center gap-2">
					<LockKeyhole size={15} class="text-positive" /> AES-256-GCM Encrypted
				</span>
				<span class="flex items-center gap-2">
					<ShieldCheck size={15} class="text-positive" /> Zero Model Training
				</span>
				<span class="flex items-center gap-2">
					<FileCheck size={15} class="text-positive" /> Starts from Gmail Profile
				</span>
			</div>
		</div>
	</section>

	<!-- ═══════════════ HOW IT WORKS (3-STEP BLACK CARDS) ═══════════════ -->
	<section id="how-it-works" class="lazy-reveal border-t border-line bg-white py-24">
		<div class="mx-auto max-w-7xl px-5 sm:px-8">
			<div class="mx-auto mb-16 max-w-3xl text-center">
				<p class="eyebrow mb-3">How It Works</p>
				<h2 class="text-3xl font-extrabold tracking-tight text-ink sm:text-5xl">
					Three steps. <span class="font-display font-normal text-ink-muted italic"
						>Zero retyping.</span
					>
				</h2>
				<p class="mt-4 text-base font-medium text-ink-muted sm:text-lg">
					Connect your Google profile, upload a form, and let Docufill fill what it already knows.
				</p>
			</div>

			<div class="grid gap-6 md:grid-cols-3">
				<!-- Step 1 -->
				<div
					class="surface-dark p-8 transition-all duration-300 hover:scale-[1.02] hover:border-brand-strong"
				>
					<div class="step-number mb-6">1</div>
					<h3 class="mb-3 text-xl font-extrabold">Connect Gmail</h3>
					<p class="text-sm leading-relaxed" style="color: var(--card-dark-muted)">
						Sign in with Google. Your name, email, and photo seed the first form immediately. We
						never read your inbox.
					</p>
				</div>

				<!-- Step 2 -->
				<div
					class="surface-dark p-8 transition-all duration-300 hover:scale-[1.02] hover:border-brand-strong"
				>
					<div class="step-number mb-6">2</div>
					<h3 class="mb-3 text-xl font-extrabold">Upload Your PDF</h3>
					<p class="text-sm leading-relaxed" style="color: var(--card-dark-muted)">
						Drop any digital PDF form. The Copilot maps fields, matches your confirmed details, and
						flags what's missing.
					</p>
				</div>

				<!-- Step 3 -->
				<div
					class="surface-dark p-8 transition-all duration-300 hover:scale-[1.02] hover:border-brand-strong"
				>
					<div class="step-number mb-6">3</div>
					<h3 class="mb-3 text-xl font-extrabold">Review & Sign</h3>
					<p class="text-sm leading-relaxed" style="color: var(--card-dark-muted)">
						Only fill what Docufill doesn't know. Invite a guarantor by email. Preview, sign,
						download.
					</p>
				</div>
			</div>
		</div>
	</section>

	<!-- ═══════════════ NEW PARTICIPANT / INVITE SECTION (MATCHING IMAGE 1) ═══════════════ -->
	<section
		id="invite-section"
		class="lazy-reveal border-t border-b border-[#E5E0D8] bg-[#FAF7F2] py-24"
	>
		<div class="mx-auto max-w-7xl px-5 sm:px-8">
			<div class="mx-auto mb-16 max-w-3xl text-center">
				<p class="eyebrow mb-3">No Account Required</p>
				<h2 class="text-3xl font-extrabold tracking-tight text-ink sm:text-5xl">
					Invite guarantors to <span class="font-display font-normal text-ink-muted italic"
						>fill their part.</span
					>
				</h2>
				<p class="mt-4 text-base font-medium text-ink-muted sm:text-lg">
					Tenancy and employment forms often need someone else. Send a private link — they answer
					only their fields without making an account.
				</p>
			</div>

			<!-- Split 2-Column Card (Matching Image 1 exact structure & styling) -->
			<div
				class="mx-auto grid max-w-6xl overflow-hidden rounded-2xl border border-[#D8D3C9] bg-white shadow-xl lg:grid-cols-2"
			>
				<!-- Left Column: Private Link Explanation -->
				<div class="flex flex-col justify-center bg-[#FAF8F5] p-8 sm:p-12 lg:p-16">
					<span
						class="mb-3 font-mono text-xs font-semibold tracking-widest text-ink-muted uppercase"
					>
						Adaeze's private link
					</span>
					<h3 class="text-3xl leading-tight font-extrabold tracking-tight text-ink sm:text-4xl">
						She only sees her section of this document.
					</h3>
					<p class="mt-6 text-base leading-relaxed text-ink-muted sm:text-lg">
						The rest of the page is hidden. She answers name, relationship, and phone. Those three
						lines write onto the same PDF.
					</p>

					<!-- Highlighted Line Pill (Matching Image 1) -->
					<div class="mt-8">
						<mark
							class="mark inline-block rounded px-2.5 py-1 font-mono text-sm font-bold text-ink sm:text-base"
						>
							Adaeze Okonkwo · Sister · +234 809 441 2201
						</mark>
					</div>

					<div class="mt-10 flex items-center gap-3 text-xs font-bold text-ink-muted">
						<span class="flex items-center gap-1.5"
							><Mail size={15} class="text-ink" /> Verified via Email OTP</span
						>
						<span>•</span>
						<span class="flex items-center gap-1.5"
							><EyeOff size={15} class="text-ink" /> Rest of PDF is hidden</span
						>
					</div>
				</div>

				<!-- Right Column: Document Preview (Matching Image 1 right panel) -->
				<div
					class="flex items-center justify-center border-t border-[#C8C2B5] bg-[#D8D3C9] p-8 sm:p-12 lg:border-t-0 lg:border-l lg:p-14"
				>
					<!-- White Paper Card -->
					<div
						class="w-full max-w-md overflow-hidden rounded-md border border-zinc-200 bg-white font-sans shadow-[0_12px_35px_rgba(0,0,0,0.18)]"
					>
						<!-- Paper Header -->
						<div class="border-b border-zinc-200 p-6">
							<span
								class="mb-1 block font-mono text-[10px] font-bold tracking-widest text-ink-muted uppercase"
							>
								GUARANTOR SECTION ONLY
							</span>
							<h4 class="font-serif text-lg leading-tight font-bold text-zinc-900">
								Residential Tenancy Application
							</h4>
							<p class="mt-0.5 font-sans text-xs text-ink-muted">
								14 Admiralty Way, Lekki Phase 1, Lagos
							</p>
						</div>

						<!-- Section 1: APPLICANT (Gray Header) -->
						<div
							class="bg-zinc-200 px-6 py-1.5 font-mono text-[11px] font-bold tracking-wider text-zinc-600 uppercase"
						>
							APPLICANT
						</div>
						<div class="space-y-2.5 p-6 text-xs text-ink-muted">
							<div class="flex justify-between">
								<span>Full legal name</span>
								<span class="text-ink">Chinedu Okonkwo</span>
							</div>
							<div class="flex justify-between border-t border-zinc-100 pt-2">
								<span>Email</span>
								<span class="text-ink">chinedu@bujeti.com</span>
							</div>
							<div class="flex justify-between border-t border-zinc-100 pt-2">
								<span>Phone</span>
								<span class="text-ink">+234 803 555 0142</span>
							</div>
							<div class="flex justify-between border-t border-zinc-100 pt-2">
								<span>Current employer</span>
								<span class="text-ink">Bujeti Limited</span>
							</div>
							<div class="flex justify-between border-t border-zinc-100 pt-2">
								<span>Monthly rent</span>
								<span class="text-ink">₦2,400,000</span>
							</div>
						</div>

						<!-- Section 2: GUARANTOR (Yellow Header & Highlighted Rows) -->
						<div
							class="border-t border-yellow-300 bg-[#FEF08A] px-6 py-1.5 font-mono text-[11px] font-bold tracking-wider text-zinc-800 uppercase"
						>
							GUARANTOR
						</div>
						<div class="space-y-2.5 bg-[#FEF9C3] p-6 text-xs font-medium text-zinc-900">
							<div class="flex justify-between">
								<span class="font-bold">Full name</span>
								<span class="font-extrabold text-black">Adaeze Okonkwo</span>
							</div>
							<div class="flex justify-between border-t border-yellow-200/80 pt-2">
								<span class="font-bold">Relationship</span>
								<span class="font-extrabold text-black">Sister</span>
							</div>
							<div class="flex justify-between border-t border-yellow-200/80 pt-2">
								<span class="font-bold">Phone</span>
								<span class="font-extrabold text-black">+234 809 441 2201</span>
							</div>
						</div>
					</div>
				</div>
			</div>
		</div>
	</section>

	<!-- ═══════════════ STICKY SCROLL HIGHLIGHT TEXT SECTION (GSAP SCROLLTRIGGER WORD HIGHLIGHT) ═══════════════ -->
	<section
		bind:this={scrollSectionEl}
		id="about"
		class="aboutSection relative flex min-h-screen items-center justify-center overflow-hidden bg-black px-6 py-24 text-white sm:px-12 md:px-16"
	>
		<div class="aboutSection__inner mx-auto w-full max-w-6xl text-left">
			<p
				class="aboutSection__label mb-6 text-xs font-black tracking-widest text-yellow-400/80 uppercase sm:mb-8"
			>
				THE PLATFORM
			</p>

			<h2
				class="aboutSection__heading font-display text-3xl leading-[1.18] font-bold tracking-tight text-balance uppercase sm:text-5xl sm:leading-[1.1] md:text-6xl lg:text-7xl xl:text-[5.25rem]"
			>
				{#each headlineWords as word, index (index)}
					<span
						class="aboutSection__word mr-[0.25em] inline-block text-white/20 transition-colors duration-200"
						>{word}</span
					>
				{/each}
			</h2>
		</div>
	</section>

	<!-- ═══════════════ COMPOUNDING MEMORY (IT GETS FASTER — GSAP FADE & HOVER) ═══════════════ -->
	<section bind:this={compoundingSectionEl} class="section-yellow py-24">
		<div class="mx-auto max-w-7xl px-5 sm:px-8">
			<div class="mx-auto mb-16 max-w-3xl text-center">
				<p class="mb-3 text-xs font-black tracking-widest text-ink/60 uppercase">
					Compounding Memory
				</p>
				<h2 class="text-3xl font-extrabold tracking-tight text-ink sm:text-5xl">
					It gets faster <span class="font-display font-normal text-ink/80 italic">every time.</span
					>
				</h2>
				<p class="mx-auto mt-4 max-w-2xl text-base font-medium text-ink/70 sm:text-lg">
					The first form still has gaps. Each confirmed answer is kept. By the third form, you
					mostly review and sign.
				</p>
			</div>

			<div class="grid gap-6 md:grid-cols-3">
				<!-- Form 1 Card -->
				<div
					class="compounding-card surface-dark relative overflow-hidden p-8 transition-all duration-300 hover:scale-[1.03] hover:border-brand-strong hover:shadow-2xl"
				>
					<div class="mb-6 flex items-center justify-between">
						<span class="text-xs font-black tracking-widest uppercase" style="color: var(--brand)"
							>Form 1</span
						>
						<span class="rounded-full bg-brand px-3 py-1 text-xs font-bold text-black"
							>30% filled</span
						>
					</div>
					<h3 class="mb-3 text-lg font-extrabold">First tenancy form</h3>
					<p class="mb-6 text-sm leading-relaxed" style="color: var(--card-dark-muted)">
						Gmail fills name & email. You add address, landlord details, next of kin manually.
					</p>
					<div class="progress-track" style="background: rgba(255,255,255,0.12)">
						<div
							class="progress-fill"
							style="width: {compoundingProgress[0]}%; background: var(--brand); transition: width 1s ease-out;"
						></div>
					</div>
				</div>

				<!-- Form 2 Card -->
				<div
					class="compounding-card surface-dark relative overflow-hidden p-8 transition-all duration-300 hover:scale-[1.03] hover:border-brand-strong hover:shadow-2xl"
				>
					<div class="mb-6 flex items-center justify-between">
						<span class="text-xs font-black tracking-widest uppercase" style="color: var(--brand)"
							>Form 2</span
						>
						<span class="rounded-full bg-brand px-3 py-1 text-xs font-bold text-black"
							>75% filled</span
						>
					</div>
					<h3 class="mb-3 text-lg font-extrabold">Second tenancy form</h3>
					<p class="mb-6 text-sm leading-relaxed" style="color: var(--card-dark-muted)">
						Those answers return automatically. You only handle what's new to this landlord.
					</p>
					<div class="progress-track" style="background: rgba(255,255,255,0.12)">
						<div
							class="progress-fill"
							style="width: {compoundingProgress[1]}%; background: var(--brand); transition: width 1s ease-out;"
						></div>
					</div>
				</div>

				<!-- Form 3 Card -->
				<div
					class="compounding-card surface-dark relative overflow-hidden p-8 transition-all duration-300 hover:scale-[1.03] hover:border-brand-strong hover:shadow-2xl"
				>
					<div class="mb-6 flex items-center justify-between">
						<span class="text-xs font-black tracking-widest uppercase" style="color: var(--brand)"
							>Form 3</span
						>
						<span class="rounded-full bg-brand px-3 py-1 text-xs font-bold text-black"
							>95% filled</span
						>
					</div>
					<h3 class="mb-3 text-lg font-extrabold">Third form onwards</h3>
					<p class="mb-6 text-sm leading-relaxed" style="color: var(--card-dark-muted)">
						You mostly review and sign. Almost everything comes from your confirmed pack.
					</p>
					<div class="progress-track" style="background: rgba(255,255,255,0.12)">
						<div
							class="progress-fill"
							style="width: {compoundingProgress[2]}%; background: var(--brand); transition: width 1s ease-out;"
						></div>
					</div>
				</div>
			</div>
		</div>
	</section>

	<!-- ═══════════════ TRY DOCUFILL — UPLOAD TRIAL ═══════════════ -->
	<section id="try-it" class="lazy-reveal bg-white py-24">
		<div class="mx-auto max-w-4xl px-5 sm:px-8">
			<div class="mx-auto mb-14 max-w-3xl text-center">
				<p class="eyebrow mb-3">Try Docufill</p>
				<h2 class="text-3xl font-extrabold tracking-tight text-ink sm:text-5xl">
					Upload a PDF and <span class="font-display font-normal text-ink-muted italic"
						>see it fill.</span
					>
				</h2>
				<p class="mt-4 text-base font-medium text-ink-muted sm:text-lg">
					No account needed. Drop a form below or pick a sample. The demo uses a synthetic Gmail
					profile.
				</p>
			</div>

			<!-- Trial Card -->
			<div class="surface overflow-hidden border border-line bg-white shadow-card">
				{#if trialStep === 'idle'}
					<!-- Drop Zone -->
					<div
						role="button"
						tabindex="0"
						class="drop-zone m-6 p-12 text-center {isDragging ? 'drag-active' : ''}"
						ondragover={handleDragOver}
						ondragleave={handleDragLeave}
						ondrop={handleDrop}
						onclick={() => document.getElementById('file-input')?.click()}
						onkeydown={(e) => {
							if (e.key === 'Enter' || e.key === ' ')
								document.getElementById('file-input')?.click();
						}}
					>
						<input
							id="file-input"
							type="file"
							accept=".pdf"
							class="hidden"
							onchange={handleFileInput}
						/>
						<div
							class="mx-auto mb-5 grid size-16 place-items-center rounded-full bg-ink text-brand"
						>
							<Upload size={28} />
						</div>
						<p class="mb-2 text-lg font-bold text-ink">Drop your PDF here, or click to browse</p>
						<p class="text-sm text-ink-muted">PDF only · 25 MB max · Digital forms only</p>
						<p class="mt-2 text-xs text-ink-muted">
							Printed scans are supported · handwriting may need review
						</p>
					</div>

					<!-- Sample Files -->
					<div class="px-6 pb-6">
						<p class="mb-3 text-xs font-bold tracking-wider text-ink-muted uppercase">
							Or try a sample
						</p>
						<div class="flex flex-wrap gap-3">
							<button
								type="button"
								onclick={() => startSampleTrial('Tenancy_Application')}
								class="inline-flex cursor-pointer items-center gap-2 rounded-full bg-ink px-5 py-2.5 text-sm font-bold text-white transition hover:bg-zinc-800"
							>
								<Building2 size={16} class="text-brand" /> Tenancy Application
							</button>
							<button
								type="button"
								onclick={() => startSampleTrial('Employment_Onboarding')}
								class="inline-flex cursor-pointer items-center gap-2 rounded-full bg-ink px-5 py-2.5 text-sm font-bold text-white transition hover:bg-zinc-800"
							>
								<Briefcase size={16} class="text-brand" /> Employment Onboarding
							</button>
							<button
								type="button"
								onclick={() => startSampleTrial('School_Admission')}
								class="inline-flex cursor-pointer items-center gap-2 rounded-full bg-ink px-5 py-2.5 text-sm font-bold text-white transition hover:bg-zinc-800"
							>
								<GraduationCap size={16} class="text-brand" /> School Admission
							</button>
						</div>
					</div>

					<!-- Legal -->
					<div class="border-t border-line px-6 py-4">
						<p class="text-xs text-ink-muted">
							Demo files and the sample Gmail profile are synthetic. Don't upload wills, court
							papers, or anything that needs a regulated signature.
							<a href={resolve('/acceptable-use')} class="underline hover:text-ink"
								>Acceptable Use</a
							>
							·
							<a href={resolve('/privacy')} class="underline hover:text-ink">Privacy</a>
						</p>
					</div>
				{:else if trialStep === 'processing'}
					<!-- Processing State -->
					<div class="p-10 text-center">
						<div
							class="mx-auto mb-6 grid size-16 animate-pulse place-items-center rounded-full bg-brand"
						>
							<Zap size={28} class="text-black" />
						</div>
						<h3 class="mb-2 text-xl font-extrabold text-ink">{uploadedFile?.name}</h3>
						<p class="mb-6 text-sm text-ink-muted">
							{#if trialProgress < 20}
								Checking PDF…
							{:else if trialProgress < 40}
								Uploading document…
							{:else if trialProgress < 65}
								Finding fields…
							{:else if trialProgress < 90}
								Applying Gmail profile…
							{:else}
								Grounding from your pack…
							{/if}
						</p>
						<div class="mx-auto max-w-md">
							<div class="progress-track">
								<div class="progress-fill" style="width: {trialProgress}%"></div>
							</div>
							<p class="mt-3 text-xs font-bold text-ink-muted">{trialProgress}%</p>
						</div>
					</div>
				{:else if trialStep === 'result'}
					<!-- Result State -->
					<div class="p-6 sm:p-8">
						<div class="mb-6 flex items-center justify-between border-b border-line pb-4">
							<div class="flex items-center gap-3">
								<div
									class="grid size-11 place-items-center rounded-control bg-ink text-sm font-black text-brand"
								>
									PDF
								</div>
								<div>
									<h3 class="text-base font-bold text-ink">{uploadedFile?.name}</h3>
									<p class="text-xs text-ink-muted">7 Fields Detected · 5 Auto-Filled</p>
								</div>
							</div>
							<button
								type="button"
								onclick={resetTrial}
								class="grid size-8 cursor-pointer place-items-center rounded-full transition hover:bg-surface-raised"
								aria-label="Close result"
							>
								<X size={18} class="text-ink-muted" />
							</button>
						</div>

						<!-- Copilot Strip -->
						<div class="mb-6 flex items-center gap-3 rounded-control bg-ink p-4 text-white">
							<Sparkles size={18} class="shrink-0 text-brand" />
							<p class="text-sm font-medium">
								I found <strong class="text-brand">7 fields</strong> · 2 from Gmail · 3 from your tenancy
								pack · 1 needs you · 1 sent to guarantor
							</p>
						</div>

						<!-- Result Fields Grid -->
						<div class="grid gap-3 sm:grid-cols-2">
							{#each sampleResults as field (field.label)}
								<div
									class="result-field rounded-control border p-4 transition {field.filled
										? 'border-line bg-surface-raised hover:border-brand-strong'
										: 'border-brand-strong/40 bg-brand-soft'}"
								>
									<div class="mb-1.5 flex items-center justify-between text-xs text-ink-muted">
										<span class="font-bold">{field.label}</span>
										{#if field.filled}
											<span class="flex items-center gap-1 font-bold text-positive"
												><CheckCircle2 size={13} /> {field.source}</span
											>
										{:else}
											<span class="rounded bg-brand/30 px-2 py-0.5 text-[10px] font-bold text-ink"
												>{field.source}</span
											>
										{/if}
									</div>
									{#if field.filled}
										<p class="text-base font-bold text-ink">{field.value}</p>
									{:else}
										<p class="text-sm text-ink-muted italic">
											{field.source === 'Needs you'
												? 'Waiting for your input'
												: 'Assigned via email + private link'}
										</p>
									{/if}
								</div>
							{/each}
						</div>

						<!-- Conversion CTA -->
						<div class="mt-8 rounded-control border border-line bg-surface-raised p-6 text-center">
							<p class="mb-4 text-sm text-ink-muted">
								This demo used a sample Gmail profile. <strong class="text-ink"
									>Continue with Google</strong
								>
								to fill from <em>your</em> name and email, keep a tenancy pack, and invite a guarantor
								by email.
							</p>
							{#if !signedIn}
								<button
									type="button"
									onclick={handleGoogleSignIn}
									disabled={loading}
									class="inline-flex min-h-12 cursor-pointer items-center gap-3 rounded-full bg-brand px-7 py-3 text-sm font-extrabold text-black transition hover:bg-brand-strong active:scale-98"
								>
									<svg class="size-4.5" viewBox="0 0 24 24">
										<path
											fill="#000"
											d="M22.56 12.25c0-.78-.07-1.53-.2-2.25H12v4.26h5.92c-.26 1.37-1.04 2.53-2.21 3.31v2.77h3.57c2.08-1.92 3.28-4.74 3.28-8.09z"
										/>
										<path
											fill="#000"
											d="M12 23c2.97 0 5.46-.98 7.28-2.66l-3.57-2.77c-.98.66-2.23 1.06-3.71 1.06-2.86 0-5.29-1.93-6.16-4.53H2.18v2.84C3.99 20.53 7.7 23 12 23z"
										/>
										<path
											fill="#000"
											d="M5.84 14.09c-.22-.66-.35-1.36-.35-2.09s.13-1.43.35-2.09V7.06H2.18C1.43 8.55 1 10.22 1 12s.43 3.45 1.18 4.94l2.85-2.22.81-.63z"
										/>
										<path
											fill="#000"
											d="M12 5.38c1.62 0 3.06.56 4.21 1.64l3.15-3.15C17.45 2.09 14.97 1 12 1 7.7 1 3.99 3.47 2.18 7.06l3.66 2.84c.87-2.6 3.3-4.52 6.16-4.52z"
										/>
									</svg>
									Continue with Google
									<ArrowRight size={16} />
								</button>
							{:else}
								<a
									href={resolve('/documents')}
									class="inline-flex min-h-12 items-center gap-3 rounded-full bg-brand px-7 py-3 text-sm font-extrabold text-black transition hover:bg-brand-strong active:scale-98"
								>
									Go to My Documents
									<ArrowRight size={16} />
								</a>
							{/if}
						</div>
					</div>
				{/if}
			</div>
		</div>
	</section>

	<!-- ═══════════════ REWORKED PRIVACY SECTION (BANNER LOOKING CARD) ═══════════════ -->
	<section id="security" class="lazy-reveal border-t border-line bg-white py-24">
		<div class="mx-auto max-w-6xl px-5 sm:px-8">
			<!-- Banner Card -->
			<div
				class="relative overflow-hidden rounded-3xl border border-zinc-800 bg-zinc-950 p-8 text-white shadow-2xl sm:p-14"
			>
				<!-- Decorative background glow -->
				<div
					class="pointer-events-none absolute -top-24 -right-24 size-80 rounded-full bg-brand/15 blur-3xl"
				></div>

				<div class="relative z-10 mb-12 max-w-3xl">
					<div
						class="mb-4 inline-flex items-center gap-2 rounded-full border border-brand/30 bg-brand/20 px-4 py-1 font-mono text-xs font-bold tracking-widest text-brand uppercase"
					>
						<LockKeyhole size={14} /> Zero-Trust Architecture
					</div>
					<h2 class="text-3xl font-extrabold tracking-tight text-white sm:text-5xl">
						Your privacy is <span class="font-display font-normal text-brand italic"
							>non-negotiable.</span
						>
					</h2>
					<p class="mt-4 text-base font-medium text-zinc-400 sm:text-lg">
						We built Docufill from day one so your data stays encrypted, private, and strictly in
						your control.
					</p>
				</div>

				<!-- 4 Banner Grid Columns inside the card -->
				<div class="grid gap-6 border-t border-zinc-800/80 pt-10 sm:grid-cols-2 lg:grid-cols-4">
					<div class="space-y-3">
						<div class="grid size-10 place-items-center rounded-lg bg-brand text-black">
							<LockKeyhole size={20} />
						</div>
						<h3 class="text-base font-bold text-white">AES-256 Encryption</h3>
						<p class="text-xs leading-relaxed text-zinc-400">
							Facts, answers, and document excerpts are encrypted with unique random nonces before
							saving.
						</p>
					</div>

					<div class="space-y-3">
						<div class="grid size-10 place-items-center rounded-lg bg-brand text-black">
							<ShieldCheck size={20} />
						</div>
						<h3 class="text-base font-bold text-white">Zero Model Training</h3>
						<p class="text-xs leading-relaxed text-zinc-400">
							Customer data is never used to train public AI models. AI receives minimal grounded
							facts per question.
						</p>
					</div>

					<div class="space-y-3">
						<div class="grid size-10 place-items-center rounded-lg bg-brand text-black">
							<EyeOff size={20} />
						</div>
						<h3 class="text-base font-bold text-white">No Inbox Access</h3>
						<p class="text-xs leading-relaxed text-zinc-400">
							Google sign-in seeds your profile name and photo. We never read or access your Gmail
							inbox.
						</p>
					</div>

					<div class="space-y-3">
						<div class="grid size-10 place-items-center rounded-lg bg-brand text-black">
							<FileCheck size={20} />
						</div>
						<h3 class="text-base font-bold text-white">Instant Self-Deletion</h3>
						<p class="text-xs leading-relaxed text-zinc-400">
							Deleting a document or account immediately purges every original, preview, and vector
							chunk.
						</p>
					</div>
				</div>
			</div>
		</div>
	</section>

	<!-- ═══════════════ FAQ ═══════════════ -->
	<section id="faq" class="lazy-reveal border-t border-line bg-white py-24">
		<div class="mx-auto max-w-4xl px-5 sm:px-8">
			<div class="mb-14 text-center">
				<p class="eyebrow mb-3">Questions & Answers</p>
				<h2 class="text-3xl font-extrabold tracking-tight text-ink sm:text-5xl">
					Frequently asked <span class="font-display font-normal text-ink-muted italic"
						>questions.</span
					>
				</h2>
			</div>

			<div class="space-y-4">
				{#each faqs as faq, index (faq.q)}
					<div class="surface-dark overflow-hidden rounded-2xl transition">
						<button
							type="button"
							onclick={() => toggleFaq(index)}
							class="flex w-full cursor-pointer items-center justify-between p-6 text-left text-lg font-extrabold"
							style="color: var(--card-dark-ink)"
						>
							<span>{faq.q}</span>
							<ChevronDown
								size={20}
								class="ml-4 shrink-0 transition-transform duration-300 {openFaq === index
									? 'rotate-180 text-brand'
									: ''}"
								style="color: {openFaq === index ? 'var(--brand)' : 'var(--card-dark-muted)'}"
							/>
						</button>
						{#if openFaq === index}
							<div
								class="border-t px-6 pt-4 pb-6 text-sm leading-relaxed font-medium"
								style="color: var(--card-dark-muted); border-color: var(--card-dark-line)"
							>
								{faq.a}
							</div>
						{/if}
					</div>
				{/each}
			</div>
		</div>
	</section>

	<!-- ═══════════════ FINAL CTA — YELLOW ═══════════════ -->
	<section class="lazy-reveal section-yellow py-24">
		<div class="mx-auto max-w-5xl px-5 text-center sm:px-8">
			<h2 class="text-3xl font-extrabold tracking-tight text-ink sm:text-5xl">
				Ready to simplify form filling <span class="font-display font-normal text-ink/80 italic"
					>forever?</span
				>
			</h2>
			<p class="mx-auto mt-4 max-w-2xl text-base font-medium text-ink/70 sm:text-lg">
				Sign in with Google to create your encrypted profile vault and fill your first document in
				seconds. We use your Google profile to start the form. We do not read your Gmail inbox.
			</p>

			<div class="mx-auto mt-8 flex max-w-md flex-col items-center justify-center gap-4">
				{#if signedIn}
					<Button
						class="w-full rounded-full py-3 text-base font-extrabold"
						onclick={() => goto(resolve('/documents'))}
					>
						Continue to My Documents <ArrowRight size={18} />
					</Button>
				{:else}
					<button
						type="button"
						onclick={handleGoogleSignIn}
						disabled={loading}
						class="inline-flex min-h-13 w-full cursor-pointer items-center justify-center gap-3 rounded-full bg-ink px-6 py-3.5 text-base font-extrabold text-white shadow-sm transition hover:bg-zinc-800"
					>
						<svg class="size-5" viewBox="0 0 24 24">
							<path
								fill="#facc15"
								d="M22.56 12.25c0-.78-.07-1.53-.2-2.25H12v4.26h5.92c-.26 1.37-1.04 2.53-2.21 3.31v2.77h3.57c2.08-1.92 3.28-4.74 3.28-8.09z"
							/>
							<path
								fill="#facc15"
								d="M12 23c2.97 0 5.46-.98 7.28-2.66l-3.57-2.77c-.98.66-2.23 1.06-3.71 1.06-2.86 0-5.29-1.93-6.16-4.53H2.18v2.84C3.99 20.53 7.7 23 12 23z"
							/>
							<path
								fill="#facc15"
								d="M5.84 14.09c-.22-.66-.35-1.36-.35-2.09s.13-1.43.35-2.09V7.06H2.18C1.43 8.55 1 10.22 1 12s.43 3.45 1.18 4.94l2.85-2.22.81-.63z"
							/>
							<path
								fill="#facc15"
								d="M12 5.38c1.62 0 3.06.56 4.21 1.64l3.15-3.15C17.45 2.09 14.97 1 12 1 7.7 1 3.99 3.47 2.18 7.06l3.66 2.84c.87-2.6 3.3-4.52 6.16-4.52z"
							/>
						</svg>
						<span>Continue with Google</span>
					</button>

					<div
						class="my-1 flex w-full items-center gap-3 text-xs font-bold tracking-wider text-ink/70 uppercase"
					>
						<span class="h-px flex-1 bg-ink/20"></span> or email magic link
						<span class="h-px flex-1 bg-ink/20"></span>
					</div>

					<form onsubmit={handleEmailSignIn} class="flex w-full gap-2">
						<input
							type="email"
							required
							bind:value={email}
							placeholder="you@example.com"
							class="min-h-11 flex-1 rounded-full border-2 border-ink/20 bg-white px-5 text-sm text-ink placeholder:text-ink-muted focus:border-ink"
						/>
						<button
							type="submit"
							disabled={loading}
							class="rounded-full bg-ink px-6 py-2.5 text-sm font-bold text-white transition hover:bg-zinc-800"
						>
							Send
						</button>
					</form>
				{/if}

				{#if message}<p class="text-xs font-bold text-positive">{message}</p>{/if}
				{#if error}<p class="text-xs font-bold text-negative">{error}</p>{/if}
			</div>
		</div>
	</section>

	<!-- ═══════════════ REFACTORED FOOTER (ROUNDED-FULL BUTTONS) ═══════════════ -->
	<footer class="border-t border-line bg-zinc-950 py-14 text-white">
		<div
			class="mx-auto flex max-w-7xl flex-col items-center justify-between gap-8 px-5 sm:px-8 md:flex-row"
		>
			<div class="flex items-center gap-3">
				<BrandMark size="sm" />
			</div>

			<div
				class="flex flex-wrap items-center justify-center gap-6 text-sm font-medium text-zinc-400"
			>
				<a href="#how-it-works" class="transition hover:text-white">How it Works</a>
				<a href="#invite-section" class="transition hover:text-white">Invites</a>
				<a href="#try-it" class="transition hover:text-white">Try It</a>
				<a href={resolve('/privacy')} class="transition hover:text-white">Privacy Policy</a>
				<a href={resolve('/acceptable-use')} class="transition hover:text-white">Acceptable Use</a>
			</div>

			<div class="font-mono text-xs text-zinc-400">
				© {new Date().getFullYear()} Docufill Agent. All rights reserved.
			</div>
		</div>
	</footer>
</div>
