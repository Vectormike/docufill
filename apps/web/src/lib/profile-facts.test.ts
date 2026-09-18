import { describe, expect, it } from 'vitest';
import {
	answerKey,
	emailFact,
	emptyOnboardingAnswers,
	factLabel,
	filledFactsFromAnswers,
	onboardingSteps,
	requireFact,
	sectionLabel
} from './profile-facts';

describe('profile fact catalog', () => {
	it('keeps onboarding fields inside the shared catalog', () => {
		const asked = onboardingSteps.flatMap((step) =>
			step.fields.map((field) => `${field.namespace}.${field.key}`)
		);
		for (const step of onboardingSteps) {
			for (const field of step.fields) {
				expect(requireFact(field).key).toBe(field.key);
			}
		}
		expect(asked).not.toContain('identification.nin');
		expect(asked).not.toContain('financial.account_number');
		expect(asked).toContain('identity.full_name');
		expect(asked).toContain('address.residential_address');
	});

	it('labels known keys from the catalog', () => {
		expect(factLabel('full_name')).toBe('Full name');
		expect(sectionLabel('employment')).toBe('Employment');
		expect(factLabel('preferred_name')).toBe('Preferred Name');
	});

	it('omits blank answers', () => {
		const answers = emptyOnboardingAnswers();
		answers[answerKey('identity', 'full_name')] = '  Victor Jonah  ';
		answers[answerKey('contact', 'phone_number')] = ' ';
		answers[answerKey('address', 'country')] = 'Nigeria';

		expect(filledFactsFromAnswers(answers)).toEqual([
			{
				namespace: 'identity',
				fact_key: 'full_name',
				value: 'Victor Jonah',
				value_type: 'text',
				sensitivity: 'personal'
			},
			{
				namespace: 'address',
				fact_key: 'country',
				value: 'Nigeria',
				value_type: 'text',
				sensitivity: 'personal'
			}
		]);
	});

	it('marks date of birth as sensitive and can add a verified email', () => {
		const answers = emptyOnboardingAnswers();
		answers[answerKey('identity', 'date_of_birth')] = '1995-05-14';
		expect(filledFactsFromAnswers(answers)).toEqual([
			{
				namespace: 'identity',
				fact_key: 'date_of_birth',
				value: '1995-05-14',
				value_type: 'text',
				sensitivity: 'sensitive'
			}
		]);
		expect(emailFact(' victor@example.com ')).toMatchObject({
			namespace: 'contact',
			fact_key: 'email_address',
			value: 'victor@example.com'
		});
		expect(emailFact('')).toBeNull();
	});
});
