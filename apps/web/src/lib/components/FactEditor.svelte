<script lang="ts">
	import { factCatalog, factSections, findFact } from '$lib/profile-facts';
	import Button from './Button.svelte';

	let {
		onsave,
		oncancel,
		saving = false
	}: {
		onsave: (input: {
			namespace: string;
			fact_key: string;
			value: string;
			value_type: string;
			sensitivity: string;
		}) => Promise<void>;
		oncancel: () => void;
		saving?: boolean;
	} = $props();

	let namespace = $state('identity');
	let factKey = $state(factCatalog.identity[0].key);
	let customFactName = $state('');
	let value = $state('');
	let sensitivity = $state('personal');
	const selectedDetail = $derived(findFact(namespace, factKey));

	function chooseSection(nextNamespace: string) {
		namespace = nextNamespace;
		const firstOption = factCatalog[nextNamespace]?.[0];
		factKey = firstOption?.key ?? '';
		customFactName = '';
		value = '';
		sensitivity = firstOption?.sensitive ? 'sensitive' : 'personal';
	}

	function chooseDetail(nextFactKey: string) {
		factKey = nextFactKey;
		sensitivity = findFact(namespace, nextFactKey)?.sensitive ? 'sensitive' : 'personal';
	}

	async function submit(event: SubmitEvent) {
		event.preventDefault();
		const selectedFactKey = namespace === 'custom' ? customFactName : factKey;
		await onsave({
			namespace,
			fact_key: selectedFactKey.trim().toLowerCase().replace(/\s+/g, '_'),
			value: value.trim(),
			value_type: 'text',
			sensitivity
		});
	}
</script>

<form class="rounded-xl border border-brand/30 bg-brand-soft/60 p-5" onsubmit={submit}>
	<div class="mb-5 rounded-xl bg-surface-raised p-4">
		<p class="text-sm font-extrabold text-ink">What information are you adding?</p>
		<p class="mt-1 text-xs leading-5 text-ink-muted">
			Choose the type of detail first, then enter your actual answer below.
		</p>
	</div>
	<div class="grid gap-4 sm:grid-cols-2">
		<label class="text-xs font-extrabold tracking-wider text-ink-muted uppercase">
			Category
			<select
				value={namespace}
				onchange={(event) => chooseSection(event.currentTarget.value)}
				class="mt-2 min-h-11 w-full rounded-control border-line bg-surface-raised text-sm text-ink"
			>
				{#each factSections as section (section.id)}
					<option value={section.id}>{section.label}</option>
				{/each}
			</select>
		</label>
		<label class="text-xs font-extrabold tracking-wider text-ink-muted uppercase">
			Detail
			{#if namespace === 'custom'}
				<input
					required
					maxlength="100"
					bind:value={customFactName}
					placeholder="e.g. Preferred contact time"
					class="mt-2 min-h-11 w-full rounded-control border-line bg-surface-raised text-sm text-ink"
				/>
			{:else}
				<select
					value={factKey}
					onchange={(event) => chooseDetail(event.currentTarget.value)}
					class="mt-2 min-h-11 w-full rounded-control border-line bg-surface-raised text-sm text-ink"
				>
					{#each factCatalog[namespace] ?? [] as option (option.key)}
						<option value={option.key}>{option.label}</option>
					{/each}
				</select>
			{/if}
		</label>
	</div>
	<label class="mt-4 block text-xs font-extrabold tracking-wider text-ink-muted uppercase">
		{selectedDetail ? `Your ${selectedDetail.label.toLowerCase()}` : 'Your answer'}
		<textarea
			required
			rows="3"
			maxlength="5000"
			bind:value
			placeholder={selectedDetail?.example ?? 'Enter the information you want Docufill to reuse'}
			class="mt-2 w-full rounded-control border-line bg-surface-raised text-sm text-ink"></textarea>
	</label>
	<label class="mt-4 flex cursor-pointer items-start gap-3 text-sm text-ink">
		<input
			type="checkbox"
			checked={sensitivity === 'sensitive'}
			onchange={(event) => (sensitivity = event.currentTarget.checked ? 'sensitive' : 'personal')}
			class="mt-0.5 rounded border-line text-brand"
		/>
		<span>
			<span class="font-bold">Hide this answer in previews</span>
			<span class="mt-0.5 block text-xs text-ink-muted"
				>Recommended for identification and financial information.</span
			>
		</span>
	</label>
	<div class="mt-5 flex justify-end gap-2">
		<Button type="button" variant="ghost" onclick={oncancel}>Cancel</Button>
		<Button type="submit" loading={saving}>Save confirmed detail</Button>
	</div>
</form>
