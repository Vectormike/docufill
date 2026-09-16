use axum::{Json, extract::State};
use uuid::Uuid;

use crate::{
    AppError, AppResult, AppState,
    auth::AuthUser,
    models::{CreateSignature, SignatureSummary},
    security::{CryptoService, normalize_signature_data_url},
};

pub async fn create_signature(
    State(state): State<AppState>,
    user: AuthUser,
    Json(input): Json<CreateSignature>,
) -> AppResult<Json<SignatureSummary>> {
    if !user.has_recent_authentication() {
        return Err(AppError::Unauthorized);
    }
    if !matches!(input.kind.as_str(), "drawn" | "typed" | "uploaded") {
        return Err(AppError::Validation(
            "Signature kind must be drawn, typed, or uploaded".to_owned(),
        ));
    }

    let normalized = normalize_signature_data_url(&input.image_data_url)?;
    let content_hash = CryptoService::hash(&normalized);
    let encrypted = state.crypto.encrypt(&normalized)?;
    let signature_id = Uuid::new_v4();
    let version = sqlx::query_scalar::<_, i32>(
        "select coalesce(max(version), 0) + 1 from public.signatures where user_id = $1",
    )
    .bind(user.id)
    .fetch_one(&state.pool)
    .await?;
    let storage_path = format!("{}/{}.bin", user.id, signature_id);

    state
        .storage
        .upload_encrypted(&state.config.signatures_bucket, &storage_path, encrypted)
        .await?;

    let mut transaction = state.pool.begin().await?;
    sqlx::query(
        "update public.signatures set revoked_at = now()
         where user_id = $1 and revoked_at is null",
    )
    .bind(user.id)
    .execute(&mut *transaction)
    .await?;
    let signature = sqlx::query_as::<_, SignatureSummary>(
        "insert into public.signatures (
            id, user_id, kind, storage_path, content_hash, version
         ) values ($1, $2, $3, $4, $5, $6)
         returning id, kind, version, created_at, revoked_at",
    )
    .bind(signature_id)
    .bind(user.id)
    .bind(input.kind)
    .bind(storage_path)
    .bind(content_hash)
    .bind(version)
    .fetch_one(&mut *transaction)
    .await?;
    sqlx::query(
        "insert into public.audit_events (
            owner_id, actor_type, actor_id, event_type, metadata
         ) values ($1, 'owner', $1::text, 'signature.saved', $2)",
    )
    .bind(user.id)
    .bind(serde_json::json!({ "signature_id": signature_id, "version": version }))
    .execute(&mut *transaction)
    .await?;
    transaction.commit().await?;

    Ok(Json(signature))
}

pub async fn revoke_signature(
    State(state): State<AppState>,
    user: AuthUser,
) -> AppResult<axum::http::StatusCode> {
    if !user.has_recent_authentication() {
        return Err(AppError::Unauthorized);
    }

    let paths = sqlx::query_scalar::<_, String>(
        "update public.signatures
         set revoked_at = coalesce(revoked_at, now())
         where user_id = $1
         returning storage_path",
    )
    .bind(user.id)
    .fetch_all(&state.pool)
    .await?;
    if paths.is_empty() {
        return Err(AppError::NotFound);
    }

    for path in &paths {
        state
            .storage
            .delete(&state.config.signatures_bucket, path)
            .await?;
    }
    sqlx::query("delete from public.signatures where user_id = $1")
        .bind(user.id)
        .execute(&state.pool)
        .await?;
    sqlx::query(
        "insert into public.audit_events (
            owner_id, actor_type, actor_id, event_type
         ) values ($1, 'owner', $1::text, 'signature.deleted')",
    )
    .bind(user.id)
    .execute(&state.pool)
    .await?;

    Ok(axum::http::StatusCode::NO_CONTENT)
}
