create function public.match_document_chunks(
  query_embedding vector(1536),
  match_owner uuid,
  match_count integer default 5
)
returns table (
  id uuid,
  document_id uuid,
  page_number integer,
  content_ciphertext bytea,
  similarity double precision
)
language sql
stable
security definer
set search_path = ''
as $$
  select
    c.id,
    c.document_id,
    c.page_number,
    c.content_ciphertext,
    1 - (c.embedding OPERATOR(public.<=>) query_embedding) as similarity
  from public.document_chunks c
  where c.owner_id = match_owner
    and c.embedding is not null
    and c.approved_at is not null
  order by c.embedding OPERATOR(public.<=>) query_embedding
  limit least(greatest(match_count, 1), 20);
$$;

create function public.claim_processing_job(worker_name text)
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
    where status = 'queued' and run_after <= now()
    order by created_at
    for update skip locked
    limit 1
  )
  returning *;
end;
$$;

create function public.prevent_audit_mutation()
returns trigger
language plpgsql
set search_path = ''
as $$
begin
  raise exception 'audit events are append-only';
end;
$$;

create trigger audit_events_immutable
before update or delete on public.audit_events
for each row execute function public.prevent_audit_mutation();

revoke all on function public.match_document_chunks(vector, uuid, integer)
from public, anon, authenticated;
revoke all on function public.claim_processing_job(text)
from public, anon, authenticated;
grant execute on function public.match_document_chunks(vector, uuid, integer)
to service_role;
grant execute on function public.claim_processing_job(text)
to service_role;

