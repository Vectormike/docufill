import { api } from '$lib/api/client';
import { getSupabase } from '$lib/supabase';

const MAX_PDF_BYTES = 25 * 1024 * 1024;

export async function uploadDocument(
	file: File,
	onProgress: (progress: number, stage: string) => void
) {
	validatePdf(file);
	const {
		data: { user },
		error: userError
	} = await getSupabase().auth.getUser();
	if (userError || !user) throw new Error('Please sign in before uploading a document.');

	onProgress(8, 'Checking your PDF');
	const contentHash = await sha256(file);
	const storagePath = `${user.id}/${crypto.randomUUID()}/original.pdf`;
	onProgress(20, 'Uploading securely');
	const { error: uploadError } = await getSupabase()
		.storage.from('documents')
		.upload(storagePath, file, {
			contentType: 'application/pdf',
			cacheControl: '0',
			upsert: false
		});
	if (uploadError) throw uploadError;

	onProgress(76, 'Starting document analysis');
	try {
		const document = await api.createDocument({
			subject: subjectFromFilename(file.name),
			original_name: file.name,
			original_storage_path: storagePath,
			content_hash: contentHash
		});
		onProgress(100, 'Upload complete');
		return document;
	} catch (error) {
		await getSupabase().storage.from('documents').remove([storagePath]);
		throw error;
	}
}

export function describeFailure(errorCode: string | null | undefined) {
	switch (errorCode) {
		case 'document_missing':
			return {
				title: 'The original file is no longer stored',
				detail:
					'Your answers are safe, but the uploaded PDF itself is gone, so it cannot be filled or previewed. Upload the form again to continue.',
				retryable: false
			};
		case 'service_not_configured':
			return {
				title: 'Document processing is unavailable',
				detail:
					'This is a problem on our side, not with your PDF. Please try again shortly — nothing you entered has been lost.',
				retryable: true
			};
		case 'upstream_unavailable':
			return {
				title: 'Processing was interrupted',
				detail:
					'A service we depend on did not respond. Your PDF looks fine, so retrying usually works.',
				retryable: true
			};
		default:
			return {
				title: 'This PDF needs attention',
				detail:
					'It may be encrypted, image-only, malformed, oversized, or require a PDF feature not supported in this release.',
				retryable: true
			};
	}
}

export function subjectFromFilename(filename: string) {
	const cleaned = filename
		.replace(/\.pdf$/i, '')
		.replace(/[-_]+/g, ' ')
		.replace(/\s+/g, ' ')
		.trim();
	const concise =
		cleaned.match(/^(.+?\b(?:form|application|agreement|questionnaire|contract)\b)/i)?.[1] ??
		cleaned;
	return concise.slice(0, 100) || 'Untitled document';
}

export function validatePdf(file: File) {
	if (file.type !== 'application/pdf' && !file.name.toLowerCase().endsWith('.pdf')) {
		throw new Error('Choose a PDF file.');
	}
	if (file.size === 0 || file.size > MAX_PDF_BYTES) {
		throw new Error('PDFs must be smaller than 25 MB.');
	}
}

async function sha256(file: File): Promise<string> {
	const digest = await crypto.subtle.digest('SHA-256', await file.arrayBuffer());
	return Array.from(new Uint8Array(digest))
		.map((byte) => byte.toString(16).padStart(2, '0'))
		.join('');
}
