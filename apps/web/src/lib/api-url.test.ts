import { describe, expect, it } from 'vitest';
import { resolveApiBaseUrl } from './api-url';

describe('API base URL', () => {
	it('keeps a configured remote API host', () => {
		expect(resolveApiBaseUrl('https://api.example.com', 'app.example.com')).toBe(
			'https://api.example.com'
		);
	});

	it('aligns loopback hosts so 127.0.0.1 pages call 127.0.0.1', () => {
		expect(resolveApiBaseUrl('http://localhost:8080', '127.0.0.1')).toBe('http://127.0.0.1:8080');
		expect(resolveApiBaseUrl('http://127.0.0.1:8080', 'localhost')).toBe('http://localhost:8080');
	});

	it('leaves a loopback API alone when the page host is missing', () => {
		expect(resolveApiBaseUrl('http://localhost:8080/')).toBe('http://localhost:8080');
	});
});
