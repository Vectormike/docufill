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
docker/                   Production images and Caddy config
scripts/deploy.sh         Single-command Docker Compose deploy
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

## Deploy

One command starts Caddy, the SvelteKit app, the API, and the worker. Install Docker, copy env, then ship:

```sh
cp .env.example .env
# fill in Supabase, encryption, and API secrets
./scripts/deploy.sh
```

`./scripts/deploy.sh` applies `supabase/migrations` in order, builds the images (PDFium and the Unicode font are included), and waits until `/health` succeeds.

- Local HTTP: keep `APP_DOMAIN=:80` and both `PUBLIC_APP_URL` / `PUBLIC_API_URL` as `http://localhost`. Open the same host you set; `localhost` and `127.0.0.1` are different CORS origins.
- Production: point DNS at the host, set `APP_DOMAIN` to that hostname, use `https://` public URLs, `APP_ENV=production`, and the launch-gate flags in `docs/launch-gates.md`. Caddy terminates TLS.
- If the schema is already applied: `SKIP_MIGRATE=1 ./scripts/deploy.sh`
- `DATABASE_URL` must be a direct Supabase Postgres URI (port `5432`, `sslmode=require`), not the pooler.

See `docs/architecture.md`, `docs/security-and-privacy.md`, and `docs/launch-gates.md` before a public production launch.

