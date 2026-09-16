create extension if not exists pgcrypto;
create extension if not exists vector;

create type public.document_status as enum (
  'uploaded', 'processing', 'needs_input', 'ready', 'completed', 'failed'
);
create type public.field_kind as enum (
  'text', 'multiline', 'date', 'number', 'email', 'phone', 'address',
  'choice', 'checkbox', 'radio', 'signature', 'declaration'
);
create type public.answer_source as enum (
  'missing', 'profile', 'derived', 'user', 'participant', 'ai_draft'
);
create type public.participant_status as enum (
  'draft', 'invited', 'viewed', 'verified', 'in_progress', 'completed', 'revoked'
);
create type public.job_status as enum (
  'queued', 'running', 'succeeded', 'failed', 'cancelled'
);

create table public.profiles (
  id uuid primary key references auth.users(id) on delete cascade,
  display_name text,
  avatar_url text,
  onboarding_completed boolean not null default false,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create table public.documents (
  id uuid primary key default gen_random_uuid(),
  owner_id uuid not null references public.profiles(id) on delete cascade,
  subject text not null,
  original_name text not null,
  original_storage_path text not null,
  preview_storage_path text,
  completed_storage_path text,
  content_hash text not null,
  status public.document_status not null default 'uploaded',
  page_count integer check (page_count is null or page_count between 1 and 100),
  progress smallint not null default 0 check (progress between 0 and 100),
  error_code text,
  memory_consent boolean not null default false,
  completed_at timestamptz,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  unique (owner_id, content_hash, id)
);

create table public.participants (
  id uuid primary key default gen_random_uuid(),
  document_id uuid not null references public.documents(id) on delete cascade,
  role text not null default 'guarantor',
  display_name text not null,
  contact_ciphertext bytea not null,
  contact_hash text not null,
  token_hash text unique,
  token_expires_at timestamptz,
  verification_code_hash text,
  verification_expires_at timestamptz,
  verification_attempts smallint not null default 0,
  status public.participant_status not null default 'draft',
  verified_at timestamptz,
  completed_at timestamptz,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create table public.profile_facts (
  id uuid primary key default gen_random_uuid(),
  user_id uuid not null references public.profiles(id) on delete cascade,
  namespace text not null,
  fact_key text not null,
  value_ciphertext bytea not null,
  value_preview text,
  value_type text not null default 'text',
  sensitivity text not null default 'personal',
  source_type public.answer_source not null default 'user',
  source_document_id uuid references public.documents(id) on delete set null,
  confirmed_at timestamptz not null default now(),
  superseded_by uuid references public.profile_facts(id) on delete set null
    deferrable initially deferred,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create table public.signatures (
  id uuid primary key default gen_random_uuid(),
  user_id uuid not null references public.profiles(id) on delete cascade,
  kind text not null check (kind in ('drawn', 'typed', 'uploaded')),
  storage_path text not null unique,
  content_hash text not null,
  version integer not null default 1,
  revoked_at timestamptz,
  created_at timestamptz not null default now()
);

create table public.document_contexts (
  id uuid primary key default gen_random_uuid(),
  document_id uuid not null references public.documents(id) on delete cascade,
  owner_id uuid not null references public.profiles(id) on delete cascade,
  kind text not null check (kind in ('text', 'file', 'forwarded_email')),
  content_ciphertext bytea,
  storage_path text,
  created_at timestamptz not null default now(),
  check (
    (kind = 'text' and content_ciphertext is not null and storage_path is null)
    or (kind <> 'text' and storage_path is not null)
  )
);

create table public.document_fields (
  id uuid primary key default gen_random_uuid(),
  document_id uuid not null references public.documents(id) on delete cascade,
  participant_id uuid references public.participants(id) on delete set null,
  field_key text not null,
  label text not null,
  instructions text,
  kind public.field_kind not null,
  page_number integer not null check (page_number > 0),
  x numeric not null,
  y numeric not null,
  width numeric not null check (width > 0),
  height numeric not null check (height > 0),
  font_size numeric check (font_size is null or font_size between 6 and 24),
  alignment text not null default 'left' check (alignment in ('left', 'center', 'right')),
  value_ciphertext bytea,
  value_preview text,
  source public.answer_source not null default 'missing',
  source_fact_id uuid references public.profile_facts(id) on delete set null,
  confidence numeric check (confidence is null or confidence between 0 and 1),
  confirmed_at timestamptz,
  sort_order integer not null default 0,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  unique (document_id, field_key)
);

create table public.answer_proposals (
  id uuid primary key default gen_random_uuid(),
  field_id uuid not null references public.document_fields(id) on delete cascade,
  source public.answer_source not null,
  value_ciphertext bytea not null,
  source_fact_ids uuid[] not null default '{}',
  explanation text,
  accepted_at timestamptz,
  rejected_at timestamptz,
  created_at timestamptz not null default now()
);

create table public.document_chunks (
  id uuid primary key default gen_random_uuid(),
  document_id uuid not null references public.documents(id) on delete cascade,
  owner_id uuid not null references public.profiles(id) on delete cascade,
  page_number integer not null check (page_number > 0),
  content_ciphertext bytea not null,
  content_hash text not null,
  embedding vector(1536),
  approved_at timestamptz not null,
  created_at timestamptz not null default now()
);

create table public.processing_jobs (
  id uuid primary key default gen_random_uuid(),
  document_id uuid not null references public.documents(id) on delete cascade,
  owner_id uuid not null references public.profiles(id) on delete cascade,
  kind text not null check (kind in ('extract', 'map', 'render', 'embed', 'delete')),
  status public.job_status not null default 'queued',
  payload jsonb not null default '{}',
  attempts smallint not null default 0,
  run_after timestamptz not null default now(),
  locked_at timestamptz,
  locked_by text,
  error_code text,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create table public.participant_sessions (
  id uuid primary key default gen_random_uuid(),
  participant_id uuid not null references public.participants(id) on delete cascade,
  token_hash text not null unique,
  expires_at timestamptz not null,
  created_at timestamptz not null default now()
);

create table public.audit_events (
  id bigint generated always as identity primary key,
  owner_id uuid not null references public.profiles(id) on delete cascade,
  document_id uuid references public.documents(id) on delete cascade,
  participant_id uuid references public.participants(id) on delete set null,
  actor_type text not null check (actor_type in ('owner', 'participant', 'system')),
  actor_id text,
  event_type text not null,
  document_hash text,
  metadata jsonb not null default '{}',
  ip_hash text,
  created_at timestamptz not null default now()
);

create index documents_owner_created_idx on public.documents(owner_id, created_at desc);
create index documents_owner_hash_idx on public.documents(owner_id, content_hash);
create index profile_facts_lookup_idx on public.profile_facts(user_id, namespace, fact_key)
  where superseded_by is null;
create unique index profile_facts_current_idx
  on public.profile_facts(user_id, namespace, fact_key)
  where superseded_by is null;
create index fields_document_order_idx on public.document_fields(document_id, sort_order);
create index fields_participant_idx on public.document_fields(participant_id)
  where participant_id is not null;
create index participants_document_status_idx on public.participants(document_id, status);
create index jobs_ready_idx on public.processing_jobs(status, run_after)
  where status = 'queued';
create index chunks_owner_document_idx on public.document_chunks(owner_id, document_id);
create index chunks_embedding_idx on public.document_chunks
  using hnsw (embedding vector_cosine_ops) where embedding is not null;
create index audit_document_created_idx on public.audit_events(document_id, created_at);

create function public.set_updated_at()
returns trigger language plpgsql set search_path = '' as $$
begin
  new.updated_at = now();
  return new;
end;
$$;

create trigger profiles_updated_at before update on public.profiles
for each row execute function public.set_updated_at();
create trigger documents_updated_at before update on public.documents
for each row execute function public.set_updated_at();
create trigger participants_updated_at before update on public.participants
for each row execute function public.set_updated_at();
create trigger profile_facts_updated_at before update on public.profile_facts
for each row execute function public.set_updated_at();
create trigger fields_updated_at before update on public.document_fields
for each row execute function public.set_updated_at();
create trigger jobs_updated_at before update on public.processing_jobs
for each row execute function public.set_updated_at();

