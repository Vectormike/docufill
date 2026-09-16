create function public.handle_new_user()
returns trigger
language plpgsql
security definer
set search_path = ''
as $$
begin
  insert into public.profiles (id, display_name, avatar_url)
  values (
    new.id,
    coalesce(new.raw_user_meta_data ->> 'full_name', new.raw_user_meta_data ->> 'name'),
    new.raw_user_meta_data ->> 'avatar_url'
  )
  on conflict (id) do nothing;
  return new;
end;
$$;

create trigger on_auth_user_created
after insert on auth.users
for each row execute function public.handle_new_user();

alter table public.profiles enable row level security;
alter table public.documents enable row level security;
alter table public.participants enable row level security;
alter table public.profile_facts enable row level security;
alter table public.signatures enable row level security;
alter table public.document_contexts enable row level security;
alter table public.document_fields enable row level security;
alter table public.answer_proposals enable row level security;
alter table public.document_chunks enable row level security;
alter table public.processing_jobs enable row level security;
alter table public.participant_sessions enable row level security;
alter table public.audit_events enable row level security;

create policy "profiles_select_own" on public.profiles
for select using (id = auth.uid());
create policy "profiles_insert_own" on public.profiles
for insert with check (id = auth.uid());
create policy "profiles_update_own" on public.profiles
for update using (id = auth.uid()) with check (id = auth.uid());

create policy "documents_owner_all" on public.documents
for all using (owner_id = auth.uid()) with check (owner_id = auth.uid());

create policy "participants_owner_all" on public.participants
for all
using (
  exists (
    select 1 from public.documents d
    where d.id = participants.document_id and d.owner_id = auth.uid()
  )
)
with check (
  exists (
    select 1 from public.documents d
    where d.id = participants.document_id and d.owner_id = auth.uid()
  )
);

create policy "profile_facts_owner_all" on public.profile_facts
for all using (user_id = auth.uid()) with check (user_id = auth.uid());

create policy "signatures_owner_metadata" on public.signatures
for select using (user_id = auth.uid());

create policy "contexts_owner_all" on public.document_contexts
for all using (owner_id = auth.uid()) with check (owner_id = auth.uid());

create policy "fields_owner_all" on public.document_fields
for all
using (
  exists (
    select 1 from public.documents d
    where d.id = document_fields.document_id and d.owner_id = auth.uid()
  )
)
with check (
  exists (
    select 1 from public.documents d
    where d.id = document_fields.document_id and d.owner_id = auth.uid()
  )
);

create policy "proposals_owner_all" on public.answer_proposals
for all
using (
  exists (
    select 1
    from public.document_fields f
    join public.documents d on d.id = f.document_id
    where f.id = answer_proposals.field_id and d.owner_id = auth.uid()
  )
)
with check (
  exists (
    select 1
    from public.document_fields f
    join public.documents d on d.id = f.document_id
    where f.id = answer_proposals.field_id and d.owner_id = auth.uid()
  )
);

create policy "chunks_owner_all" on public.document_chunks
for all using (owner_id = auth.uid()) with check (owner_id = auth.uid());

create policy "jobs_owner_read" on public.processing_jobs
for select using (owner_id = auth.uid());

create policy "audit_owner_read" on public.audit_events
for select using (owner_id = auth.uid());

revoke insert, update, delete on public.audit_events from authenticated;
revoke all on public.participant_sessions from anon, authenticated;

insert into storage.buckets (id, name, public, file_size_limit, allowed_mime_types)
values (
  'documents',
  'documents',
  false,
  26214400,
  array['application/pdf']
)
on conflict (id) do update set
  public = excluded.public,
  file_size_limit = excluded.file_size_limit,
  allowed_mime_types = excluded.allowed_mime_types;

insert into storage.buckets (id, name, public, file_size_limit, allowed_mime_types)
values (
  'document-context',
  'document-context',
  false,
  10485760,
  array['application/pdf', 'text/plain', 'message/rfc822']
)
on conflict (id) do update set
  public = excluded.public,
  file_size_limit = excluded.file_size_limit,
  allowed_mime_types = excluded.allowed_mime_types;

insert into storage.buckets (id, name, public, file_size_limit, allowed_mime_types)
values (
  'signatures',
  'signatures',
  false,
  1048576,
  array['application/octet-stream']
)
on conflict (id) do update set
  public = excluded.public,
  file_size_limit = excluded.file_size_limit,
  allowed_mime_types = excluded.allowed_mime_types;

create policy "documents_insert_own_folder" on storage.objects
for insert to authenticated
with check (
  bucket_id = 'documents'
  and (storage.foldername(name))[1] = auth.uid()::text
);
create policy "documents_read_own_folder" on storage.objects
for select to authenticated
using (
  bucket_id = 'documents'
  and (storage.foldername(name))[1] = auth.uid()::text
);
create policy "documents_update_own_folder" on storage.objects
for update to authenticated
using (
  bucket_id = 'documents'
  and (storage.foldername(name))[1] = auth.uid()::text
)
with check (
  bucket_id = 'documents'
  and (storage.foldername(name))[1] = auth.uid()::text
);
create policy "documents_delete_own_folder" on storage.objects
for delete to authenticated
using (
  bucket_id = 'documents'
  and (storage.foldername(name))[1] = auth.uid()::text
);

create policy "contexts_insert_own_folder" on storage.objects
for insert to authenticated
with check (
  bucket_id = 'document-context'
  and (storage.foldername(name))[1] = auth.uid()::text
);
create policy "contexts_read_own_folder" on storage.objects
for select to authenticated
using (
  bucket_id = 'document-context'
  and (storage.foldername(name))[1] = auth.uid()::text
);
create policy "contexts_delete_own_folder" on storage.objects
for delete to authenticated
using (
  bucket_id = 'document-context'
  and (storage.foldername(name))[1] = auth.uid()::text
);

