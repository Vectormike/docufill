#!/usr/bin/env bash
set -euo pipefail

if [[ -z "${DATABASE_URL:-}" ]]; then
  echo "DATABASE_URL is required to apply migrations" >&2
  exit 1
fi

export PGCONNECT_TIMEOUT="${PGCONNECT_TIMEOUT:-15}"

psql "$DATABASE_URL" -v ON_ERROR_STOP=1 <<'SQL'
create schema if not exists supabase_migrations;
create table if not exists supabase_migrations.schema_migrations (
  version text primary key,
  statements text[],
  name text
);
SQL

shopt -s nullglob
migrations=(/migrations/*.sql)
if [[ ${#migrations[@]} -eq 0 ]]; then
  echo "no SQL files in /migrations" >&2
  exit 1
fi

for file in "${migrations[@]}"; do
  filename="$(basename "$file")"
  version="${filename%.sql}"
  applied="$(psql "$DATABASE_URL" -tAc "select 1 from supabase_migrations.schema_migrations where version = '${version}'")"
  if [[ "${applied}" == "1" ]]; then
    echo "skip ${filename}"
    continue
  fi

  echo "apply ${filename}"
  psql "$DATABASE_URL" -v ON_ERROR_STOP=1 -f "$file"
  psql "$DATABASE_URL" -v ON_ERROR_STOP=1 \
    -c "insert into supabase_migrations.schema_migrations (version, name) values ('${version}', '${filename}')"
done

echo "migrations complete"
