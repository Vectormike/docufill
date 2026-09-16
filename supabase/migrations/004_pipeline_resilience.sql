create table public.document_extractions (
  id uuid primary key default gen_random_uuid(),
  owner_id uuid not null references public.profiles(id) on delete cascade,
  content_hash text not null check (length(content_hash) = 64),
  analyzer_version text not null,
  payload_ciphertext bytea not null,
  created_at timestamptz not null default now(),
  unique (owner_id, content_hash, analyzer_version)
);

create index document_extractions_owner_idx
on public.document_extractions (owner_id, created_at desc);

alter table public.document_extractions enable row level security;

-- Extraction cache contains encrypted document text and is worker-only.
revoke all on public.document_extractions from anon, authenticated;
grant select, insert, delete on public.document_extractions to service_role;

create or replace function public.claim_processing_job(worker_name text)
returns setof public.processing_jobs
language plpgsql
security definer
set search_path = ''
as $$
begin
  return query
  update public.processing_jobs
  set
    status = 'running',
    locked_at = now(),
    locked_by = worker_name,
    attempts = attempts + 1,
    updated_at = now()
  where id = (
    select id
    from public.processing_jobs
    where (
      status = 'queued' and run_after <= now()
    ) or (
      status = 'running' and locked_at < now() - interval '15 minutes'
    )
    order by created_at
    for update skip locked
    limit 1
  )
  returning *;
end;
$$;

revoke all on function public.claim_processing_job(text)
from public, anon, authenticated;
grant execute on function public.claim_processing_job(text)
to service_role;

