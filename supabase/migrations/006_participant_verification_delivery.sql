alter table public.participants
add column verification_sent_at timestamptz;

create index participants_verification_delivery_idx
on public.participants (id, verification_sent_at);

