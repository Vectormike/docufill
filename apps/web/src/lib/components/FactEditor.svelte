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

	type DetailOption = {
		key: string;
		label: string;
		example: string;
		sensitive?: boolean;
	};

	const detailOptions: Record<string, DetailOption[]> = {
		identity: [
			{ key: 'full_name', label: 'Full name', example: 'e.g. Victor Jonah' },
			{
				key: 'date_of_birth',
				label: 'Date of birth',
				example: 'e.g. 14 May 1995',
				sensitive: true
			},
			{ key: 'nationality', label: 'Nationality', example: 'e.g. Nigerian' },
			{ key: 'marital_status', label: 'Marital status', example: 'e.g. Single' }
		],
		contact: [
			{ key: 'email_address', label: 'Email address', example: 'e.g. victor@example.com' },
			{ key: 'phone_number', label: 'Phone number', example: 'e.g. +234 800 000 0000' }
		],
		address: [
			{
				key: 'residential_address',
				label: 'Residential address',
				example: 'Enter your full address'
			},
			{ key: 'city', label: 'City', example: 'e.g. Abuja' },
			{ key: 'state', label: 'State', example: 'e.g. FCT' },
			{ key: 'country', label: 'Country', example: 'e.g. Nigeria' }
		],
		employment: [
			{ key: 'employer_name', label: 'Current employer', example: 'e.g. Bujeti' },
			{ key: 'occupation', label: 'Occupation or job title', example: 'e.g. Backend Engineer' },
			{ key: 'employment_status', label: 'Employment status', example: 'e.g. Full-time employee' },
			{
				key: 'employer_address',
				label: 'Employer address',
				example: "Enter your employer's address"
			},
			{ key: 'employment_start_date', label: 'Employment start date', example: 'e.g. March 2025' }
		],
		education: [
			{
				key: 'institution_name',
				label: 'School or institution',
				example: 'e.g. University of Abuja'
			},
			{ key: 'qualification', label: 'Qualification', example: 'e.g. BSc Computer Science' },
			{ key: 'graduation_year', label: 'Graduation year', example: 'e.g. 2022' }
		],
		contacts: [
			{ key: 'next_of_kin_name', label: 'Next of kin name', example: 'Enter their full name' },
			{ key: 'next_of_kin_phone', label: 'Next of kin phone', example: 'e.g. +234 800 000 0000' },
			{
				key: 'emergency_contact_name',
				label: 'Emergency contact name',
				example: 'Enter their full name'
			},
			{
				key: 'emergency_contact_phone',
				label: 'Emergency contact phone',
				example: 'e.g. +234 800 000 0000'
			}
		],
		financial: [
			{ key: 'bank_name', label: 'Bank name', example: 'e.g. Guaranty Trust Bank' },
			{
				key: 'account_name',
				label: 'Account name',
				example: 'Enter the account name',
				sensitive: true
			},
			{
				key: 'account_number',
				label: 'Account number',
				example: 'Enter the account number',
				sensitive: true
			}
		],
		identification: [
			{
				key: 'nin',
				label: 'National Identification Number (NIN)',
				example: 'Enter your NIN',
				sensitive: true
			},
			{
				key: 'passport_number',
				label: 'Passport number',
				example: 'Enter your passport number',
				sensitive: true
			},
			{
				key: 'drivers_licence_number',
				label: "Driver's licence number",
				example: 'Enter your licence number',
				sensitive: true
			}
		]
	};

	let namespace = $state('identity');
	let factKey = $state(detailOptions.identity[0].key);
	let customFactName = $state('');
	let value = $state('');
	let sensitivity = $state('personal');
	const selectedDetail = $derived(
		detailOptions[namespace]?.find((option) => option.key === factKey)
	);

	function chooseSection(nextNamespace: string) {
		namespace = nextNamespace;
		const firstOption = detailOptions[nextNamespace]?.[0];
		factKey = firstOption?.key ?? '';
		customFactName = '';
		value = '';
		sensitivity = firstOption?.sensitive ? 'sensitive' : 'personal';
	}

	function chooseDetail(nextFactKey: string) {
		factKey = nextFactKey;
		const option = detailOptions[namespace]?.find((item) => item.key === nextFactKey);
		sensitivity = option?.sensitive ? 'sensitive' : 'personal';
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
				{#each sections as section (section[0])}
					<option value={section[0]}>{section[1]}</option>
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
					{#each detailOptions[namespace] ?? [] as option (option.key)}
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
