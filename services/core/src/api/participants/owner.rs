use std::collections::HashSet;

use axum::{
    Json,
    extract::{Path, State},
};
use chrono::{Duration, Utc};
use sqlx::FromRow;
use uuid::Uuid;

use crate::{
    AppError, AppResult, AppState,
    auth::AuthUser,
    models::{CreateParticipant, ParticipantInvitation, ParticipantSummary},
    security::{CryptoService, secure_token, verification_code},
};

#[derive(FromRow)]
struct ParticipantRecord {
    id: Uuid,
    role: String,
    display_name: String,
    contact_ciphertext: Vec<u8>,
    status: String,
    token_expires_at: Option<chrono::DateTime<Utc>>,
    completed_at: Option<chrono::DateTime<Utc>>,
}

pub async fn list_participants(
    State(state): State<AppState>,
    user: AuthUser,
    Path(document_id): Path<Uuid>,
) -> AppResult<Json<Vec<ParticipantSummary>>> {
    let rows = sqlx::query_as::<_, ParticipantRecord>(
        "select p.id, p.role, p.display_name, p.contact_ciphertext,
                p.status::text as status, p.token_expires_at, p.completed_at
         from public.participants p
         join public.documents d on d.id = p.document_id
         where p.document_id = $1 and d.owner_id = $2
         order by p.created_at",
    )
    .bind(document_id)
    .bind(user.id)
    .fetch_all(&state.pool)
    .await?;

    let participants = rows
        .into_iter()
        .map(|row| to_summary(&state, row))
        .collect::<AppResult<Vec<_>>>()?;
    Ok(Json(participants))
}

pub async fn create_participant(
    State(state): State<AppState>,
    user: AuthUser,
    Path(document_id): Path<Uuid>,
    Json(input): Json<CreateParticipant>,
) -> AppResult<Json<ParticipantInvitation>> {
    validate_input(&input)?;
    if input.send_invitation && !state.notifications.can_deliver() {
        return Err(AppError::configuration(
            "Email delivery is required to invite someone",
        ));
    }
    let document = sqlx::query_as::<_, (String, Option<String>)>(
        "select d.subject, p.display_name
         from public.documents d
         join public.profiles p on p.id = d.owner_id
         where d.id = $1 and d.owner_id = $2
           and d.status in ('needs_input', 'ready', 'failed')",
    )
    .bind(document_id)
    .bind(user.id)
    .fetch_one(&state.pool)
    .await?;

    let participant_id = Uuid::new_v4();
    let token = secure_token();
    let code = verification_code();
    let token_hash = CryptoService::hash(token.as_bytes());
    let contact = input.contact.trim().to_ascii_lowercase();
    let contact_hash = CryptoService::hash(contact.as_bytes());
    let contact_ciphertext = state.crypto.encrypt(contact.as_bytes())?;
    let code_hash = CryptoService::hash(code.as_bytes());
    let token_expires_at = Utc::now() + Duration::days(7);
    let verification_expires_at = Utc::now() + Duration::minutes(15);

    let mut transaction = state.pool.begin().await?;
    let row = sqlx::query_as::<_, ParticipantRecord>(
        "insert into public.participants (
            id, document_id, role, display_name, contact_ciphertext, contact_hash,
            token_hash, token_expires_at, verification_code_hash,
            verification_expires_at, verification_sent_at, status
         ) values (
            $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, 'invited'
         )
         returning id, role, display_name, contact_ciphertext,
                   status::text as status, token_expires_at, completed_at",
    )
    .bind(participant_id)
    .bind(document_id)
    .bind(input.role.trim())
    .bind(input.display_name.trim())
    .bind(contact_ciphertext)
    .bind(contact_hash)
    .bind(token_hash)
    .bind(token_expires_at)
    .bind(code_hash)
    .bind(verification_expires_at)
    .bind(input.send_invitation.then(Utc::now))
    .fetch_one(&mut *transaction)
    .await?;
    let field_ids = input.field_ids.iter().copied().collect::<HashSet<_>>();
    let assigned = sqlx::query(
        "update public.document_fields f
         set participant_id = $4, value_ciphertext = null, value_preview = null,
             source = 'missing', source_fact_id = null, confidence = null,
             confirmed_at = null
         from public.documents d
         where f.id = any($1) and f.document_id = $2
           and d.id = f.document_id and d.owner_id = $3",
    )
    .bind(field_ids.iter().copied().collect::<Vec<_>>())
    .bind(document_id)
    .bind(user.id)
    .bind(participant_id)
    .execute(&mut *transaction)
    .await?;
    if assigned.rows_affected() != field_ids.len() as u64 {
        return Err(AppError::Validation(
            "One or more assigned fields are invalid".to_owned(),
        ));
    }
    sqlx::query(
        "update public.documents
         set status = 'needs_input', preview_storage_path = null
         where id = $1 and owner_id = $2",
    )
    .bind(document_id)
    .bind(user.id)
    .execute(&mut *transaction)
    .await?;

    // Send before committing: a delivered invite that nobody can open would
    // hold the owner's own fields hostage, so a failed send must take the
    // participant and the field assignments with it.
    let share_url = format!("{}/share/{}", state.config.public_app_url, token);
    let requester = document.1.as_deref().unwrap_or("A Docufill user");
    let invitation_sent = if input.send_invitation {
        state
            .notifications
            .send_participant_invitation(&contact, requester, &document.0, &share_url, &code)
            .await?
    } else {
        false
    };
    transaction.commit().await?;
    sqlx::query(
        "insert into public.audit_events (
            owner_id, document_id, participant_id, actor_type, actor_id,
            event_type, metadata
         ) values (
            $1, $2, $3, 'owner', $1::text, 'participant.invited',
            jsonb_build_object('delivery_requested', $4)
         )",
    )
    .bind(user.id)
    .bind(document_id)
    .bind(participant_id)
    .bind(input.send_invitation)
    .execute(&state.pool)
    .await?;

    Ok(Json(ParticipantInvitation {
        participant: to_summary(&state, row)?,
        share_url,
        invitation_sent,
    }))
}

pub async fn revoke_participant(
    State(state): State<AppState>,
    user: AuthUser,
    Path((document_id, participant_id)): Path<(Uuid, Uuid)>,
) -> AppResult<axum::http::StatusCode> {
    let mut transaction = state.pool.begin().await?;
    sqlx::query(
        "delete from public.participant_sessions
         where participant_id = $1",
    )
    .bind(participant_id)
    .execute(&mut *transaction)
    .await?;
    let revoked = sqlx::query(
        "update public.participants p
         set status = 'revoked', token_hash = null, token_expires_at = null
         from public.documents d
         where p.id = $1 and p.document_id = $2
           and d.id = p.document_id and d.owner_id = $3",
    )
    .bind(participant_id)
    .bind(document_id)
    .bind(user.id)
    .execute(&mut *transaction)
    .await?;
    if revoked.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    sqlx::query(
        "update public.document_fields
         set participant_id = null, value_ciphertext = null, value_preview = null,
             source = 'missing', source_fact_id = null, confidence = null,
             confirmed_at = null
         where document_id = $1 and participant_id = $2",
    )
    .bind(document_id)
    .bind(participant_id)
    .execute(&mut *transaction)
    .await?;
    sqlx::query(
        "update public.documents
         set status = 'needs_input', preview_storage_path = null
         where id = $1 and owner_id = $2",
    )
    .bind(document_id)
    .bind(user.id)
    .execute(&mut *transaction)
    .await?;
    sqlx::query(
        "insert into public.audit_events (
            owner_id, document_id, participant_id, actor_type, actor_id, event_type
         ) values ($1, $2, $3, 'owner', $1::text, 'participant.revoked')",
    )
    .bind(user.id)
    .bind(document_id)
    .bind(participant_id)
    .execute(&mut *transaction)
    .await?;
    transaction.commit().await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

fn to_summary(state: &AppState, row: ParticipantRecord) -> AppResult<ParticipantSummary> {
    let contact = String::from_utf8(state.crypto.decrypt(&row.contact_ciphertext)?)
        .map_err(AppError::internal)?;
    Ok(ParticipantSummary {
        id: row.id,
        role: row.role,
        display_name: row.display_name,
        contact_hint: mask_email(&contact),
        status: row.status,
        token_expires_at: row.token_expires_at,
        completed_at: row.completed_at,
    })
}

fn validate_input(input: &CreateParticipant) -> AppResult<()> {
    if !(2..=100).contains(&input.display_name.trim().chars().count()) {
        return Err(AppError::Validation(
            "Participant name must contain 2 to 100 characters".to_owned(),
        ));
    }
    if !matches!(input.role.as_str(), "guarantor" | "co_applicant" | "other") {
        return Err(AppError::Validation(
            "Participant role is invalid".to_owned(),
        ));
    }
    if input.field_ids.is_empty() || input.field_ids.len() > 100 {
        return Err(AppError::Validation(
            "Assign at least one and at most 100 questions".to_owned(),
        ));
    }
    let contact = input.contact.trim();
    if !contact.contains('@') || contact.contains(char::is_whitespace) || contact.len() > 254 {
        return Err(AppError::Validation(
            "A valid participant email is required".to_owned(),
        ));
    }
    Ok(())
}

fn mask_email(email: &str) -> String {
    let Some((name, domain)) = email.split_once('@') else {
        return "••••".to_owned();
    };
    let first = name.chars().next().unwrap_or('•');
    format!("{first}•••@{domain}")
}
