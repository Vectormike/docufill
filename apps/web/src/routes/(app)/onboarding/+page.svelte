<script lang="ts">
	import { ArrowRight } from '@lucide/svelte';
	import { goto } from '$app/navigation';
	import { resolve } from '$app/paths';
	import { onMount } from 'svelte';
	import { api, type ProfileFact } from '$lib/api/client';
	import { currentSession } from '$lib/auth';
	import BrandMark from '$lib/components/BrandMark.svelte';
	import Button from '$lib/components/Button.svelte';
	import {
		answerKey,
		emailFact,
		emptyOnboardingAnswers,
		filledFactsFromAnswers,
		onboardingSteps,
		requireFact,
		type DraftAnswers,
		type ProfileFactInput
	} from '$lib/profile-facts';

	let answers = $state<DraftAnswers>(emptyOnboardingAnswers());
	let stepIndex = $state(0);
	let loading = $state(true);
	let saving = $state(false);
	let error = $state('');
	let email = $state('');
	let existingFacts = $state<ProfileFact[]>([]);

	const step = $derived(onboardingSteps[stepIndex]);
	const lastStep = $derived(stepIndex === onboardingSteps.length - 1);

	onMount(() => void load());

	async function load() {
		loading = true;
		error = '';
		try {
			const [vault, session] = await Promise.all([api.profile(), currentSession()]);
			email = session?.user.email ?? '';
			existingFacts = vault.facts;
			const next = emptyOnboardingAnswers();
			const displayName = vault.profile.display_name?.trim();
			if (displayName) next[answerKey('identity', 'full_name')] = displayName;
			for (const fact of vault.facts) {
				const key = answerKey(fact.namespace, fact.fact_key);
				if (key in next && fact.value_preview && fact.sensitivity !== 'sensitive') {
					next[key] = fact.value_preview;
				}
			}
			answers = next;
		} catch (cause) {
			error = cause instanceof Error ? cause.message : 'Your details could not be loaded.';
		} finally {
			loading = false;
		}
	}

	async function continueStep() {
		saving = true;
		error = '';
		try {
			await saveFacts(factsForCurrentStep());
			if (lastStep) {
				await finish();
				return;
			}
			goTo(stepIndex + 1);
		} catch (cause) {
			error = cause instanceof Error ? cause.message : 'Those details could not be saved.';
		} finally {
			saving = false;
		}
	}

	async function skipStep() {
		if (lastStep) {
			await skipOnboarding();
			return;
		}
		goTo(stepIndex + 1);
	}

	async function skipOnboarding() {
		saving = true;
		error = '';
		try {
			await finish();
		} catch (cause) {
			error = cause instanceof Error ? cause.message : 'We could not continue.';
			saving = false;
		}
	}

	async function finish() {
		await saveFacts(filledFactsFromAnswers(answers));
		const verifiedEmail = emailFact(email);
		if (verifiedEmail && !hasFact('contact', 'email_address')) {
			await saveFacts([verifiedEmail]);
		}
		const fullName = answers[answerKey('identity', 'full_name')]?.trim();
		await api.updateProfile({
			display_name: fullName && fullName.length >= 2 ? fullName : undefined,
			onboarding_completed: true
		});
		await goto(resolve('/documents'), { replaceState: true });
	}

	async function saveFacts(facts: ProfileFactInput[]) {
		for (const fact of facts) {
			const saved = await api.saveFact(fact);
			existingFacts = [
				...existingFacts.filter(
					(item) => !(item.namespace === saved.namespace && item.fact_key === saved.fact_key)
				),
				saved
			];
		}
	}

	function factsForCurrentStep(): ProfileFactInput[] {
		const currentKeys = new Set(step.fields.map((field) => answerKey(field.namespace, field.key)));
		return filledFactsFromAnswers(answers).filter((fact) =>
			currentKeys.has(answerKey(fact.namespace, fact.fact_key))
		);
	}

	function goTo(index: number) {
		stepIndex = index;
		if (onboardingSteps[index]?.id !== 'home') return;
		const key = answerKey('address', 'country');
		if (!answers[key]?.trim()) answers[key] = 'Nigeria';
	}

	function hasFact(namespace: string, key: string) {
		return existingFacts.some((fact) => fact.namespace === namespace && fact.fact_key === key);
	}

	function fieldId(namespace: string, key: string) {
		return `onboarding-${namespace}-${key}`;
	}
</script>

<svelte:head><title>Tell us about you — Docufill</title></svelte:head>

<main class="mx-auto flex min-h-dvh max-w-xl flex-col px-5 py-8 sm:px-8 sm:py-12">
	<div class="mb-10 flex items-center justify-between gap-4">
		<BrandMark size="sm" />
		<button
			type="button"
			class="text-sm font-semibold text-ink-muted hover:text-ink"
			onclick={skipOnboarding}
			disabled={saving}
		>
			I'll add these later
		</button>
	</div>

	{#if loading}
		<div class="surface h-80 p-6">
			<div class="skeleton h-full rounded-xl"></div>
		</div>
	{:else}
		<p class="eyebrow">{stepIndex + 1} of {onboardingSteps.length}</p>
		<h1 class="ask mt-3 text-3xl leading-tight text-ink sm:text-4xl">
			<span class="mark">{step.title}</span>
		</h1>
		<p class="mt-4 text-sm leading-6 text-ink-muted">{step.prompt}</p>

		{#if error}
			<p
				class="mt-5 rounded-xl bg-negative/10 p-4 text-sm font-semibold text-negative"
				role="alert"
			>
				{error}
			</p>
		{/if}

		<form
			class="mt-8 space-y-6"
			onsubmit={(event) => {
				event.preventDefault();
				void continueStep();
			}}
		>
			{#each step.fields as field (answerKey(field.namespace, field.key))}
				{@const definition = requireFact(field)}
				{@const key = answerKey(field.namespace, field.key)}
				<label class="block" for={fieldId(field.namespace, field.key)}>
					<span class="text-xs font-extrabold tracking-wider text-ink-muted uppercase">
						{definition.label}
					</span>
					{#if definition.input === 'textarea'}
						<textarea
							id={fieldId(field.namespace, field.key)}
							rows="3"
							maxlength="5000"
							autocomplete={definition.autocomplete}
							bind:value={answers[key]}
							placeholder={definition.example}
							class="mt-2 w-full rounded-control border-line bg-surface-raised text-sm text-ink"
						></textarea>
					{:else}
						<input
							id={fieldId(field.namespace, field.key)}
							type={definition.input ?? 'text'}
							maxlength="5000"
							autocomplete={definition.autocomplete}
							bind:value={answers[key]}
							placeholder={definition.example}
							class="mt-2 min-h-11 w-full rounded-control border-line bg-surface-raised text-sm text-ink"
						/>
					{/if}
				</label>
			{/each}

			<div class="flex flex-col-reverse gap-3 pt-2 sm:flex-row sm:items-center sm:justify-between">
				<div class="flex flex-wrap gap-2">
					{#if stepIndex > 0}
						<Button
							type="button"
							variant="ghost"
							onclick={() => goTo(stepIndex - 1)}
							disabled={saving}
						>
							Back
						</Button>
					{/if}
					<Button type="button" variant="ghost" onclick={skipStep} disabled={saving}>
						Skip this step
					</Button>
				</div>
				<Button type="submit" loading={saving} class="sm:min-w-44">
					{lastStep ? 'Save and continue' : 'Continue'}
					<ArrowRight size={16} />
				</Button>
			</div>
		</form>
	{/if}
</main>
