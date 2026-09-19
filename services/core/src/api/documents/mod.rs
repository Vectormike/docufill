mod completion;

use std::collections::HashSet;

use axum::{
    Json,
    extract::{Path, State},
};
use chrono::{DateTime, Utc};
use sqlx::FromRow;
use uuid::Uuid;

use crate::{
    AppError, AppResult, AppState,
    auth::AuthUser,
    fields::requires_owner_answer,
    memory::is_sensitive_label,
    models::{
        AddTextContext, AssignField, CopilotSummary, CreateDocument, DocumentDetail, DocumentField,
        DocumentSummary, DownloadUrl, SetMemoryConsent, UpdateFieldAnswer, UpdateFieldLayout,
    },
};

pub use completion::complete_document;

#[derive(FromRow)]
struct EncryptedField {
    id: Uuid,
    participant_id: Option<Uuid>,
    field_key: String,
    label: String,
    instructions: Option<String>,
    kind: String,
    page_number: i32,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    font_size: Option<f64>,
    alignment: String,
    value_ciphertext: Option<Vec<u8>>,
    value_preview: Option<String>,
    source: String,
    confidence: Option<f64>,
    source_explanation: Option<String>,
    source_reference_count: i32,
    confirmed_at: Option<DateTime<Utc>>,
    sort_order: i32,
}

#[derive(FromRow)]
struct MemoryCandidate {
    label: String,
    kind: String,
    value_ciphertext: Vec<u8>,
    value_preview: Option<String>,
}

pub async fn list_documents(
    State(state): State<AppState>,
    user: AuthUser,
) -> AppResult<Json<Vec<DocumentSummary>>> {
    let documents = sqlx::query_as::<_, DocumentSummary>(
        "select id, subject, original_name, status::text as status, page_count,
                progress, memory_consent, completed_at, created_at, updated_at
         from public.documents
         where owner_id = $1
         order by created_at desc
         limit 100",
    )
    .bind(user.id)
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(documents))
}

pub async fn create_document(
    State(state): State<AppState>,
    user: AuthUser,
    Json(input): Json<CreateDocument>,
) -> AppResult<Json<DocumentSummary>> {
    validate_document_input(user.id, &input)?;
    let mut transaction = state.pool.begin().await?;
    sqlx::query(
        "insert into public.profiles (id)
         values ($1)
         on conflict (id) do nothing",
    )
    .bind(user.id)
    .execute(&mut *transaction)
    .await?;
    let document = sqlx::query_as::<_, DocumentSummary>(
        "insert into public.documents (
            owner_id, subject, original_name, original_storage_path, content_hash
         ) values ($1, $2, $3, $4, $5)
         returning id, subject, original_name, status::text as status, page_count,
                   progress, memory_consent, completed_at, created_at, updated_at",
    )
    .bind(user.id)
    .bind(input.subject.trim())
    .bind(input.original_name.trim())
    .bind(input.original_storage_path)
    .bind(input.content_hash.to_ascii_lowercase())
    .fetch_one(&mut *transaction)
    .await?;
    sqlx::query(
        "insert into public.processing_jobs (document_id, owner_id, kind)
         values ($1, $2, 'extract')",
    )
    .bind(document.id)
    .bind(user.id)
    .execute(&mut *transaction)
    .await?;
    audit(
        &mut transaction,
        user.id,
        document.id,
        "document.uploaded",
        serde_json::json!({ "original_name": document.original_name }),
    )
    .await?;
    transaction.commit().await?;
    Ok(Json(document))
}

pub async fn get_document(
    State(state): State<AppState>,
    user: AuthUser,
    Path(document_id): Path<Uuid>,
) -> AppResult<Json<DocumentDetail>> {
    let document = owned_document(&state, user.id, document_id).await?;
    let fields = fields_for_document(&state, user.id, document_id).await?;
    let copilot = CopilotSummary::from_fields(&fields);
    Ok(Json(DocumentDetail {
        document,
        fields,
        copilot,
    }))
}

pub async fn delete_document(
    State(state): State<AppState>,
    user: AuthUser,
    Path(document_id): Path<Uuid>,
) -> AppResult<axum::http::StatusCode> {
    let (original_path, content_hash, status) = sqlx::query_as::<_, (String, String, String)>(
        "select original_storage_path, content_hash, status::text
         from public.documents where id = $1 and owner_id = $2",
    )
    .bind(document_id)
    .bind(user.id)
    .fetch_one(&state.pool)
    .await?;
    if status == "processing" {
        return Err(AppError::Conflict);
    }

    let mut document_paths = sqlx::query_scalar::<_, String>(
        "select storage_path from public.document_artifacts
         where document_id = $1 and owner_id = $2",
    )
    .bind(document_id)
    .bind(user.id)
    .fetch_all(&state.pool)
    .await?
    .into_iter()
    .collect::<HashSet<_>>();
    document_paths.insert(original_path);
    for path in document_paths {
        state
            .storage
            .delete(&state.config.documents_bucket, &path)
            .await?;
    }
    let context_paths = sqlx::query_scalar::<_, String>(
        "select storage_path from public.document_contexts
         where document_id = $1 and owner_id = $2 and storage_path is not null",
    )
    .bind(document_id)
    .bind(user.id)
    .fetch_all(&state.pool)
    .await?;
    for path in context_paths {
        state
            .storage
            .delete(&state.config.context_bucket, &path)
            .await?;
    }

    let mut transaction = state.pool.begin().await?;
    sqlx::query(
        "delete from public.document_extractions
         where owner_id = $1 and content_hash = $2",
    )
    .bind(user.id)
    .bind(content_hash)
    .execute(&mut *transaction)
    .await?;
    sqlx::query("delete from public.documents where id = $1 and owner_id = $2")
        .bind(document_id)
        .bind(user.id)
        .execute(&mut *transaction)
        .await?;
    sqlx::query(
        "insert into public.audit_events (
            owner_id, actor_type, actor_id, event_type, metadata
         ) values (
            $1, 'owner', $1::text, 'document.deleted',
            jsonb_build_object('deleted_document_id', $2::text)
         )",
    )
    .bind(user.id)
    .bind(document_id)
    .execute(&mut *transaction)
    .await?;
    transaction.commit().await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

pub async fn update_field(
    State(state): State<AppState>,
    user: AuthUser,
    Path((document_id, field_id)): Path<(Uuid, Uuid)>,
    Json(input): Json<UpdateFieldAnswer>,
) -> AppResult<Json<DocumentField>> {
    let value = input.value.trim();
    if value.is_empty() || value.len() > 10_000 {
        return Err(AppError::Validation("Answer is invalid".to_owned()));
    }
    if matches!(input.source, crate::models::AnswerTrust::AiDraft) && input.confirmed {
        return Err(AppError::Validation(
            "Accept an AI draft as a user-confirmed answer before finalization".to_owned(),
        ));
    }

    let encrypted = state.crypto.encrypt(value.as_bytes())?;
    let mut transaction = state.pool.begin().await?;
    let field = sqlx::query_as::<_, DocumentField>(
        "update public.document_fields f
         set value_ciphertext = $4,
             value_preview = $5,
             source = $6::public.answer_source,
             confirmed_at = case when $7 then now() else null end
         from public.documents d
         where f.id = $1 and f.document_id = $2
           and d.id = f.document_id and d.owner_id = $3
           and d.status in ('needs_input', 'ready', 'failed')
           and f.participant_id is null
         returning f.id, f.participant_id, f.field_key, f.label, f.instructions,
                   f.kind::text as kind, f.page_number, f.x::float8 as x,
                   f.y::float8 as y, f.width::float8 as width,
                   f.height::float8 as height, f.font_size::float8 as font_size,
                   f.alignment, $5::text as value, f.value_preview,
                   f.source::text as source, f.confidence::float8 as confidence,
                   null::text as source_explanation, 0::int as source_reference_count,
                   f.confirmed_at, f.sort_order",
    )
    .bind(field_id)
    .bind(document_id)
    .bind(user.id)
    .bind(encrypted)
    .bind(value.chars().take(120).collect::<String>())
    .bind(input.source.as_str())
    .bind(input.confirmed)
    .fetch_one(&mut *transaction)
    .await?;
    let field = with_required(field);
    if input.confirmed {
        sqlx::query(
            "update public.answer_proposals
             set accepted_at = now()
             where field_id = $1 and accepted_at is null and rejected_at is null",
        )
        .bind(field_id)
        .execute(&mut *transaction)
        .await?;
    }
    refresh_owner_input_status(&mut transaction, document_id, Some(user.id)).await?;
    audit(
        &mut transaction,
        user.id,
        document_id,
        "document.answer_confirmed",
        serde_json::json!({ "field_id": field_id, "source": input.source.as_str() }),
    )
    .await?;
    transaction.commit().await?;
    Ok(Json(field))
}

pub async fn clear_field(
    State(state): State<AppState>,
    user: AuthUser,
    Path((document_id, field_id)): Path<(Uuid, Uuid)>,
) -> AppResult<axum::http::StatusCode> {
    let mut transaction = state.pool.begin().await?;
    let cleared = sqlx::query(
        "update public.document_fields f
         set value_ciphertext = null, value_preview = null, source = 'missing',
             source_fact_id = null, confidence = null, confirmed_at = null
         from public.documents d
         where f.id = $1 and f.document_id = $2
           and d.id = f.document_id and d.owner_id = $3
           and d.status in ('needs_input', 'ready', 'failed')
           and f.participant_id is null",
    )
    .bind(field_id)
    .bind(document_id)
    .bind(user.id)
    .execute(&mut *transaction)
    .await?;
    refresh_owner_input_status(&mut transaction, document_id, Some(user.id)).await?;
    if cleared.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    sqlx::query(
        "update public.answer_proposals set rejected_at = now()
         where field_id = $1 and accepted_at is null",
    )
    .bind(field_id)
    .execute(&mut *transaction)
    .await?;
    transaction.commit().await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

pub async fn assign_field(
    State(state): State<AppState>,
    user: AuthUser,
    Path((document_id, field_id)): Path<(Uuid, Uuid)>,
    Json(input): Json<AssignField>,
) -> AppResult<axum::http::StatusCode> {
    if let Some(participant_id) = input.participant_id {
        let belongs = sqlx::query_scalar::<_, bool>(
            "select exists(
                select 1 from public.participants p
                join public.documents d on d.id = p.document_id
                where p.id = $1 and p.document_id = $2 and d.owner_id = $3
                  and p.status not in ('revoked', 'completed')
             )",
        )
        .bind(participant_id)
        .bind(document_id)
        .bind(user.id)
        .fetch_one(&state.pool)
        .await?;
        if !belongs {
            return Err(AppError::Validation(
                "Participant does not belong to this document".to_owned(),
            ));
        }
    }

    let mut transaction = state.pool.begin().await?;
    let updated = sqlx::query(
        "update public.document_fields f
         set participant_id = $4, value_ciphertext = null,
             value_preview = null, source = 'missing',
             source_fact_id = null, confidence = null, confirmed_at = null
         from public.documents d
         where f.id = $1 and f.document_id = $2
           and d.id = f.document_id and d.owner_id = $3
           and d.status in ('needs_input', 'ready', 'failed')",
    )
    .bind(field_id)
    .bind(document_id)
    .bind(user.id)
    .bind(input.participant_id)
    .execute(&mut *transaction)
    .await?;
    if updated.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    refresh_owner_input_status(&mut transaction, document_id, Some(user.id)).await?;
    audit(
        &mut transaction,
        user.id,
        document_id,
        "document.field_assigned",
        serde_json::json!({
            "field_id": field_id,
            "participant_id": input.participant_id
        }),
    )
    .await?;
    transaction.commit().await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

pub async fn update_field_layout(
    State(state): State<AppState>,
    user: AuthUser,
    Path((document_id, field_id)): Path<(Uuid, Uuid)>,
    Json(input): Json<UpdateFieldLayout>,
) -> AppResult<axum::http::StatusCode> {
    if input.x < 0.0
        || input.y < 0.0
        || !(10.0..=2_000.0).contains(&input.width)
        || !(8.0..=2_000.0).contains(&input.height)
        || input
            .font_size
            .is_some_and(|size| !(6.0..=24.0).contains(&size))
        || !matches!(input.alignment.as_str(), "left" | "center" | "right")
    {
        return Err(AppError::Validation(
            "Field layout values are invalid".to_owned(),
        ));
    }
    let mut transaction = state.pool.begin().await?;
    let updated = sqlx::query(
        "update public.document_fields f
         set x = $4, y = $5, width = $6, height = $7,
             font_size = $8, alignment = $9
         from public.documents d
         where f.id = $1 and f.document_id = $2
           and d.id = f.document_id and d.owner_id = $3
           and d.status in ('needs_input', 'ready', 'failed')",
    )
    .bind(field_id)
    .bind(document_id)
    .bind(user.id)
    .bind(input.x)
    .bind(input.y)
    .bind(input.width)
    .bind(input.height)
    .bind(input.font_size)
    .bind(input.alignment)
    .execute(&mut *transaction)
    .await?;
    if updated.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    sqlx::query(
        "update public.documents set preview_storage_path = null
         where id = $1 and owner_id = $2",
    )
    .bind(document_id)
    .bind(user.id)
    .execute(&mut *transaction)
    .await?;
    transaction.commit().await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

pub async fn add_text_context(
    State(state): State<AppState>,
    user: AuthUser,
    Path(document_id): Path<Uuid>,
    Json(input): Json<AddTextContext>,
) -> AppResult<axum::http::StatusCode> {
    if input.content.trim().is_empty() || input.content.len() > 50_000 {
        return Err(AppError::Validation(
            "Context must contain 1 to 50,000 characters".to_owned(),
        ));
    }
    ensure_owns_document(&state, user.id, document_id).await?;
    let encrypted = state.crypto.encrypt(input.content.trim().as_bytes())?;
    sqlx::query(
        "insert into public.document_contexts (
            document_id, owner_id, kind, content_ciphertext
         ) values ($1, $2, 'text', $3)",
    )
    .bind(document_id)
    .bind(user.id)
    .bind(encrypted)
    .execute(&state.pool)
    .await?;
    Ok(axum::http::StatusCode::CREATED)
}

pub async fn set_memory_consent(
    State(state): State<AppState>,
    user: AuthUser,
    Path(document_id): Path<Uuid>,
    Json(input): Json<SetMemoryConsent>,
) -> AppResult<axum::http::StatusCode> {
    if input.enabled && input.approved_field_ids.is_empty() {
        return Err(AppError::Validation(
            "Choose at least one confirmed answer to remember".to_owned(),
        ));
    }
    let mut transaction = state.pool.begin().await?;
    let updated = sqlx::query(
        "update public.documents set memory_consent = $3
         where id = $1 and owner_id = $2 and status = 'completed'",
    )
    .bind(document_id)
    .bind(user.id)
    .bind(input.enabled)
    .execute(&mut *transaction)
    .await?;
    if updated.rows_affected() == 0 {
        return Err(AppError::Conflict);
    }

    if input.enabled {
        let candidates = sqlx::query_as::<_, MemoryCandidate>(
            "select label, kind::text as kind, value_ciphertext, value_preview
             from public.document_fields
             where document_id = $1
               and id = any($2)
               and confirmed_at is not null
               and participant_id is null
               and value_ciphertext is not null
               and kind not in ('signature', 'declaration')",
        )
        .bind(document_id)
        .bind(&input.approved_field_ids)
        .fetch_all(&mut *transaction)
        .await?;
        if candidates.len() != input.approved_field_ids.len()
            || candidates
                .iter()
                .any(|candidate| is_sensitive_label(&candidate.label))
        {
            return Err(AppError::Validation(
                "Memory can include only confirmed, non-sensitive owner answers".to_owned(),
            ));
        }
        for candidate in &candidates {
            promote_memory_candidate(&mut transaction, user.id, document_id, candidate).await?;
        }
        sqlx::query("delete from public.document_chunks where document_id = $1 and owner_id = $2")
            .bind(document_id)
            .bind(user.id)
            .execute(&mut *transaction)
            .await?;
        sqlx::query(
            "insert into public.processing_jobs (document_id, owner_id, kind, payload)
             values (
                $1, $2, 'embed',
                jsonb_build_object('approved_field_ids', $3::uuid[])
             )",
        )
        .bind(document_id)
        .bind(user.id)
        .bind(&input.approved_field_ids)
        .execute(&mut *transaction)
        .await?;
    } else {
        sqlx::query(
            "delete from public.processing_jobs
             where document_id = $1 and owner_id = $2
               and kind = 'embed' and status = 'queued'",
        )
        .bind(document_id)
        .bind(user.id)
        .execute(&mut *transaction)
        .await?;
        sqlx::query("delete from public.document_chunks where document_id = $1 and owner_id = $2")
            .bind(document_id)
            .bind(user.id)
            .execute(&mut *transaction)
            .await?;
    }
    audit(
        &mut transaction,
        user.id,
        document_id,
        "document.memory_consent_changed",
        serde_json::json!({
            "enabled": input.enabled,
            "approved_field_count": input.approved_field_ids.len(),
            "promoted_fact_count": if input.enabled {
                input.approved_field_ids.len()
            } else {
                0
            },
        }),
    )
    .await?;
    transaction.commit().await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

pub async fn request_preview(
    State(state): State<AppState>,
    user: AuthUser,
    Path(document_id): Path<Uuid>,
) -> AppResult<Json<crate::models::CompletionQueued>> {
    ensure_owns_document(&state, user.id, document_id).await?;
    let unconfirmed = count_blocking_owner_fields(&state.pool, document_id).await?;
    if unconfirmed > 0 {
        return Err(AppError::Validation(format!(
            "{unconfirmed} answers still need confirmation"
        )));
    }

    let mut transaction = state.pool.begin().await?;
    let updated = sqlx::query(
        "update public.documents
         set status = 'processing', progress = 85, preview_storage_path = null
         where id = $1 and owner_id = $2
           and status in ('needs_input', 'ready', 'failed')",
    )
    .bind(document_id)
    .bind(user.id)
    .execute(&mut *transaction)
    .await?;
    if updated.rows_affected() == 0 {
        return Err(AppError::Conflict);
    }
    sqlx::query(
        "insert into public.processing_jobs (document_id, owner_id, kind, payload)
         values ($1, $2, 'render', '{\"preview\": true}'::jsonb)",
    )
    .bind(document_id)
    .bind(user.id)
    .execute(&mut *transaction)
    .await?;
    transaction.commit().await?;
    Ok(Json(crate::models::CompletionQueued {
        document_id,
        status: "processing".to_owned(),
    }))
}

pub async fn preview_document(
    State(state): State<AppState>,
    user: AuthUser,
    Path(document_id): Path<Uuid>,
) -> AppResult<Json<DownloadUrl>> {
    let path = sqlx::query_scalar::<_, Option<String>>(
        "select preview_storage_path from public.documents
         where id = $1 and owner_id = $2",
    )
    .bind(document_id)
    .bind(user.id)
    .fetch_one(&state.pool)
    .await?
    .ok_or(AppError::NotFound)?;
    let expires_in = 300;
    let url = state
        .storage
        .signed_download_url(&state.config.documents_bucket, &path, expires_in)
        .await?;
    Ok(Json(DownloadUrl { url, expires_in }))
}

pub async fn download_document(
    State(state): State<AppState>,
    user: AuthUser,
    Path(document_id): Path<Uuid>,
) -> AppResult<Json<DownloadUrl>> {
    let path = sqlx::query_scalar::<_, Option<String>>(
        "select completed_storage_path from public.documents
         where id = $1 and owner_id = $2 and status = 'completed'",
    )
    .bind(document_id)
    .bind(user.id)
    .fetch_one(&state.pool)
    .await?
    .ok_or(AppError::NotFound)?;
    let expires_in = 300;
    let url = state
        .storage
        .signed_download_url(&state.config.documents_bucket, &path, expires_in)
        .await?;
    Ok(Json(DownloadUrl { url, expires_in }))
}

async fn owned_document(
    state: &AppState,
    owner_id: Uuid,
    document_id: Uuid,
) -> AppResult<DocumentSummary> {
    sqlx::query_as::<_, DocumentSummary>(
        "select id, subject, original_name, status::text as status, page_count,
                progress, memory_consent, completed_at, created_at, updated_at
         from public.documents where id = $1 and owner_id = $2",
    )
    .bind(document_id)
    .bind(owner_id)
    .fetch_one(&state.pool)
    .await
    .map_err(Into::into)
}

#[derive(FromRow)]
struct FieldGate {
    label: String,
    kind: String,
    participant_id: Option<Uuid>,
    has_value: bool,
    confirmed: bool,
}

async fn field_gates(
    executor: impl sqlx::Executor<'_, Database = sqlx::Postgres>,
    document_id: Uuid,
) -> AppResult<Vec<FieldGate>> {
    sqlx::query_as::<_, FieldGate>(
        "select label, kind::text as kind, participant_id,
                value_ciphertext is not null as has_value,
                confirmed_at is not null as confirmed
         from public.document_fields
         where document_id = $1",
    )
    .bind(document_id)
    .fetch_all(executor)
    .await
    .map_err(Into::into)
}

pub(crate) fn with_required(mut field: DocumentField) -> DocumentField {
    field.required = requires_owner_answer(&field.label, &field.kind);
    field
}

fn blocking_owner_count(fields: &[FieldGate]) -> i64 {
    fields
        .iter()
        .filter(|field| {
            field.participant_id.is_none()
                && requires_owner_answer(&field.label, &field.kind)
                && (!field.has_value || !field.confirmed)
        })
        .count() as i64
}

async fn count_blocking_owner_fields(pool: &sqlx::PgPool, document_id: Uuid) -> AppResult<i64> {
    Ok(blocking_owner_count(&field_gates(pool, document_id).await?))
}

pub(crate) async fn refresh_owner_input_status(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    document_id: Uuid,
    owner_id: Option<Uuid>,
) -> AppResult<()> {
    let blocking = blocking_owner_count(&field_gates(&mut **transaction, document_id).await?);
    let status = if blocking > 0 { "needs_input" } else { "ready" };
    if let Some(owner_id) = owner_id {
        sqlx::query(
            "update public.documents
             set preview_storage_path = null,
                 status = $3::public.document_status
             where id = $1 and owner_id = $2",
        )
        .bind(document_id)
        .bind(owner_id)
        .bind(status)
        .execute(&mut **transaction)
        .await?;
    } else {
        sqlx::query(
            "update public.documents
             set preview_storage_path = null,
                 status = $2::public.document_status
             where id = $1",
        )
        .bind(document_id)
        .bind(status)
        .execute(&mut **transaction)
        .await?;
    }
    Ok(())
}

async fn fields_for_document(
    state: &AppState,
    owner_id: Uuid,
    document_id: Uuid,
) -> AppResult<Vec<DocumentField>> {
    let rows = sqlx::query_as::<_, EncryptedField>(
        "select f.id, f.participant_id, f.field_key, f.label, f.instructions,
                f.kind::text as kind, f.page_number, f.x::float8 as x,
                f.y::float8 as y, f.width::float8 as width,
                f.height::float8 as height, f.font_size::float8 as font_size,
                f.alignment, f.value_ciphertext, f.value_preview,
                f.source::text as source, f.confidence::float8 as confidence,
                proposal.explanation as source_explanation,
                coalesce(proposal.source_reference_count, 0) as source_reference_count,
                f.confirmed_at, f.sort_order
         from public.document_fields f
         join public.documents d on d.id = f.document_id
         left join lateral (
            select explanation, cardinality(source_fact_ids) as source_reference_count
            from public.answer_proposals
            where field_id = f.id
            order by created_at desc
            limit 1
         ) proposal on true
         where f.document_id = $1 and d.owner_id = $2
         order by f.sort_order, f.created_at",
    )
    .bind(document_id)
    .bind(owner_id)
    .fetch_all(&state.pool)
    .await?;
    rows.into_iter()
        .map(|row| {
            let value = if row.kind == "signature" {
                None
            } else {
                row.value_ciphertext
                    .as_ref()
                    .map(|encrypted| state.crypto.decrypt(encrypted))
                    .transpose()?
                    .map(String::from_utf8)
                    .transpose()
                    .map_err(AppError::internal)?
            };
            Ok(with_required(DocumentField {
                id: row.id,
                participant_id: row.participant_id,
                field_key: row.field_key,
                label: row.label,
                instructions: row.instructions,
                kind: row.kind,
                page_number: row.page_number,
                x: row.x,
                y: row.y,
                width: row.width,
                height: row.height,
                font_size: row.font_size,
                alignment: row.alignment,
                value,
                value_preview: row.value_preview,
                source: row.source,
                confidence: row.confidence,
                source_explanation: row.source_explanation,
                source_reference_count: row.source_reference_count,
                confirmed_at: row.confirmed_at,
                sort_order: row.sort_order,
                required: false,
            }))
        })
        .collect()
}

async fn ensure_owns_document(
    state: &AppState,
    owner_id: Uuid,
    document_id: Uuid,
) -> AppResult<()> {
    let owns = sqlx::query_scalar::<_, bool>(
        "select exists(
            select 1 from public.documents where id = $1 and owner_id = $2
         )",
    )
    .bind(document_id)
    .bind(owner_id)
    .fetch_one(&state.pool)
    .await?;
    if !owns {
        return Err(AppError::NotFound);
    }
    Ok(())
}

async fn promote_memory_candidate(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    owner_id: Uuid,
    document_id: Uuid,
    candidate: &MemoryCandidate,
) -> AppResult<()> {
    let fact_key = normalized_fact_key(&candidate.label);
    let namespace = sqlx::query_scalar::<_, String>(
        "select namespace from public.profile_facts
         where user_id = $1 and fact_key = $2 and superseded_by is null
         order by confirmed_at desc
         limit 1",
    )
    .bind(owner_id)
    .bind(&fact_key)
    .fetch_optional(&mut **transaction)
    .await?
    .unwrap_or_else(|| "custom".to_owned());
    let fact_id = Uuid::new_v4();
    sqlx::query(
        "update public.profile_facts set superseded_by = $4
         where user_id = $1 and namespace = $2 and fact_key = $3
           and superseded_by is null",
    )
    .bind(owner_id)
    .bind(&namespace)
    .bind(&fact_key)
    .bind(fact_id)
    .execute(&mut **transaction)
    .await?;
    sqlx::query(
        "insert into public.profile_facts (
            id, user_id, namespace, fact_key, value_ciphertext, value_preview,
            value_type, sensitivity, source_type, source_document_id
         ) values ($1, $2, $3, $4, $5, $6, $7, 'personal', 'user', $8)",
    )
    .bind(fact_id)
    .bind(owner_id)
    .bind(namespace)
    .bind(fact_key)
    .bind(&candidate.value_ciphertext)
    .bind(&candidate.value_preview)
    .bind(&candidate.kind)
    .bind(document_id)
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

fn normalized_fact_key(label: &str) -> String {
    label
        .chars()
        .map(|character| {
            if character.is_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect::<String>()
        .split('_')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("_")
        .chars()
        .take(100)
        .collect()
}

fn validate_document_input(user_id: Uuid, input: &CreateDocument) -> AppResult<()> {
    if input.subject.trim().is_empty() || input.subject.len() > 200 {
        return Err(AppError::Validation(
            "Document subject is invalid".to_owned(),
        ));
    }
    if !input.original_name.to_ascii_lowercase().ends_with(".pdf") {
        return Err(AppError::Validation("Document must be a PDF".to_owned()));
    }
    if !input
        .original_storage_path
        .starts_with(&format!("{user_id}/"))
    {
        return Err(AppError::Forbidden);
    }
    if input.content_hash.len() != 64
        || !input
            .content_hash
            .chars()
            .all(|character| character.is_ascii_hexdigit())
    {
        return Err(AppError::Validation("Document hash is invalid".to_owned()));
    }
    Ok(())
}

async fn audit(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    owner_id: Uuid,
    document_id: Uuid,
    event_type: &str,
    metadata: serde_json::Value,
) -> AppResult<()> {
    sqlx::query(
        "insert into public.audit_events (
            owner_id, document_id, actor_type, actor_id, event_type, metadata
         ) values ($1, $2, 'owner', $1::text, $3, $4)",
    )
    .bind(owner_id)
    .bind(document_id)
    .bind(event_type)
    .bind(metadata)
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::normalized_fact_key;

    #[test]
    fn normalizes_confirmed_document_labels_for_profile_reuse() {
        assert_eq!(
            normalized_fact_key("Current residential address"),
            "current_residential_address"
        );
    }
}
