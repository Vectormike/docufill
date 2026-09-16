const POLICIES: &str = include_str!("../../../supabase/migrations/002_security_policies.sql");
const PRIVATE_FUNCTIONS: &str =
    include_str!("../../../supabase/migrations/003_private_functions.sql");
const PIPELINE_RESILIENCE: &str =
    include_str!("../../../supabase/migrations/004_pipeline_resilience.sql");
const DOCUMENT_LIFECYCLE: &str =
    include_str!("../../../supabase/migrations/005_document_lifecycle.sql");

#[test]
fn sensitive_tables_have_row_level_security() {
    for table in [
        "profiles",
        "documents",
        "participants",
        "profile_facts",
        "signatures",
        "document_contexts",
        "document_fields",
        "answer_proposals",
        "document_chunks",
        "processing_jobs",
        "participant_sessions",
        "audit_events",
    ] {
        assert!(
            POLICIES.contains(&format!(
                "alter table public.{table} enable row level security"
            )),
            "missing RLS for {table}"
        );
    }
}

#[test]
fn storage_is_private_and_audit_events_are_immutable() {
    assert_eq!(POLICIES.matches("\n  false,").count(), 3);
    assert!(POLICIES.contains("revoke all on public.participant_sessions"));
    assert!(PRIVATE_FUNCTIONS.contains("trigger audit_events_immutable"));
    assert!(PRIVATE_FUNCTIONS.contains("raise exception 'audit events are append-only'"));
}

#[test]
fn worker_only_tables_remain_private() {
    for (migration, table) in [
        (PIPELINE_RESILIENCE, "document_extractions"),
        (DOCUMENT_LIFECYCLE, "document_artifacts"),
    ] {
        assert!(migration.contains(&format!(
            "alter table public.{table} enable row level security"
        )));
        assert!(migration.contains(&format!(
            "revoke all on public.{table} from anon, authenticated"
        )));
    }
}
