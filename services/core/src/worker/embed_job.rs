use pgvector::Vector;
use serde::Deserialize;
use sqlx::FromRow;
use uuid::Uuid;

use crate::{
    AppError, AppResult, AppState,
    memory::{embed_texts, is_sensitive_label},
    models::ProcessingJob,
    security::CryptoService,
};

#[derive(FromRow)]
struct MemoryField {
    label: String,
    page_number: i32,
    value_ciphertext: Vec<u8>,
}

#[derive(Deserialize)]
struct EmbedPayload {
    #[serde(default)]
    approved_field_ids: Vec<Uuid>,
}

pub async fn run(state: &AppState, job: &ProcessingJob) -> AppResult<()> {
    let payload: EmbedPayload =
        serde_json::from_value(job.payload.clone()).map_err(AppError::internal)?;
    if payload.approved_field_ids.is_empty() {
        return Ok(());
    }
    let consent = sqlx::query_scalar::<_, bool>(
        "select memory_consent from public.documents
         where id = $1 and owner_id = $2",
    )
    .bind(job.document_id)
    .bind(job.owner_id)
    .fetch_one(&state.pool)
    .await?;
    if !consent {
        return Ok(());
    }

    let fields = sqlx::query_as::<_, MemoryField>(
        "select label, page_number, value_ciphertext
         from public.document_fields
         where document_id = $1
           and id = any($2)
           and confirmed_at is not null
           and value_ciphertext is not null
           and participant_id is null
           and kind not in ('signature', 'declaration')",
    )
    .bind(job.document_id)
    .bind(&payload.approved_field_ids)
    .fetch_all(&state.pool)
    .await?;
    let chunks = fields
        .into_iter()
        .filter(|field| !is_sensitive_label(&field.label))
        .map(|field| {
            let value = String::from_utf8(state.crypto.decrypt(&field.value_ciphertext)?)
                .map_err(AppError::internal)?;
            Ok((field.page_number, format!("{}: {}", field.label, value)))
        })
        .collect::<AppResult<Vec<_>>>()?;
    if chunks.is_empty() {
        return Ok(());
    }

    let inputs: Vec<_> = chunks.iter().map(|(_, content)| content.clone()).collect();
    let input_count = inputs.len();
    let embeddings = embed_texts(state, &inputs).await?.ok_or_else(|| {
        AppError::configuration("AI_API_KEY and EMBEDDING_MODEL are required for document memory")
    })?;

    let mut transaction = state.pool.begin().await?;
    sqlx::query(
        "delete from public.document_chunks
         where document_id = $1 and owner_id = $2",
    )
    .bind(job.document_id)
    .bind(job.owner_id)
    .execute(&mut *transaction)
    .await?;
    for ((page_number, content), embedding) in chunks.into_iter().zip(embeddings) {
        let encrypted = state.crypto.encrypt(content.as_bytes())?;
        let content_hash = CryptoService::hash(content.as_bytes());
        sqlx::query(
            "insert into public.document_chunks (
                document_id, owner_id, page_number, content_ciphertext,
                content_hash, embedding, approved_at
             )
             select $1, $2, $3, $4, $5, $6, now()
             where exists (
                select 1 from public.documents
                where id = $1 and owner_id = $2 and memory_consent
             )",
        )
        .bind(job.document_id)
        .bind(job.owner_id)
        .bind(page_number)
        .bind(encrypted)
        .bind(content_hash)
        .bind(Vector::from(embedding))
        .execute(&mut *transaction)
        .await?;
    }
    sqlx::query(
        "insert into public.audit_events (
            owner_id, document_id, actor_type, event_type, metadata
         ) values (
            $1, $2, 'system', 'memory.indexed',
            jsonb_build_object('chunk_count', $3)
         )",
    )
    .bind(job.owner_id)
    .bind(job.document_id)
    .bind(input_count as i32)
    .execute(&mut *transaction)
    .await?;
    transaction.commit().await?;
    Ok(())
}

pub async fn delete_memory(state: &AppState, job: &ProcessingJob) -> AppResult<()> {
    let mut transaction = state.pool.begin().await?;
    sqlx::query(
        "delete from public.document_chunks
         where document_id = $1 and owner_id = $2",
    )
    .bind(job.document_id)
    .bind(job.owner_id)
    .execute(&mut *transaction)
    .await?;
    sqlx::query(
        "update public.documents set memory_consent = false
         where id = $1 and owner_id = $2",
    )
    .bind(job.document_id)
    .bind(job.owner_id)
    .execute(&mut *transaction)
    .await?;
    transaction.commit().await?;
    Ok(())
}
