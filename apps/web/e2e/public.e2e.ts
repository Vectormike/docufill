import AxeBuilder from '@axe-core/playwright';
import { expect, test } from '@playwright/test';

test('landing experience is accessible, responsive, and fast', async ({ page }) => {
	await page.goto('/');

	await expect(page).toHaveTitle(/Docufill.*Fill forms once/);
	await expect(page.getByRole('heading', { level: 1 })).toContainText('Fill forms once');
	await expect(page.getByRole('button', { name: 'Continue with Google' })).toBeVisible();
	await expect(page.getByText('optional questions', { exact: false })).toBeVisible();

	const accessibility = await new AxeBuilder({ page }).analyze();
	expect(
		accessibility.violations.filter(
			(violation) => violation.impact === 'critical' || violation.impact === 'serious'
		)
	).toEqual([]);

	const metrics = await page.evaluate(() => {
		const navigation = performance.getEntriesByType('navigation')[0] as PerformanceNavigationTiming;
		return {
			duration: navigation.duration,
			overflow: document.documentElement.scrollWidth - document.documentElement.clientWidth
		};
	});
	expect(metrics.duration).toBeLessThan(5_000);
	expect(metrics.overflow).toBeLessThanOrEqual(1);
});

test('reduced motion removes long onboarding animation', async ({ page }) => {
	await page.emulateMedia({ reducedMotion: 'reduce' });
	await page.goto('/');

	const duration = await page.locator('.logo-drop').evaluate((element) => {
		return Number.parseFloat(getComputedStyle(element).animationDuration);
	});
	expect(duration).toBeLessThanOrEqual(0.01);
});

test('the app renders in light mode only', async ({ page }) => {
	await page.emulateMedia({ colorScheme: 'dark' });
	await page.goto('/');

	const canvas = await page
		.locator('body')
		.evaluate((element) => getComputedStyle(element).colorScheme);
	expect(canvas).toBe('light');
	await expect(page.getByRole('button', { name: /dark theme/i })).toHaveCount(0);
});

test('privacy, scope, and PWA metadata are public', async ({ page, request }) => {
	await page.goto('/privacy');
	await expect(page.getByRole('heading', { name: 'Privacy notice' })).toBeVisible();
	await expect(page.getByText('Nigeria Data Protection Commission')).toBeVisible();

	await page.goto('/acceptable-use');
	await expect(
		page.getByRole('heading', { name: 'Supported and excluded documents' })
	).toBeVisible();
	await expect(page.getByText('Wills, testamentary documents', { exact: false })).toBeVisible();

	const manifest = await request.get('/manifest.webmanifest');
	expect(manifest.ok()).toBe(true);
	const metadata = await manifest.json();
	expect(metadata).toMatchObject({
		name: 'Docufill',
		display: 'standalone',
		theme_color: '#fbfaf8'
	});
	for (const icon of metadata.icons as Array<{ src: string }>) {
		expect((await request.get(icon.src)).ok()).toBe(true);
	}
});
