# Security and privacy controls

## Data minimisation

- OAuth profile use is limited to identity and avatar metadata returned by Supabase.
- Direct Gmail access is not included in the MVP.
- AI field mapping receives a field and at most six relevant confirmed facts, never the full vault.
- AI suggestions remain unconfirmed drafts; identity and financial values are never invented.
- Optional document memory excludes known sensitive identifiers and stores encrypted content separately from vectors.
- Customer content is not intended for model training; production AI terms must be approved before startup.

## Access controls

- PostgreSQL row-level security is enabled for every user-data table.
- Worker-only session, extraction-cache, and artifact tables have no authenticated-client policy.
- Private storage paths begin with the authenticated owner's ID.
- Participant APIs authorize both the hashed invitation token and hashed, expiring session.
- Owners cannot answer fields while those fields are assigned to a participant.
- Assigning or revoking a participant clears prior field values to prevent cross-person disclosure.

## Cryptography and integrity

- TLS is required for production origins.
- Profile facts, answers, participant contacts, signatures, text context, extraction caches, and memory excerpts use AES-256-GCM with a random nonce.
- Passwords and raw signature images are never logged.
- Document, consent, contact, invitation, verification, and memory hashes use SHA-256.
- Signature uploads accept only decodable PNG/JPEG data, are resized, converted to PNG, encrypted, and stored separately.
- Final document hashes and non-secret consent metadata are recorded in append-only audit events.

## Retention and incident operations

- Users can export account metadata and decrypted confirmed facts.
- Document deletion removes the original, tracked previews/finals, context objects, database records, and memory.
- Account deletion removes all tracked storage objects before deleting the Supabase identity and cascading application records.
- Participant links, codes, sessions, and signed downloads expire after seven days, 15 minutes, two hours, and five minutes respectively.
- Rotate the application encryption key only with a planned decrypt/re-encrypt migration; changing it directly makes existing ciphertext unreadable.
- On a suspected service-role or encryption-key exposure, disable affected services, rotate credentials, preserve non-sensitive operational logs, assess impacted records, and follow the approved Nigerian breach-response process.

Security controls reduce risk but do not replace penetration testing, provider configuration review, DPIA, or legal review before public production use.

