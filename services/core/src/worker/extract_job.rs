use std::collections::{HashMap, HashSet};

use futures::{StreamExt, TryStreamExt, stream};
use sqlx::FromRow;
use uuid::Uuid;

use crate::{
    AppError, AppResult, AppState,
    ai::{AiMapper, FactForMapping, FieldForMapping},
    memory::retrieve_approved_excerpts,
    models::ProcessingJob,
    pdf::{ExtractedDocument, ExtractedField, PdfEngine},
    security::CryptoService,
};

const ANALYZER_VERSION: &str = "digital-pdf-v4";
const MAX_CONCURRENT_MAPPINGS: usize = 4;

#[derive(FromRow)]
struct DocumentRecord {
    original_storage_path: String,
    content_hash: String,
}

#[derive(FromRow)]
struct FactRecord {
    id: Uuid,
    fact_key: String,
    value_ciphertext: Vec<u8>,
}

pub async fn run(state: &AppState, pdf: &PdfEngine, job: &ProcessingJob) -> AppResult<()> {
    let document = sqlx::query_as::<_, DocumentRecord>(
        "select original_storage_path, content_hash
         from public.documents where id = $1 and owner_id = $2",
    )
    .bind(job.document_id)
    .bind(job.owner_id)
    .fetch_one(&state.pool)
    .await?;
    sqlx::query(
        "update public.documents
         set status = 'processing', progress = 10, error_code = null
         where id = $1",
    )
    .bind(job.document_id)
    .execute(&state.pool)
    .await?;

    let bytes = state
        .storage
        .download(
            &state.config.documents_bucket,
            &document.original_storage_path,
        )
        .await?;
    if CryptoService::hash(&bytes) != document.content_hash {
        return Err(AppError::Validation(
            "Uploaded document failed its integrity check".to_owned(),
        ));
    }
    sqlx::query("update public.documents set progress = 25 where id = $1")
        .bind(job.document_id)
        .execute(&state.pool)
        .await?;

    let (extracted, cache_hit) = if let Some(cached) =
        cached_extraction(state, job.owner_id, &document.content_hash).await?
    {
        (cached, true)
    } else {
        let engine = pdf.clone();
        let extracted = tokio::task::spawn_blocking(move || engine.extract(bytes))
            .await
            .map_err(AppError::internal)??;
        store_extraction(state, job.owner_id, &document.content_hash, &extracted).await?;
        (extracted, false)
    };
    if let Some(subject) = super::document_title::suggest(&extracted) {
        sqlx::query("update public.documents set subject = $2 where id = $1 and owner_id = $3")
            .bind(job.document_id)
            .bind(subject)
            .bind(job.owner_id)
            .execute(&state.pool)
            .await?;
    }
    sqlx::query("update public.documents set progress = 45 where id = $1")
        .bind(job.document_id)
        .execute(&state.pool)
        .await?;

    let fact_records = sqlx::query_as::<_, FactRecord>(
        "select id, fact_key, value_ciphertext
         from public.profile_facts
         where user_id = $1 and superseded_by is null",
    )
    .bind(job.owner_id)
    .fetch_all(&state.pool)
    .await?;
    let mut facts = fact_records
        .into_iter()
        .map(|fact| {
            let value = String::from_utf8(state.crypto.decrypt(&fact.value_ciphertext)?)
                .map_err(AppError::internal)?;
            Ok(FactForMapping {
                id: fact.id,
                key: fact.fact_key,
                value,
                allow_exact: true,
            })
        })
        .collect::<AppResult<Vec<_>>>()?;
    let profile_fact_ids: HashSet<_> = facts.iter().map(|fact| fact.id).collect();
    if !facts.iter().any(|fact| fact.key == "full_name") {
        let display_name = sqlx::query_scalar::<_, Option<String>>(
            "select display_name from public.profiles where id = $1",
        )
        .bind(job.owner_id)
        .fetch_one(&state.pool)
        .await?;
        if let Some(display_name) = display_name.filter(|name| !name.trim().is_empty()) {
            facts.push(FactForMapping {
                id: job.owner_id,
                key: "full_name".to_owned(),
                value: display_name,
                allow_exact: true,
            });
        }
    }
    let mapper = AiMapper::new(&state.config)?;
    let fields = unique_fields(extracted.fields);
    let memory_queries = fields
        .iter()
        .map(|field| field.label.clone())
        .collect::<Vec<_>>();
    let retrieved_memory =
        match retrieve_approved_excerpts(state, job.owner_id, &memory_queries).await {
            Ok(memory) => memory,
            Err(AppError::Upstream) => vec![Vec::new(); fields.len()],
            Err(error) => return Err(error),
        };
    let mut mapped = stream::iter(fields.into_iter().zip(retrieved_memory).enumerate())
        .map(|(sort_order, (field, memory))| {
            let mapper = &mapper;
            let facts = &facts;
            async move {
                let field_id = Uuid::new_v4();
                let request = FieldForMapping {
                    id: field_id,
                    label: field.label.clone(),
                    instructions: None,
                    kind: field.kind.clone(),
                };
                let mut scoped_facts = facts.clone();
                scoped_facts.extend(memory.into_iter().map(|excerpt| FactForMapping {
                    id: excerpt.id,
                    key: field.label.clone(),
                    value: excerpt.content,
                    allow_exact: false,
                }));
                let mapping = match mapper.map(&request, &scoped_facts).await {
                    Ok(mapping) => mapping,
                    Err(AppError::Upstream) => None,
                    Err(error) => return Err(error),
                };
                Ok::<_, AppError>((sort_order, field_id, field, mapping))
            }
        })
        .buffer_unordered(MAX_CONCURRENT_MAPPINGS)
        .try_collect::<Vec<_>>()
        .await?;
    mapped.sort_by_key(|(sort_order, ..)| *sort_order);
    sqlx::query("update public.documents set progress = 75 where id = $1")
        .bind(job.document_id)
        .execute(&state.pool)
        .await?;

    let mut transaction = state.pool.begin().await?;
    sqlx::query("delete from public.document_fields where document_id = $1")
        .bind(job.document_id)
        .execute(&mut *transaction)
        .await?;
    let mut needs_input = false;
    for (sort_order, field_id, field, mapping) in mapped {
        let direct = mapping.as_ref().is_some_and(|value| value.deterministic);
        let source = if mapping.is_none() {
            needs_input = true;
            "missing"
        } else if direct {
            "profile"
        } else {
            needs_input = true;
            "ai_draft"
        };
        let encrypted = mapping
            .as_ref()
            .map(|value| state.crypto.encrypt(value.value.as_bytes()))
            .transpose()?;
        let preview = mapping
            .as_ref()
            .map(|value| value.value.chars().take(120).collect::<String>());
        let confidence = mapping.as_ref().map(|value| value.confidence);
        let source_fact_id = mapping
            .as_ref()
            .and_then(|value| {
                value
                    .source_fact_ids
                    .iter()
                    .find(|id| profile_fact_ids.contains(id))
            })
            .copied();

        sqlx::query(
            "insert into public.document_fields (
                id, document_id, field_key, label, kind, page_number,
                x, y, width, height, value_ciphertext, value_preview,
                source, source_fact_id, confidence, confirmed_at, sort_order
             ) values (
                $1, $2, $3, $4, $5::public.field_kind, $6,
                $7, $8, $9, $10, $11, $12,
                $13::public.answer_source, $14, $15,
                case when $16 then now() else null end, $17
             )",
        )
        .bind(field_id)
        .bind(job.document_id)
        .bind(&field.key)
        .bind(&field.label)
        .bind(&field.kind)
        .bind(i32::from(field.page_number))
        .bind(field.x)
        .bind(field.y)
        .bind(field.width)
        .bind(field.height)
        .bind(encrypted.clone())
        .bind(preview)
        .bind(source)
        .bind(source_fact_id)
        .bind(confidence)
        .bind(direct)
        .bind(sort_order as i32)
        .execute(&mut *transaction)
        .await?;

        if let Some(proposal) = mapping.filter(|_| !direct) {
            sqlx::query(
                "insert into public.answer_proposals (
                    field_id, source, value_ciphertext, source_fact_ids, explanation
                 ) values ($1, 'ai_draft', $2, $3, $4)",
            )
            .bind(field_id)
            .bind(encrypted.expect("mapped value is encrypted"))
            .bind(proposal.source_fact_ids)
            .bind(proposal.explanation)
            .execute(&mut *transaction)
            .await?;
        }
    }

    let status = if needs_input { "needs_input" } else { "ready" };
    sqlx::query(
        "update public.documents
         set status = $2::public.document_status, progress = 100,
             page_count = $3, error_code = null
         where id = $1",
    )
    .bind(job.document_id)
    .bind(status)
    .bind(i32::from(extracted.page_count))
    .execute(&mut *transaction)
    .await?;
    sqlx::query(
        "insert into public.audit_events (
            owner_id, document_id, actor_type, event_type, metadata
         ) values (
            $1, $2, 'system', 'document.extracted',
            jsonb_build_object(
                'field_count', $3, 'page_count', $4,
                'analyzer_version', $5, 'cache_hit', $6
            )
         )",
    )
    .bind(job.owner_id)
    .bind(job.document_id)
    .bind(fields_count(&mut transaction, job.document_id).await?)
    .bind(i32::from(extracted.page_count))
    .bind(ANALYZER_VERSION)
    .bind(cache_hit)
    .execute(&mut *transaction)
    .await?;
    transaction.commit().await?;
    Ok(())
}

async fn cached_extraction(
    state: &AppState,
    owner_id: Uuid,
    content_hash: &str,
) -> AppResult<Option<ExtractedDocument>> {
    let encrypted = sqlx::query_scalar::<_, Vec<u8>>(
        "select payload_ciphertext from public.document_extractions
         where owner_id = $1 and content_hash = $2 and analyzer_version = $3",
    )
    .bind(owner_id)
    .bind(content_hash)
    .bind(ANALYZER_VERSION)
    .fetch_optional(&state.pool)
    .await?;
    encrypted
        .map(|payload| {
            let decrypted = state.crypto.decrypt(&payload)?;
            serde_json::from_slice(&decrypted).map_err(AppError::internal)
        })
        .transpose()
}

async fn store_extraction(
    state: &AppState,
    owner_id: Uuid,
    content_hash: &str,
    extracted: &ExtractedDocument,
) -> AppResult<()> {
    let payload = serde_json::to_vec(extracted).map_err(AppError::internal)?;
    let encrypted = state.crypto.encrypt(&payload)?;
    sqlx::query(
        "insert into public.document_extractions (
            owner_id, content_hash, analyzer_version, payload_ciphertext
         ) values ($1, $2, $3, $4)
         on conflict (owner_id, content_hash, analyzer_version) do nothing",
    )
    .bind(owner_id)
    .bind(content_hash)
    .bind(ANALYZER_VERSION)
    .bind(encrypted)
    .execute(&state.pool)
    .await?;
    Ok(())
}

fn unique_fields(fields: Vec<ExtractedField>) -> Vec<ExtractedField> {
    let mut seen = HashMap::<String, usize>::new();
    fields
        .into_iter()
        .map(|mut field| {
            let count = seen.entry(field.key.clone()).or_default();
            if *count > 0 {
                field.key = format!("{}-p{}-{}", field.key, field.page_number, count);
            }
            *count += 1;
            field
        })
        .collect()
}

async fn fields_count(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    document_id: Uuid,
) -> AppResult<i64> {
    sqlx::query_scalar("select count(*) from public.document_fields where document_id = $1")
        .bind(document_id)
        .fetch_one(&mut **transaction)
        .await
        .map_err(Into::into)
}
