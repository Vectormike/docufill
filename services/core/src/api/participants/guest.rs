use axum::{
    Json,
    extract::{Path, State},
    http::HeaderMap,
};
use chrono::{Duration, Utc};
use sqlx::FromRow;
use subtle::ConstantTimeEq;
use uuid::Uuid;

use crate::{
    AppError, AppResult, AppState,
    models::{
        DocumentField, ParticipantAnswer, ParticipantAssignment, ParticipantLanding,
        ParticipantReceipt, ParticipantSession, VerifyParticipant,
    },
    security::{CryptoService, normalize_signature_data_url, secure_token, verification_code},
};

#[derive(FromRow)]
struct LandingRecord {
    participant_id: Uuid,
    document_id: Uuid,
    owner_id: Uuid,
    document_subject: String,
    requester_name: Option<String>,
    participant_name: String,
    role: String,
    status: String,
}

#[derive(FromRow)]
struct VerificationRecord {
    participant_id: Uuid,
    verification_code_hash: Option<String>,
    verification_expires_at: Option<chrono::DateTime<Utc>>,
    verification_attempts: i16,
}

#[derive(FromRow)]
struct VerificationDelivery {
    contact_ciphertext: Vec<u8>,
    verification_sent_at: Option<chrono::DateTime<Utc>>,
}

pub async fn participant_landing(
    State(state): State<AppState>,
    Path(token): Path<String>,
) -> AppResult<Json<ParticipantLanding>> {
    let record = landing_record(&state, &token).await?;
    let viewed = sqlx::query(
        "update public.participants set status = 'viewed'
         where id = $1 and status = 'invited'",
    )
    .bind(record.participant_id)
    .execute(&state.pool)
    .await?;
    if viewed.rows_affected() > 0 {
        sqlx::query(
            "insert into public.audit_events (
                owner_id, document_id, participant_id, actor_type, actor_id, event_type
             ) values ($1, $2, $3, 'participant', $3::text, 'participant.viewed')",
        )
        .bind(record.owner_id)
        .bind(record.document_id)
        .bind(record.participant_id)
        .execute(&state.pool)
        .await?;
    }
    Ok(Json(ParticipantLanding {
        document_subject: record.document_subject,
        requester_name: record
            .requester_name
            .unwrap_or_else(|| "A Docufill user".to_owned()),
        participant_name: record.participant_name,
        role: record.role,
        status: if record.status == "invited" {
            "viewed".to_owned()
        } else {
            record.status
        },
        verification_required: true,
    }))
}

pub async fn verify_participant(
    State(state): State<AppState>,
    Path(token): Path<String>,
    Json(input): Json<VerifyParticipant>,
) -> AppResult<Json<ParticipantSession>> {
    if input.code.len() != 6
        || !input
            .code
            .chars()
            .all(|character| character.is_ascii_digit())
    {
        return Err(AppError::Validation(
            "Enter the six-digit verification code".to_owned(),
        ));
    }
    let token_hash = CryptoService::hash(token.as_bytes());
    let record = sqlx::query_as::<_, VerificationRecord>(
        "select id as participant_id, verification_code_hash,
                verification_expires_at, verification_attempts
         from public.participants
         where token_hash = $1 and token_expires_at > now()
           and status not in ('revoked', 'completed')",
    )
    .bind(token_hash)
    .fetch_one(&state.pool)
    .await?;
    if record.verification_attempts >= 5 {
        return Err(AppError::Forbidden);
    }
    let expected = record
        .verification_code_hash
        .ok_or(AppError::Unauthorized)?;
    let valid_time = record
        .verification_expires_at
        .is_some_and(|expires_at| expires_at > Utc::now());
    let supplied = CryptoService::hash(input.code.as_bytes());
    let valid_code: bool = expected.as_bytes().ct_eq(supplied.as_bytes()).into();
    if !valid_time || !valid_code {
        sqlx::query(
            "update public.participants
             set verification_attempts = verification_attempts + 1
             where id = $1",
        )
        .bind(record.participant_id)
        .execute(&state.pool)
        .await?;
        return Err(AppError::Unauthorized);
    }

    let session_token = secure_token();
    let session_hash = CryptoService::hash(session_token.as_bytes());
    let expires_at = Utc::now() + Duration::hours(2);
    let mut transaction = state.pool.begin().await?;
    sqlx::query(
        "update public.participants
         set status = 'verified', verified_at = now(),
             verification_code_hash = null, verification_expires_at = null
         where id = $1",
    )
    .bind(record.participant_id)
    .execute(&mut *transaction)
    .await?;
    sqlx::query("delete from public.participant_sessions where participant_id = $1")
        .bind(record.participant_id)
        .execute(&mut *transaction)
        .await?;
    sqlx::query(
        "insert into public.participant_sessions (
            participant_id, token_hash, expires_at
         ) values ($1, $2, $3)",
    )
    .bind(record.participant_id)
    .bind(session_hash)
    .bind(expires_at)
    .execute(&mut *transaction)
    .await?;
    transaction.commit().await?;

    Ok(Json(ParticipantSession {
        session_token,
        expires_at,
    }))
}

pub async fn request_verification_code(
    State(state): State<AppState>,
    Path(token): Path<String>,
) -> AppResult<axum::http::StatusCode> {
    let record = landing_record(&state, &token).await?;
    if matches!(record.status.as_str(), "completed" | "revoked") {
        return Err(AppError::Forbidden);
    }
    let delivery = sqlx::query_as::<_, VerificationDelivery>(
        "select contact_ciphertext, verification_sent_at
         from public.participants where id = $1",
    )
    .bind(record.participant_id)
    .fetch_one(&state.pool)
    .await?;
    if delivery
        .verification_sent_at
        .is_some_and(|sent_at| sent_at > Utc::now() - Duration::minutes(1))
    {
        return Err(AppError::Conflict);
    }

    let code = verification_code();
    let expires_at = Utc::now() + Duration::minutes(15);
    sqlx::query(
        "update public.participants
         set verification_code_hash = $2, verification_expires_at = $3,
             verification_attempts = 0, verification_sent_at = now()
         where id = $1",
    )
    .bind(record.participant_id)
    .bind(CryptoService::hash(code.as_bytes()))
    .bind(expires_at)
    .execute(&state.pool)
    .await?;

    let recipient = String::from_utf8(state.crypto.decrypt(&delivery.contact_ciphertext)?)
        .map_err(AppError::internal)?;
    let share_url = format!("{}/share/{}", state.config.public_app_url, token);
    let sent = state
        .notifications
        .send_participant_invitation(
            &recipient,
            record
                .requester_name
                .as_deref()
                .unwrap_or("A Docufill user"),
            &record.document_subject,
            &share_url,
            &code,
        )
        .await?;
    if !sent {
        return Err(AppError::configuration(
            "Email delivery is required to send a verification code",
        ));
    }
    sqlx::query(
        "insert into public.audit_events (
            owner_id, document_id, participant_id, actor_type, actor_id, event_type
         ) values (
            $1, $2, $3, 'participant', $3::text, 'participant.verification_sent'
         )",
    )
    .bind(record.owner_id)
    .bind(record.document_id)
    .bind(record.participant_id)
    .execute(&state.pool)
    .await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

pub async fn participant_assignment(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(token): Path<String>,
) -> AppResult<Json<ParticipantAssignment>> {
    let record = authorize_session(&state, &headers, &token).await?;
    let fields = sqlx::query_as::<_, DocumentField>(
        "select id, participant_id, field_key, label, instructions,
                kind::text as kind, page_number, x::float8 as x, y::float8 as y,
                width::float8 as width, height::float8 as height,
                font_size::float8 as font_size, alignment,
                value_preview as value, value_preview,
                source::text as source, confidence::float8 as confidence,
                null::text as source_explanation, 0::int as source_reference_count,
                confirmed_at, sort_order
         from public.document_fields
         where document_id = $1 and participant_id = $2
         order by sort_order, created_at",
    )
    .bind(record.document_id)
    .bind(record.participant_id)
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(ParticipantAssignment {
        document_subject: record.document_subject,
        requester_name: record
            .requester_name
            .unwrap_or_else(|| "A Docufill user".to_owned()),
        participant_name: record.participant_name,
        role: record.role,
        status: record.status,
        fields: fields
            .into_iter()
            .map(|mut field| {
                field.required = true;
                field
            })
            .collect(),
    }))
}

pub async fn participant_answer(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((token, field_id)): Path<(String, Uuid)>,
    Json(input): Json<ParticipantAnswer>,
) -> AppResult<Json<DocumentField>> {
    let record = authorize_session(&state, &headers, &token).await?;
    let value = input.value.trim();
    let kind = sqlx::query_scalar::<_, String>(
        "select kind::text from public.document_fields
         where id = $1 and participant_id = $2",
    )
    .bind(field_id)
    .bind(record.participant_id)
    .fetch_one(&state.pool)
    .await?;
    if value.is_empty() || (kind != "signature" && value.len() > 10_000) {
        return Err(AppError::Validation("Answer is invalid".to_owned()));
    }
    let (encrypted, preview, returned_value) = if kind == "signature" {
        let normalized = normalize_signature_data_url(value)?;
        (
            state.crypto.encrypt(&normalized)?,
            "Signature provided".to_owned(),
            "Signature provided".to_owned(),
        )
    } else {
        (
            state.crypto.encrypt(value.as_bytes())?,
            value.chars().take(120).collect::<String>(),
            value.to_owned(),
        )
    };
    let mut transaction = state.pool.begin().await?;
    let field = sqlx::query_as::<_, DocumentField>(
        "update public.document_fields
         set value_ciphertext = $3, value_preview = $4,
             source = 'participant', confirmed_at = now()
         where id = $1 and participant_id = $2
           and exists (
             select 1 from public.documents
             where id = $6 and status in ('needs_input', 'ready', 'failed')
           )
         returning id, participant_id, field_key, label, instructions,
                   kind::text as kind, page_number, x::float8 as x, y::float8 as y,
                   width::float8 as width, height::float8 as height,
                   font_size::float8 as font_size, alignment,
                   $5::text as value, value_preview, source::text as source,
                   confidence::float8 as confidence,
                   null::text as source_explanation, 0::int as source_reference_count,
                   confirmed_at, sort_order",
    )
    .bind(field_id)
    .bind(record.participant_id)
    .bind(encrypted)
    .bind(preview)
    .bind(returned_value)
    .bind(record.document_id)
    .fetch_one(&mut *transaction)
    .await?;
    sqlx::query(
        "update public.participants set status = 'in_progress'
         where id = $1 and status in ('verified', 'viewed')",
    )
    .bind(record.participant_id)
    .execute(&mut *transaction)
    .await?;
    crate::api::documents::refresh_owner_input_status(&mut transaction, record.document_id, None)
        .await?;
    transaction.commit().await?;
    let mut field = field;
    field.required = true;
    Ok(Json(field))
}

pub async fn submit_participant(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(token): Path<String>,
) -> AppResult<Json<ParticipantReceipt>> {
    let record = authorize_session(&state, &headers, &token).await?;
    let missing = sqlx::query_scalar::<_, i64>(
        "select count(*) from public.document_fields
         where participant_id = $1 and value_ciphertext is null",
    )
    .bind(record.participant_id)
    .fetch_one(&state.pool)
    .await?;
    if missing > 0 {
        return Err(AppError::Validation(format!(
            "{missing} assigned answers are still missing"
        )));
    }

    let submitted_at = Utc::now();
    let receipt_code = CryptoService::hash(
        format!(
            "{}:{}:{}",
            record.participant_id,
            record.document_id,
            submitted_at.timestamp_millis()
        )
        .as_bytes(),
    )[..12]
        .to_ascii_uppercase();
    let mut transaction = state.pool.begin().await?;
    sqlx::query(
        "update public.participants
         set status = 'completed', completed_at = $2
         where id = $1",
    )
    .bind(record.participant_id)
    .bind(submitted_at)
    .execute(&mut *transaction)
    .await?;
    sqlx::query("delete from public.participant_sessions where participant_id = $1")
        .bind(record.participant_id)
        .execute(&mut *transaction)
        .await?;
    sqlx::query(
        "insert into public.audit_events (
            owner_id, document_id, participant_id, actor_type, actor_id,
            event_type, metadata
         ) values (
            $1, $2, $3, 'participant', $3::text, 'participant.completed',
            jsonb_build_object('receipt_code', $4)
         )",
    )
    .bind(record.owner_id)
    .bind(record.document_id)
    .bind(record.participant_id)
    .bind(&receipt_code)
    .execute(&mut *transaction)
    .await?;
    transaction.commit().await?;

    Ok(Json(ParticipantReceipt {
        participant_id: record.participant_id,
        document_subject: record.document_subject,
        submitted_at,
        receipt_code,
    }))
}

async fn landing_record(state: &AppState, token: &str) -> AppResult<LandingRecord> {
    let token_hash = CryptoService::hash(token.as_bytes());
    sqlx::query_as::<_, LandingRecord>(
        "select p.id as participant_id, p.document_id, d.owner_id,
                d.subject as document_subject, owner.display_name as requester_name,
                p.display_name as participant_name, p.role, p.status::text as status
         from public.participants p
         join public.documents d on d.id = p.document_id
         join public.profiles owner on owner.id = d.owner_id
         where p.token_hash = $1 and p.token_expires_at > now()
           and p.status <> 'revoked'",
    )
    .bind(token_hash)
    .fetch_one(&state.pool)
    .await
    .map_err(Into::into)
}

async fn authorize_session(
    state: &AppState,
    headers: &HeaderMap,
    token: &str,
) -> AppResult<LandingRecord> {
    let session = headers
        .get("x-participant-session")
        .and_then(|value| value.to_str().ok())
        .filter(|value| !value.is_empty())
        .ok_or(AppError::Unauthorized)?;
    let record = landing_record(state, token).await?;
    let session_hash = CryptoService::hash(session.as_bytes());
    let valid = sqlx::query_scalar::<_, bool>(
        "select exists(
            select 1 from public.participant_sessions
            where participant_id = $1 and token_hash = $2 and expires_at > now()
         )",
    )
    .bind(record.participant_id)
    .bind(session_hash)
    .fetch_one(&state.pool)
    .await?;
    if !valid {
        return Err(AppError::Unauthorized);
    }
    Ok(record)
}
