use std::collections::HashSet;

use axum::{Json, extract::State, http::StatusCode};
use serde_json::{Value, json};
use sqlx::FromRow;

use crate::{AppError, AppResult, AppState, auth::AuthUser, models::DeleteAccount};

#[derive(FromRow)]
struct ExportFact {
    namespace: String,
    fact_key: String,
    value_ciphertext: Vec<u8>,
    value_type: String,
    sensitivity: String,
    source_type: String,
    confirmed_at: chrono::DateTime<chrono::Utc>,
}

pub async fn export_account(
    State(state): State<AppState>,
    user: AuthUser,
) -> AppResult<Json<Value>> {
    let profile = sqlx::query_scalar::<_, Value>(
        "select jsonb_build_object(
            'id', id, 'display_name', display_name, 'avatar_url', avatar_url,
            'onboarding_completed', onboarding_completed,
            'created_at', created_at, 'updated_at', updated_at
         ) from public.profiles where id = $1",
    )
    .bind(user.id)
    .fetch_one(&state.pool)
    .await?;
    let facts = sqlx::query_as::<_, ExportFact>(
        "select namespace, fact_key, value_ciphertext, value_type,
                sensitivity::text as sensitivity, source_type,
                confirmed_at
         from public.profile_facts
         where user_id = $1 and superseded_by is null
         order by namespace, fact_key",
    )
    .bind(user.id)
    .fetch_all(&state.pool)
    .await?
    .into_iter()
    .map(|fact| {
        let value = state
            .crypto
            .decrypt(&fact.value_ciphertext)
            .and_then(|bytes| String::from_utf8(bytes).map_err(AppError::internal))?;
        Ok(json!({
            "namespace": fact.namespace,
            "key": fact.fact_key,
            "value": value,
            "value_type": fact.value_type,
            "sensitivity": fact.sensitivity,
            "source": fact.source_type,
            "confirmed_at": fact.confirmed_at
        }))
    })
    .collect::<AppResult<Vec<_>>>()?;
    let documents = aggregate_json(
        &state,
        "select coalesce(jsonb_agg(jsonb_build_object(
            'id', id, 'subject', subject, 'original_name', original_name,
            'status', status, 'page_count', page_count,
            'memory_consent', memory_consent, 'created_at', created_at,
            'completed_at', completed_at
         ) order by created_at desc), '[]'::jsonb)
         from public.documents where owner_id = $1",
        user.id,
    )
    .await?;
    let participants = aggregate_json(
        &state,
        "select coalesce(jsonb_agg(jsonb_build_object(
            'id', p.id, 'document_id', p.document_id,
            'display_name', p.display_name, 'role', p.role,
            'contact_type', p.contact_type, 'status', p.status,
            'created_at', p.created_at, 'completed_at', p.completed_at
         ) order by p.created_at desc), '[]'::jsonb)
         from public.participants p
         join public.documents d on d.id = p.document_id
         where d.owner_id = $1",
        user.id,
    )
    .await?;
    let audit_events = aggregate_json(
        &state,
        "select coalesce(jsonb_agg(jsonb_build_object(
            'event_type', event_type, 'document_id', document_id,
            'actor_type', actor_type, 'created_at', created_at,
            'metadata', metadata, 'document_hash', document_hash
         ) order by created_at), '[]'::jsonb)
         from public.audit_events where owner_id = $1",
        user.id,
    )
    .await?;

    sqlx::query(
        "insert into public.audit_events (owner_id, actor_type, actor_id, event_type)
         values ($1, 'owner', $1::text, 'account.exported')",
    )
    .bind(user.id)
    .execute(&state.pool)
    .await?;

    Ok(Json(json!({
        "exported_at": chrono::Utc::now(),
        "profile": profile,
        "profile_facts": facts,
        "documents": documents,
        "participants": participants,
        "audit_events": audit_events
    })))
}

pub async fn delete_account(
    State(state): State<AppState>,
    user: AuthUser,
    Json(input): Json<DeleteAccount>,
) -> AppResult<StatusCode> {
    if !user.has_recent_authentication() {
        return Err(AppError::Unauthorized);
    }
    if input.confirmation != "DELETE" {
        return Err(AppError::Validation(
            "Type DELETE to confirm account deletion".to_owned(),
        ));
    }

    let document_paths = sqlx::query_as::<_, (Option<String>, Option<String>, Option<String>)>(
        "select original_storage_path, preview_storage_path, completed_storage_path
         from public.documents where owner_id = $1",
    )
    .bind(user.id)
    .fetch_all(&state.pool)
    .await?;
    let mut paths = document_paths
        .into_iter()
        .flat_map(|paths| [paths.0, paths.1, paths.2])
        .flatten()
        .collect::<HashSet<_>>();
    paths.extend(
        sqlx::query_scalar::<_, String>(
            "select storage_path from public.document_artifacts where owner_id = $1",
        )
        .bind(user.id)
        .fetch_all(&state.pool)
        .await?,
    );
    for path in paths {
        state
            .storage
            .delete(&state.config.documents_bucket, &path)
            .await?;
    }
    let context_paths = sqlx::query_scalar::<_, String>(
        "select dc.storage_path from public.document_contexts dc
         join public.documents d on d.id = dc.document_id
         where d.owner_id = $1 and dc.storage_path is not null",
    )
    .bind(user.id)
    .fetch_all(&state.pool)
    .await?;
    for path in context_paths {
        state
            .storage
            .delete(&state.config.context_bucket, &path)
            .await?;
    }
    let signature_paths = sqlx::query_scalar::<_, String>(
        "select storage_path from public.signatures where user_id = $1",
    )
    .bind(user.id)
    .fetch_all(&state.pool)
    .await?;
    for path in signature_paths {
        state
            .storage
            .delete(&state.config.signatures_bucket, &path)
            .await?;
    }

    let response = state
        .http
        .delete(format!(
            "{}/auth/v1/admin/users/{}",
            state.config.supabase_url, user.id
        ))
        .header("apikey", &state.config.supabase_service_role_key)
        .bearer_auth(&state.config.supabase_service_role_key)
        .send()
        .await
        .map_err(|_| AppError::Upstream)?;
    if !response.status().is_success() {
        return Err(AppError::Upstream);
    }

    Ok(StatusCode::NO_CONTENT)
}

async fn aggregate_json(
    state: &AppState,
    query: &'static str,
    user_id: uuid::Uuid,
) -> AppResult<Value> {
    sqlx::query_scalar::<_, Value>(query)
        .bind(user_id)
        .fetch_one(&state.pool)
        .await
        .map_err(Into::into)
}
