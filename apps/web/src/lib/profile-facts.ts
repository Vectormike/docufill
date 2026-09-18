import type { FullAutoFill } from 'svelte/elements';

export type FactInput = 'text' | 'tel' | 'email' | 'date' | 'textarea';

export type FactDefinition = {
	key: string;
	label: string;
	example: string;
	sensitive?: boolean;
	input?: FactInput;
	autocomplete?: FullAutoFill;
};

export const factSections = [
	{ id: 'identity', label: 'Identity' },
	{ id: 'contact', label: 'Contact' },
	{ id: 'address', label: 'Address' },
	{ id: 'employment', label: 'Employment' },
	{ id: 'education', label: 'Education' },
	{ id: 'contacts', label: 'Next of kin & contacts' },
	{ id: 'financial', label: 'Financial' },
	{ id: 'identification', label: 'Identification' },
	{ id: 'custom', label: 'Reusable answer' }
] as const;

export const factCatalog: Record<string, FactDefinition[]> = {
	identity: [
		{ key: 'full_name', label: 'Full name', example: 'e.g. Victor Jonah', autocomplete: 'name' },
		{
			key: 'date_of_birth',
			label: 'Date of birth',
			example: 'e.g. 14 May 1995',
			sensitive: true,
			input: 'date',
			autocomplete: 'bday'
		},
		{ key: 'nationality', label: 'Nationality', example: 'e.g. Nigerian' },
		{ key: 'marital_status', label: 'Marital status', example: 'e.g. Single' }
	],
	contact: [
		{
			key: 'email_address',
			label: 'Email address',
			example: 'e.g. victor@example.com',
			input: 'email',
			autocomplete: 'email'
		},
		{
			key: 'phone_number',
			label: 'Phone number',
			example: 'e.g. +234 800 000 0000',
			input: 'tel',
			autocomplete: 'tel'
		}
	],
	address: [
		{
			key: 'residential_address',
			label: 'Residential address',
			example: 'Enter your full address',
			input: 'textarea',
			autocomplete: 'street-address'
		},
		{ key: 'city', label: 'City', example: 'e.g. Abuja', autocomplete: 'address-level2' },
		{ key: 'state', label: 'State', example: 'e.g. FCT', autocomplete: 'address-level1' },
		{ key: 'country', label: 'Country', example: 'e.g. Nigeria', autocomplete: 'country-name' }
	],
	employment: [
		{ key: 'employer_name', label: 'Current employer', example: 'e.g. Bujeti' },
		{ key: 'occupation', label: 'Occupation or job title', example: 'e.g. Backend Engineer' },
		{ key: 'employment_status', label: 'Employment status', example: 'e.g. Full-time employee' },
		{
			key: 'employer_address',
			label: 'Employer address',
			example: "Enter your employer's address",
			input: 'textarea'
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
		{
			key: 'next_of_kin_phone',
			label: 'Next of kin phone',
			example: 'e.g. +234 800 000 0000',
			input: 'tel'
		},
		{
			key: 'emergency_contact_name',
			label: 'Emergency contact name',
			example: 'Enter their full name'
		},
		{
			key: 'emergency_contact_phone',
			label: 'Emergency contact phone',
			example: 'e.g. +234 800 000 0000',
			input: 'tel'
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
	],
	custom: []
};

export type OnboardingField = {
	namespace: string;
	key: string;
};

export type OnboardingStep = {
	id: string;
	title: string;
	prompt: string;
	fields: OnboardingField[];
};

export const onboardingSteps: OnboardingStep[] = [
	{
		id: 'about',
		title: 'About you',
		prompt:
			'Start with the details forms ask for most often. Skip anything you would rather add later.',
		fields: [
			{ namespace: 'identity', key: 'full_name' },
			{ namespace: 'contact', key: 'phone_number' },
			{ namespace: 'identity', key: 'date_of_birth' },
			{ namespace: 'identity', key: 'nationality' }
		]
	},
	{
		id: 'home',
		title: 'Where you live',
		prompt:
			'An address lets Docufill fill residence, postal, and location fields without asking again.',
		fields: [
			{ namespace: 'address', key: 'residential_address' },
			{ namespace: 'address', key: 'city' },
			{ namespace: 'address', key: 'state' },
			{ namespace: 'address', key: 'country' }
		]
	},
	{
		id: 'work',
		title: 'Work',
		prompt:
			'Employment details show up on HR, KYC, and school forms. Leave a field blank if it does not apply.',
		fields: [
			{ namespace: 'employment', key: 'occupation' },
			{ namespace: 'employment', key: 'employer_name' },
			{ namespace: 'employment', key: 'employment_status' }
		]
	}
];

export type DraftAnswers = Record<string, string>;

export type ProfileFactInput = {
	namespace: string;
	fact_key: string;
	value: string;
	value_type: string;
	sensitivity: string;
};

export function answerKey(namespace: string, key: string): string {
	return `${namespace}:${key}`;
}

export function findFact(namespace: string, key: string): FactDefinition | undefined {
	return factCatalog[namespace]?.find((fact) => fact.key === key);
}

export function factLabel(key: string): string {
	for (const facts of Object.values(factCatalog)) {
		const match = facts.find((fact) => fact.key === key);
		if (match) return match.label;
	}
	return key.replaceAll('_', ' ').replace(/\b\w/g, (letter) => letter.toUpperCase());
}

export function sectionLabel(namespace: string): string {
	return factSections.find((section) => section.id === namespace)?.label ?? factLabel(namespace);
}

export function requireFact(field: OnboardingField): FactDefinition {
	const definition = findFact(field.namespace, field.key);
	if (!definition) {
		throw new Error(`Unknown onboarding field ${field.namespace}.${field.key}`);
	}
	return definition;
}

export function emptyOnboardingAnswers(): DraftAnswers {
	const answers: DraftAnswers = {};
	for (const step of onboardingSteps) {
		for (const field of step.fields) {
			answers[answerKey(field.namespace, field.key)] = '';
		}
	}
	return answers;
}

export function filledFactsFromAnswers(answers: DraftAnswers): ProfileFactInput[] {
	const facts: ProfileFactInput[] = [];
	for (const step of onboardingSteps) {
		for (const field of step.fields) {
			const value = answers[answerKey(field.namespace, field.key)]?.trim() ?? '';
			if (!value) continue;
			const definition = requireFact(field);
			facts.push({
				namespace: field.namespace,
				fact_key: field.key,
				value,
				value_type: 'text',
				sensitivity: definition.sensitive ? 'sensitive' : 'personal'
			});
		}
	}
	return facts;
}

export function emailFact(email: string): ProfileFactInput | null {
	const value = email.trim();
	if (!value) return null;
	return {
		namespace: 'contact',
		fact_key: 'email_address',
		value,
		value_type: 'text',
		sensitivity: 'personal'
	};
}
