create table public.document_artifacts (
  id uuid primary key default gen_random_uuid(),
  document_id uuid not null references public.documents(id) on delete cascade,
  owner_id uuid not null references public.profiles(id) on delete cascade,
  kind text not null check (kind in ('preview', 'completed')),
  storage_path text not null unique,
  content_hash text not null,
  created_at timestamptz not null default now()
);

create index document_artifacts_document_idx
on public.document_artifacts (document_id, created_at desc);

alter table public.document_artifacts enable row level security;
revoke all on public.document_artifacts from anon, authenticated;
grant select, insert, delete on public.document_artifacts to service_role;

