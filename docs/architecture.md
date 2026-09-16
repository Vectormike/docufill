# Docufill architecture

## Runtime

- `apps/web`: mobile-first SvelteKit PWA. It caches only the public shell and static assets. Profile, document, participant, account, and API routes remain network-only.
- `services/core --bin api`: Axum API for authenticated owners and scoped participant sessions.
- `services/core --bin worker`: asynchronous extraction, mapping, rendering, and memory jobs. Stale jobs are reclaimed after 15 minutes.
- Supabase: Google/email authentication, PostgreSQL, private object storage, Realtime status updates, and pgvector.
- PDFium: digital-PDF field discovery, text extraction, native field filling, coordinate overlays, and rendering.

## Trust boundaries

Owner requests use a validated Supabase bearer token. Signature, completion, and account deletion also require recent authentication based on Supabase authentication method references or the server-confirmed last sign-in time.

Participants use a 256-bit-equivalent random link plus a six-digit code sent to the invited email. The code expires after 15 minutes, permits five attempts, and can be resent once per minute. The resulting scoped session expires after two hours and can access only fields assigned to that participant.

Canonical profile facts, signatures, answers, context, extraction caches, and approved memory excerpts are application-encrypted. Storage buckets are private. Download URLs expire after five minutes.

## Document lifecycle

1. The browser validates and hashes a PDF, then uploads it directly to the owner's private storage prefix.
2. The API validates ownership of the path and queues extraction.
3. The worker verifies the content hash, rejects malformed/encrypted/oversized PDFs, checks the encrypted per-owner extraction cache, and extracts native or inferred fields.
4. Exact confirmed facts are mapped deterministically. Up to four minimal-context AI mapping requests run concurrently; unavailable AI degrades to missing questions.
5. The owner confirms answers or assigns fields. Any change invalidates the current preview.
6. A worker renders a preview from the immutable original. Completion is impossible through the API until a current preview exists.
7. After recent authentication and explicit consent, a separate final render applies the selected signature. The final hash, signature version, authentication method, consent hash, and timestamps are audited.

Generated artifacts are tracked so account or document deletion removes every stored version. Originals are never modified.

