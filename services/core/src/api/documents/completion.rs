use axum::{
    Json,
    extract::{Path, State},
};
use serde_json::json;
use uuid::Uuid;

use crate::{
    AppError, AppResult, AppState,
    auth::AuthUser,
    models::{CompleteDocument, CompletionQueued},
    security::CryptoService,
};

pub async fn complete_document(
    State(state): State<AppState>,
    user: AuthUser,
    Path(document_id): Path<Uuid>,
    Json(input): Json<CompleteDocument>,
) -> AppResult<Json<CompletionQueued>> {
    if !user.has_recent_authentication() {
        return Err(AppError::Unauthorized);
    }
    if input.consent_text.trim().chars().count() < 10 {
        return Err(AppError::Validation(
            "Explicit completion and signing consent is required".to_owned(),
        ));
    }

    let document_exists = sqlx::query_scalar::<_, bool>(
        "select exists(
            select 1 from public.documents
            where id = $1 and owner_id = $2
              and status in ('ready', 'failed')
              and preview_storage_path is not null
         )",
    )
    .bind(document_id)
    .bind(user.id)
    .fetch_one(&state.pool)
    .await?;
    if !document_exists {
        return Err(AppError::NotFound);
    }

    let missing = sqlx::query_scalar::<_, i64>(
        "select count(*)
         from public.document_fields
         where document_id = $1
           and kind not in ('signature', 'declaration')
           and (value_ciphertext is null or confirmed_at is null)",
    )
    .bind(document_id)
    .fetch_one(&state.pool)
    .await?;
    if missing > 0 {
        return Err(AppError::Validation(format!(
            "{missing} required answers are still missing"
        )));
    }

    let incomplete_participants = sqlx::query_scalar::<_, i64>(
        "select count(*) from public.participants
         where document_id = $1 and status not in ('completed', 'revoked')",
    )
    .bind(document_id)
    .fetch_one(&state.pool)
    .await?;
    if incomplete_participants > 0 {
        return Err(AppError::Validation(
            "All invited participants must finish before completion".to_owned(),
        ));
    }

    let signature_required = sqlx::query_scalar::<_, bool>(
        "select exists(
            select 1 from public.document_fields
            where document_id = $1 and kind = 'signature' and participant_id is null
         )",
    )
    .bind(document_id)
    .fetch_one(&state.pool)
    .await?;
    if signature_required && input.apply_signature_id.is_none() {
        return Err(AppError::Validation(
            "Choose a saved signature before completing this document".to_owned(),
        ));
    }
    let signature_version = if let Some(signature_id) = input.apply_signature_id {
        let version = sqlx::query_scalar::<_, i32>(
            "select version from public.signatures
             where id = $1 and user_id = $2 and revoked_at is null",
        )
        .bind(signature_id)
        .bind(user.id)
        .fetch_optional(&state.pool)
        .await?;
        Some(version.ok_or(AppError::Forbidden)?)
    } else {
        None
    };

    let consent_hash = CryptoService::hash(input.consent_text.trim().as_bytes());
    let payload = json!({
        "signature_id": input.apply_signature_id,
        "signature_version": signature_version,
        "consent_hash": consent_hash,
        "authentication_method": user.authentication_method,
        "authenticated_at": user.authenticated_at,
    });
    let mut transaction = state.pool.begin().await?;
    sqlx::query(
        "update public.documents
         set status = 'processing', progress = 90, error_code = null
         where id = $1 and owner_id = $2",
    )
    .bind(document_id)
    .bind(user.id)
    .execute(&mut *transaction)
    .await?;
    sqlx::query(
        "insert into public.processing_jobs (document_id, owner_id, kind, payload)
         values ($1, $2, 'render', $3)",
    )
    .bind(document_id)
    .bind(user.id)
    .bind(payload.clone())
    .execute(&mut *transaction)
    .await?;
    sqlx::query(
        "insert into public.audit_events (
            owner_id, document_id, actor_type, actor_id, event_type, metadata
         ) values (
            $1, $2, 'owner', $1::text, 'document.completion_requested', $3
         )",
    )
    .bind(user.id)
    .bind(document_id)
    .bind(payload)
    .execute(&mut *transaction)
    .await?;
    transaction.commit().await?;

    Ok(Json(CompletionQueued {
        document_id,
        status: "processing".to_owned(),
    }))
}
