import { readFile } from 'node:fs/promises';
import { describe, expect, it } from 'vitest';

describe('service worker privacy boundary', () => {
	it('keeps sensitive application routes network-only', async () => {
		const source = await readFile(new URL('../service-worker.ts', import.meta.url), 'utf8');

		for (const route of ['/documents', '/profile', '/settings', '/share', '/auth']) {
			expect(source).toContain(`'${route}'`);
		}
		expect(source).toContain("request.method !== 'GET'");
		expect(source).toContain('fetch(request).catch');
	});
});
