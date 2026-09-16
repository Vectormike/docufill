import type { components } from './generated/schema';
import { currentSession } from '$lib/auth';
import { apiBaseUrl } from '$lib/supabase';

type Schema<Name extends keyof components['schemas']> = components['schemas'][Name];

export type ProfileVault = Schema<'ProfileVault'>;
export type ProfileFact = Schema<'ProfileFact'>;
export type DocumentSummary = Schema<'DocumentSummary'>;
export type DocumentDetail = Schema<'DocumentDetail'>;
export type DocumentField = Schema<'DocumentField'>;
export type ParticipantSummary = Schema<'ParticipantSummary'>;
export type ParticipantInvitation = Schema<'ParticipantInvitation'>;
export type ParticipantLanding = Schema<'ParticipantLanding'>;
export type ParticipantAssignment = Schema<'ParticipantAssignment'>;
export type ParticipantReceipt = Schema<'ParticipantReceipt'>;

export class ApiError extends Error {
	constructor(
		message: string,
		public readonly code: string,
		public readonly status: number
	) {
		super(message);
	}
}

async function request<T>(
	path: string,
	init: RequestInit = {},
	options: { public?: boolean; participantSession?: string } = {}
): Promise<T> {
	const headers = new Headers(init.headers);
	if (init.body) headers.set('content-type', 'application/json');
	if (options.participantSession) {
		headers.set('x-participant-session', options.participantSession);
	} else if (!options.public) {
		const session = await currentSession();
		if (!session) throw new ApiError('Please sign in to continue.', 'unauthorized', 401);
		headers.set('authorization', `Bearer ${session.access_token}`);
	}

	const response = await fetch(`${apiBaseUrl()}${path}`, { ...init, headers, cache: 'no-store' });
	if (!response.ok) {
		const payload = await response.json().catch(() => null);
		throw new ApiError(
			payload?.error?.message ?? 'Something went wrong. Please try again.',
			payload?.error?.code ?? 'request_failed',
			response.status
		);
	}
	if (response.status === 204) return undefined as T;
	return response.json() as Promise<T>;
}

export const api = {
	exportAccount: () => request<Record<string, unknown>>('/v1/account'),
	deleteAccount: () =>
		request<void>('/v1/account', {
			method: 'DELETE',
			body: JSON.stringify({ confirmation: 'DELETE' })
		}),
	profile: () => request<ProfileVault>('/v1/profile'),
	updateProfile: (input: Schema<'UpdateProfile'>) =>
		request<Schema<'Profile'>>('/v1/profile', {
			method: 'PATCH',
			body: JSON.stringify(input)
		}),
	saveFact: (input: Schema<'UpsertProfileFact'>) =>
		request<ProfileFact>('/v1/profile/facts', {
			method: 'POST',
			body: JSON.stringify(input)
		}),
	deleteFact: (factId: string) =>
		request<void>(`/v1/profile/facts/${factId}`, { method: 'DELETE' }),
	saveSignature: (input: Schema<'CreateSignature'>) =>
		request<Schema<'SignatureSummary'>>('/v1/profile/signature', {
			method: 'POST',
			body: JSON.stringify(input)
		}),
	deleteSignature: () => request<void>('/v1/profile/signature', { method: 'DELETE' }),

	documents: () => request<DocumentSummary[]>('/v1/documents'),
	createDocument: (input: Schema<'CreateDocument'>) =>
		request<DocumentSummary>('/v1/documents', {
			method: 'POST',
			body: JSON.stringify(input)
		}),
	document: (documentId: string) => request<DocumentDetail>(`/v1/documents/${documentId}`),
	deleteDocument: (documentId: string) =>
		request<void>(`/v1/documents/${documentId}`, { method: 'DELETE' }),
	updateField: (documentId: string, fieldId: string, input: Schema<'UpdateFieldAnswer'>) =>
		request<DocumentField>(`/v1/documents/${documentId}/fields/${fieldId}`, {
			method: 'PATCH',
			body: JSON.stringify(input)
		}),
	clearField: (documentId: string, fieldId: string) =>
		request<void>(`/v1/documents/${documentId}/fields/${fieldId}`, {
			method: 'DELETE'
		}),
	assignField: (documentId: string, fieldId: string, participantId: string | null) =>
		request<void>(`/v1/documents/${documentId}/fields/${fieldId}/assignment`, {
			method: 'PATCH',
			body: JSON.stringify({ participant_id: participantId })
		}),
	updateFieldLayout: (documentId: string, fieldId: string, input: Schema<'UpdateFieldLayout'>) =>
		request<void>(`/v1/documents/${documentId}/fields/${fieldId}/layout`, {
			method: 'PATCH',
			body: JSON.stringify(input)
		}),
	addContext: (documentId: string, content: string) =>
		request<void>(`/v1/documents/${documentId}/context`, {
			method: 'POST',
			body: JSON.stringify({ content })
		}),
	setMemory: (documentId: string, enabled: boolean, approvedFieldIds: string[] = []) =>
		request<void>(`/v1/documents/${documentId}/memory`, {
			method: 'PATCH',
			body: JSON.stringify({ enabled, approved_field_ids: approvedFieldIds })
		}),
	complete: (documentId: string, input: Schema<'CompleteDocument'>) =>
		request<Schema<'CompletionQueued'>>(`/v1/documents/${documentId}/complete`, {
			method: 'POST',
			body: JSON.stringify(input)
		}),
	requestPreview: (documentId: string) =>
		request<Schema<'CompletionQueued'>>(`/v1/documents/${documentId}/preview`, {
			method: 'POST'
		}),
	preview: (documentId: string) =>
		request<Schema<'DownloadUrl'>>(`/v1/documents/${documentId}/preview`),
	download: (documentId: string) =>
		request<Schema<'DownloadUrl'>>(`/v1/documents/${documentId}/download`),

	participants: (documentId: string) =>
		request<ParticipantSummary[]>(`/v1/documents/${documentId}/participants`),
	inviteParticipant: (documentId: string, input: Schema<'CreateParticipant'>) =>
		request<ParticipantInvitation>(`/v1/documents/${documentId}/participants`, {
			method: 'POST',
			body: JSON.stringify(input)
		}),
	revokeParticipant: (documentId: string, participantId: string) =>
		request<void>(`/v1/documents/${documentId}/participants/${participantId}`, {
			method: 'DELETE'
		}),

	shareLanding: (token: string) =>
		request<ParticipantLanding>(`/v1/share/${encodeURIComponent(token)}`, {}, { public: true }),
	verifyShare: (token: string, code: string) =>
		request<Schema<'ParticipantSession'>>(
			`/v1/share/${encodeURIComponent(token)}/verify`,
			{ method: 'POST', body: JSON.stringify({ code }) },
			{ public: true }
		),
	requestShareVerificationCode: (token: string) =>
		request<void>(
			`/v1/share/${encodeURIComponent(token)}/verification-code`,
			{ method: 'POST' },
			{ public: true }
		),
	assignment: (token: string, participantSession: string) =>
		request<ParticipantAssignment>(
			`/v1/share/${encodeURIComponent(token)}/assignment`,
			{},
			{ public: true, participantSession }
		),
	answerAssignedField: (
		token: string,
		fieldId: string,
		value: string,
		participantSession: string
	) =>
		request<DocumentField>(
			`/v1/share/${encodeURIComponent(token)}/fields/${fieldId}`,
			{ method: 'PATCH', body: JSON.stringify({ value }) },
			{ public: true, participantSession }
		),
	submitAssignment: (token: string, participantSession: string) =>
		request<ParticipantReceipt>(
			`/v1/share/${encodeURIComponent(token)}/submit`,
			{ method: 'POST' },
			{ public: true, participantSession }
		)
};
