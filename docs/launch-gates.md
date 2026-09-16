# Nigeria MVP launch gates

Production must remain blocked until every item below has evidence and an accountable owner.

## Product and legal

- [ ] Nigerian counsel has approved the completion-first scope, signature consent text, privacy notice, participant notice, excluded-document policy, and electronic-signature limitations.
- [ ] A Nigeria Data Protection Act/GAID assessment and DPIA are complete.
- [ ] Controller identity, privacy contact, processor list, international-transfer basis, backup retention, and data-subject request procedure are published.
- [ ] AI provider data-processing terms prohibit training on customer content and meet the approved transfer/retention requirements.
- [ ] Support and incident-response owners are assigned.

Set `DPIA_COMPLETED`, `LEGAL_REVIEW_APPROVED`, and `AI_DATA_PROCESSING_TERMS_APPROVED` to `true` only after evidence is recorded. The API refuses to start in production without these flags, HTTPS origins, email delivery, AI/embedding configuration, and `PRIVACY_CONTACT_EMAIL`.

## Security

- [ ] Apply every Supabase migration in order and verify RLS with separate owner, participant, anonymous, and service-role sessions.
- [ ] Keep service-role, encryption, email, and AI keys in the deployment secret manager; confirm none are shipped to the web bundle.
- [ ] Enable MFA and least-privilege production administration.
- [ ] Complete dependency, SAST, secret, storage-policy, and external penetration tests.
- [ ] Exercise account/document deletion and confirm tracked objects are absent from private storage.
- [ ] Verify backup retention, restoration, key rotation, breach notification, and provider outage procedures.

## Quality and performance

- [ ] `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, web type checks, lint, unit tests, production build, and Playwright desktop/mobile tests pass.
- [ ] Test representative Nigerian employment, rental, school, and guarantor PDFs without real personal data.
- [ ] Reject image-only, handwritten, encrypted, malformed, over-25-MB, over-100-page, and excluded/high-risk documents.
- [ ] Confirm every answer edit or reassignment invalidates the preview and that completion requires reviewing a newly rendered PDF.
- [ ] Run `k6 run tests/load/api.js` against staging. Required thresholds are under 1% errors, health p95 below 250 ms, and authenticated document-read p95 below 750 ms.
- [ ] On representative mid-tier Android hardware and Nigerian mobile networks, meet LCP below 2.5 s, INP below 200 ms, CLS below 0.1, and verify the network-only sensitive-route boundary.

## Release decision

Record the deployed commit, migration versions, test evidence, unresolved risks, approvers, rollback owner, and release time. No checkbox may be waived silently.

