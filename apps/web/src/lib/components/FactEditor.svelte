<script lang="ts">
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

	const sections = [
		['identity', 'Identity'],
		['contact', 'Contact'],
		['address', 'Address'],
		['employment', 'Employment'],
		['education', 'Education'],
		['contacts', 'Next of kin & contacts'],
		['financial', 'Financial'],
		['identification', 'Identification'],
		['custom', 'Reusable answer']
	];

	let namespace = $state('identity');
	let factKey = $state('');
	let value = $state('');
	let sensitivity = $state('personal');

	async function submit(event: SubmitEvent) {
		event.preventDefault();
		await onsave({
			namespace,
			fact_key: factKey.trim().toLowerCase().replace(/\s+/g, '_'),
			value: value.trim(),
			value_type: 'text',
			sensitivity
		});
	}
</script>

<form class="rounded-2xl border border-brand/30 bg-brand-soft/60 p-5" onsubmit={submit}>
	<div class="grid gap-4 sm:grid-cols-2">
		<label class="text-xs font-extrabold tracking-wider text-ink-muted uppercase">
			Section
			<select
				bind:value={namespace}
				class="mt-2 min-h-11 w-full rounded-control border-line bg-surface-raised text-sm text-ink"
			>
				{#each sections as section (section[0])}
					<option value={section[0]}>{section[1]}</option>
				{/each}
			</select>
		</label>
		<label class="text-xs font-extrabold tracking-wider text-ink-muted uppercase">
			Detail name
			<input
				required
				maxlength="100"
				bind:value={factKey}
				placeholder="e.g. Current employer"
				class="mt-2 min-h-11 w-full rounded-control border-line bg-surface-raised text-sm text-ink"
			/>
		</label>
	</div>
	<label class="mt-4 block text-xs font-extrabold tracking-wider text-ink-muted uppercase">
		Value
		<textarea
			required
			rows="3"
			maxlength="5000"
			bind:value
			placeholder="Enter the confirmed information"
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
			<span class="font-bold">Mask this sensitive value</span>
			<span class="mt-0.5 block text-xs text-ink-muted"
				>Use for identifiers or financial details that should not appear in previews.</span
			>
		</span>
	</label>
	<div class="mt-5 flex justify-end gap-2">
		<Button type="button" variant="ghost" onclick={oncancel}>Cancel</Button>
		<Button type="submit" loading={saving}>Save confirmed detail</Button>
	</div>
</form>
