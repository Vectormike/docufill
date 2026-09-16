use axum::{
    Json,
    extract::{Path, State},
};
use uuid::Uuid;

use crate::{
    AppError, AppResult, AppState,
    auth::AuthUser,
    models::{
        Profile, ProfileFact, ProfileVault, SignatureSummary, UpdateProfile, UpsertProfileFact,
    },
    security::CryptoService,
};

pub async fn get_profile(
    State(state): State<AppState>,
    user: AuthUser,
) -> AppResult<Json<ProfileVault>> {
    sqlx::query("insert into public.profiles (id) values ($1) on conflict (id) do nothing")
        .bind(user.id)
        .execute(&state.pool)
        .await?;

    let profile = sqlx::query_as::<_, Profile>(
        "select id, display_name, avatar_url, onboarding_completed, created_at, updated_at
         from public.profiles where id = $1",
    )
    .bind(user.id)
    .fetch_one(&state.pool)
    .await?;
    let facts = sqlx::query_as::<_, ProfileFact>(
        "select id, namespace, fact_key, value_preview, value_type, sensitivity,
                source_type::text as source_type, source_document_id, confirmed_at,
                (select count(*) from public.document_fields used
                 where used.source_fact_id = facts.id)::bigint as usage_count
         from public.profile_facts facts
         where user_id = $1 and superseded_by is null
         order by namespace, fact_key",
    )
    .bind(user.id)
    .fetch_all(&state.pool)
    .await?;
    let signature = sqlx::query_as::<_, SignatureSummary>(
        "select id, kind, version, created_at, revoked_at
         from public.signatures
         where user_id = $1 and revoked_at is null
         order by version desc limit 1",
    )
    .bind(user.id)
    .fetch_optional(&state.pool)
    .await?;

    Ok(Json(ProfileVault {
        profile,
        facts,
        signature,
    }))
}

pub async fn update_profile(
    State(state): State<AppState>,
    user: AuthUser,
    Json(input): Json<UpdateProfile>,
) -> AppResult<Json<Profile>> {
    if input
        .display_name
        .as_ref()
        .is_some_and(|value| !(2..=100).contains(&value.trim().chars().count()))
    {
        return Err(AppError::Validation(
            "Display name must contain 2 to 100 characters".to_owned(),
        ));
    }

    let profile = sqlx::query_as::<_, Profile>(
        "update public.profiles
         set display_name = coalesce($2, display_name),
             onboarding_completed = coalesce($3, onboarding_completed)
         where id = $1
         returning id, display_name, avatar_url, onboarding_completed, created_at, updated_at",
    )
    .bind(user.id)
    .bind(input.display_name.map(|value| value.trim().to_owned()))
    .bind(input.onboarding_completed)
    .fetch_one(&state.pool)
    .await?;

    Ok(Json(profile))
}

pub async fn upsert_fact(
    State(state): State<AppState>,
    user: AuthUser,
    Json(input): Json<UpsertProfileFact>,
) -> AppResult<Json<ProfileFact>> {
    validate_fact(&input)?;
    if let Some(document_id) = input.source_document_id {
        let owns_document = sqlx::query_scalar::<_, bool>(
            "select exists(
                select 1 from public.documents where id = $1 and owner_id = $2
             )",
        )
        .bind(document_id)
        .bind(user.id)
        .fetch_one(&state.pool)
        .await?;
        if !owns_document {
            return Err(AppError::Forbidden);
        }
    }

    let fact_id = Uuid::new_v4();
    let encrypted = state.crypto.encrypt(input.value.trim().as_bytes())?;
    let preview = preview_for(&input.sensitivity, input.value.trim());
    let mut transaction = state.pool.begin().await?;
    sqlx::query(
        "update public.profile_facts set superseded_by = $4
         where user_id = $1 and namespace = $2 and fact_key = $3
           and superseded_by is null",
    )
    .bind(user.id)
    .bind(&input.namespace)
    .bind(&input.fact_key)
    .bind(fact_id)
    .execute(&mut *transaction)
    .await?;
    let fact = sqlx::query_as::<_, ProfileFact>(
        "insert into public.profile_facts (
            id, user_id, namespace, fact_key, value_ciphertext, value_preview,
            value_type, sensitivity, source_type, source_document_id
         ) values ($1, $2, $3, $4, $5, $6, $7, $8, 'user', $9)
         returning id, namespace, fact_key, value_preview, value_type, sensitivity,
                   source_type::text as source_type, source_document_id, confirmed_at,
                   0::bigint as usage_count",
    )
    .bind(fact_id)
    .bind(user.id)
    .bind(input.namespace)
    .bind(input.fact_key)
    .bind(encrypted)
    .bind(preview)
    .bind(input.value_type)
    .bind(input.sensitivity)
    .bind(input.source_document_id)
    .fetch_one(&mut *transaction)
    .await?;
    transaction.commit().await?;

    Ok(Json(fact))
}

pub async fn delete_fact(
    State(state): State<AppState>,
    user: AuthUser,
    Path(fact_id): Path<Uuid>,
) -> AppResult<axum::http::StatusCode> {
    let deleted = sqlx::query(
        "delete from public.profile_facts
         where id = $1 and user_id = $2 and superseded_by is null",
    )
    .bind(fact_id)
    .bind(user.id)
    .execute(&state.pool)
    .await?;

    if deleted.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    Ok(axum::http::StatusCode::NO_CONTENT)
}

fn validate_fact(input: &UpsertProfileFact) -> AppResult<()> {
    const NAMESPACES: &[&str] = &[
        "identity",
        "contact",
        "address",
        "employment",
        "education",
        "contacts",
        "financial",
        "identification",
        "custom",
    ];
    if !NAMESPACES.contains(&input.namespace.as_str()) {
        return Err(AppError::Validation("Unknown profile section".to_owned()));
    }
    if input.fact_key.trim().is_empty() || input.fact_key.len() > 100 {
        return Err(AppError::Validation("Fact key is invalid".to_owned()));
    }
    if input.value.trim().is_empty() || input.value.len() > 5_000 {
        return Err(AppError::Validation("Fact value is invalid".to_owned()));
    }
    if !matches!(input.sensitivity.as_str(), "personal" | "sensitive") {
        return Err(AppError::Validation(
            "Sensitivity must be personal or sensitive".to_owned(),
        ));
    }
    Ok(())
}

fn preview_for(sensitivity: &str, value: &str) -> Option<String> {
    if sensitivity == "sensitive" {
        Some(CryptoService::mask(value))
    } else {
        Some(value.chars().take(80).collect())
    }
}
