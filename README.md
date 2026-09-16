# Docufill

Docufill is a privacy-first personal document agent that turns documents into guided questions, reuses confirmed information, coordinates participants, and produces clean completed PDFs.

## Repository structure

```text
apps/
  web/                    SvelteKit PWA
    src/
      lib/
        api/generated/    Generated Rust API client
        components/       Shared UI components
      routes/
        (app)/            Authenticated owner experience
        share/[token]/    Scoped participant experience
    static/icons/         PWA and brand icons
services/
  core/                   Shared Rust domain service
    src/
      ai/                 Grounded document interpretation
      bin/                API and worker entry points
      memory/             Structured facts and RAG retrieval
      pdf/                PDF extraction, filling, and rendering
      profile/            Profile facts and derivations
      security/           Encryption and authorization helpers
supabase/
  migrations/             Database schema and access policies
tests/
  fixtures/pdfs/          Synthetic PDF fixtures only
docs/                     Architecture and operational documentation
```

## Local development

1. Install Node.js, pnpm, Rust, PDFium, and a local or hosted Supabase project.
2. Copy each `.env.example` to the corresponding local `.env` and provide development credentials. Never commit these files.
3. Apply `supabase/migrations` in numeric order.
4. Run `pnpm dev`, `pnpm dev:api`, and `pnpm dev:worker` in separate terminals.

Useful checks:

```sh
pnpm check
pnpm lint
pnpm test
pnpm build
cargo clippy --workspace --all-targets -- -D warnings
```

See `docs/architecture.md`, `docs/security-and-privacy.md`, and `docs/launch-gates.md` before deploying.

