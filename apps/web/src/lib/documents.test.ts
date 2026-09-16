import { describe, expect, it } from 'vitest';
import { validatePdf } from './documents';

function file(name: string, type: string, size: number): File {
	return { name, type, size } as File;
}

describe('PDF upload validation', () => {
	it('accepts a PDF within the upload budget', () => {
		expect(() => validatePdf(file('form.pdf', 'application/pdf', 2_048))).not.toThrow();
	});

	it('rejects oversized files before upload', () => {
		expect(() => validatePdf(file('large.pdf', 'application/pdf', 25 * 1024 * 1024 + 1))).toThrow(
			'smaller than 25 MB'
		);
	});

	it('rejects non-PDF content without a PDF extension', () => {
		expect(() => validatePdf(file('signature.png', 'image/png', 2_048))).toThrow('Choose a PDF');
	});
});
