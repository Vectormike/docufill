#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"

if ! command -v docker >/dev/null 2>&1; then
  echo "Docker is required." >&2
  exit 1
fi

if ! docker compose version >/dev/null 2>&1; then
  echo "Docker Compose v2 is required." >&2
  exit 1
fi

if [[ ! -f .env ]]; then
  cp .env.example .env
  echo "Created .env from .env.example. Fill in secrets, then run ./scripts/deploy.sh again." >&2
  exit 1
fi

env_value() {
  local line value
  line="$(grep -E "^${1}=" .env | tail -n1 || true)"
  value="${line#*=}"
  value="${value%\"}"
  value="${value#\"}"
  value="${value%\'}"
  value="${value#\'}"
  printf '%s' "$value"
}

required=(
  APP_DOMAIN
  PUBLIC_APP_URL
  PUBLIC_API_URL
  PUBLIC_SUPABASE_URL
  PUBLIC_SUPABASE_ANON_KEY
  DATABASE_URL
  SUPABASE_URL
  SUPABASE_ANON_KEY
  SUPABASE_SERVICE_ROLE_KEY
  ENCRYPTION_KEY
)

missing=0
for key in "${required[@]}"; do
  value="$(env_value "$key")"
  if [[ -z "$value" || "$value" == your-* || "$value" == *PASSWORD* || "$value" == replace-with-* ]]; then
    echo "Set $key in .env" >&2
    missing=1
  fi
done
if [[ "$missing" -ne 0 ]]; then
  exit 1
fi

app_url="$(env_value PUBLIC_APP_URL)"

if [[ "${SKIP_MIGRATE:-}" != "1" ]]; then
  echo "Applying Supabase migrations..."
  docker compose --profile migrate run --rm migrate
fi

echo "Building and starting Docufill..."
docker compose up --build -d

echo "Waiting for API health..."
for _ in $(seq 1 60); do
  if docker compose exec -T api curl -fsS http://127.0.0.1:8080/health >/dev/null 2>&1; then
    docker compose ps
    echo "Docufill is up at ${app_url}"
    exit 0
  fi
  sleep 2
done

echo "API did not become healthy. Check logs with: docker compose logs" >&2
docker compose ps >&2
exit 1
