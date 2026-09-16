use sqlx::FromRow;
use uuid::Uuid;

use crate::{
    AppResult, AppState,
    models::ProcessingJob,
    pdf::{FieldPlacement, PdfEngine, SignaturePlacement, render_answers},
    security::CryptoService,
};

#[derive(FromRow)]
struct DocumentRecord {
    original_storage_path: String,
}

#[derive(FromRow)]
struct FieldRecord {
    field_key: String,
    kind: String,
    page_number: i32,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    font_size: Option<f64>,
    alignment: String,
    value_ciphertext: Option<Vec<u8>>,
    participant_id: Option<Uuid>,
}

pub async fn run(state: &AppState, pdf: &PdfEngine, job: &ProcessingJob) -> AppResult<()> {
    let document = sqlx::query_as::<_, DocumentRecord>(
        "select original_storage_path from public.documents
         where id = $1 and owner_id = $2",
    )
    .bind(job.document_id)
    .bind(job.owner_id)
    .fetch_one(&state.pool)
    .await?;
    let fields = sqlx::query_as::<_, FieldRecord>(
        "select field_key, kind::text as kind, page_number,
                x::float8 as x, y::float8 as y, width::float8 as width,
                height::float8 as height, font_size::float8 as font_size,
                alignment, value_ciphertext, participant_id
         from public.document_fields
         where document_id = $1
         order by sort_order",
    )
    .bind(job.document_id)
    .fetch_all(&state.pool)
    .await?;

    let original = state
        .storage
        .download(
            &state.config.documents_bucket,
            &document.original_storage_path,
        )
        .await?;
    let placements = fields
        .iter()
        .filter(|field| field.kind != "signature")
        .filter_map(|field| {
            let encrypted = field.value_ciphertext.as_ref()?;
            let value = String::from_utf8(state.crypto.decrypt(encrypted).ok()?).ok()?;
            Some(FieldPlacement {
                key: field.field_key.clone(),
                kind: field.kind.clone(),
                page_number: field.page_number as u16,
                x: field.x as f32,
                y: field.y as f32,
                width: field.width as f32,
                height: field.height as f32,
                font_size: field.font_size.map(|value| value as f32),
                alignment: field.alignment.clone(),
                value,
            })
        })
        .collect::<Vec<_>>();

    let mut signatures = fields
        .iter()
        .filter(|field| field.kind == "signature" && field.participant_id.is_some())
        .filter_map(|field| {
            let encrypted = field.value_ciphertext.as_ref()?;
            let png_bytes = state.crypto.decrypt(encrypted).ok()?;
            Some(SignaturePlacement {
                page_number: field.page_number as u16,
                x: field.x as f32,
                y: field.y as f32,
                width: field.width as f32,
                height: field.height as f32,
                png_bytes,
            })
        })
        .collect::<Vec<_>>();
    if let Some(signature_id) = job
        .payload
        .get("signature_id")
        .and_then(serde_json::Value::as_str)
        .and_then(|value| Uuid::parse_str(value).ok())
    {
        let signature_path = sqlx::query_scalar::<_, String>(
            "select storage_path from public.signatures
             where id = $1 and user_id = $2 and revoked_at is null",
        )
        .bind(signature_id)
        .bind(job.owner_id)
        .fetch_one(&state.pool)
        .await?;
        let encrypted = state
            .storage
            .download(&state.config.signatures_bucket, &signature_path)
            .await?;
        let png_bytes = state.crypto.decrypt(&encrypted)?;
        signatures.extend(
            fields
                .iter()
                .filter(|field| field.kind == "signature" && field.participant_id.is_none())
                .map(|field| SignaturePlacement {
                    page_number: field.page_number as u16,
                    x: field.x as f32,
                    y: field.y as f32,
                    width: field.width as f32,
                    height: field.height as f32,
                    png_bytes: png_bytes.clone(),
                }),
        );
    }

    let engine = pdf.clone();
    let completed = tokio::task::spawn_blocking(move || {
        render_answers(&engine, original, &placements, &signatures)
    })
    .await
    .map_err(crate::AppError::internal)??;
    let completed_hash = CryptoService::hash(&completed);
    let is_preview = job
        .payload
        .get("preview")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    let file_name = if is_preview { "preview" } else { "completed" };
    let completed_path = format!(
        "{}/{}/{}-{}.pdf",
        job.owner_id, job.document_id, file_name, job.id
    );
    state
        .storage
        .upload(
            &state.config.documents_bucket,
            &completed_path,
            completed,
            "application/pdf",
        )
        .await?;

    let mut transaction = state.pool.begin().await?;
    sqlx::query(
        "insert into public.document_artifacts (
            document_id, owner_id, kind, storage_path, content_hash
         ) values ($1, $2, $3, $4, $5)",
    )
    .bind(job.document_id)
    .bind(job.owner_id)
    .bind(file_name)
    .bind(&completed_path)
    .bind(&completed_hash)
    .execute(&mut *transaction)
    .await?;
    if is_preview {
        sqlx::query(
            "update public.documents
             set status = 'ready', progress = 100, preview_storage_path = $2,
                 error_code = null
             where id = $1 and owner_id = $3",
        )
        .bind(job.document_id)
        .bind(&completed_path)
        .bind(job.owner_id)
        .execute(&mut *transaction)
        .await?;
    } else {
        sqlx::query(
            "update public.documents
             set status = 'completed', progress = 100, completed_storage_path = $2,
                 completed_at = now(), error_code = null
             where id = $1 and owner_id = $3",
        )
        .bind(job.document_id)
        .bind(&completed_path)
        .bind(job.owner_id)
        .execute(&mut *transaction)
        .await?;
    }
    let event_type = if is_preview {
        "document.preview_rendered"
    } else {
        "document.completed"
    };
    sqlx::query(
        "insert into public.audit_events (
            owner_id, document_id, actor_type, event_type,
            document_hash, metadata
         ) values (
            $1, $2, 'system', $4, $3, $5
         )",
    )
    .bind(job.owner_id)
    .bind(job.document_id)
    .bind(completed_hash)
    .bind(event_type)
    .bind(&job.payload)
    .execute(&mut *transaction)
    .await?;
    transaction.commit().await?;
    Ok(())
}
