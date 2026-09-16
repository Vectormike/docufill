use futures::{StreamExt, TryStreamExt, stream};
use pgvector::Vector;
use sqlx::FromRow;
use uuid::Uuid;

use crate::{AppError, AppResult, AppState};

use super::embedding::embed_texts;

const MAX_CONCURRENT_RETRIEVALS: usize = 4;
const MATCHES_PER_QUERY: i32 = 3;

#[derive(Debug, Clone)]
pub struct MemoryExcerpt {
    pub id: Uuid,
    pub content: String,
}

#[derive(FromRow)]
struct EncryptedMatch {
    chunk_id: Uuid,
    content_ciphertext: Vec<u8>,
}

pub async fn retrieve_approved_excerpts(
    state: &AppState,
    owner_id: Uuid,
    queries: &[String],
) -> AppResult<Vec<Vec<MemoryExcerpt>>> {
    let has_memory = sqlx::query_scalar::<_, bool>(
        "select exists(
            select 1 from public.document_chunks
            where owner_id = $1 and approved_at is not null
         )",
    )
    .bind(owner_id)
    .fetch_one(&state.pool)
    .await?;
    if !has_memory {
        return Ok(vec![Vec::new(); queries.len()]);
    }

    let Some(embeddings) = embed_texts(state, queries).await? else {
        return Ok(vec![Vec::new(); queries.len()]);
    };
    stream::iter(embeddings.into_iter().enumerate())
        .map(|(index, embedding)| async move {
            let matches = sqlx::query_as::<_, EncryptedMatch>(
                "select id as chunk_id, content_ciphertext
                 from public.match_document_chunks($1, $2, $3)",
            )
            .bind(Vector::from(embedding))
            .bind(owner_id)
            .bind(MATCHES_PER_QUERY)
            .fetch_all(&state.pool)
            .await?;
            let excerpts = matches
                .into_iter()
                .map(|item| {
                    let content =
                        String::from_utf8(state.crypto.decrypt(&item.content_ciphertext)?)
                            .map_err(AppError::internal)?;
                    Ok(MemoryExcerpt {
                        id: item.chunk_id,
                        content,
                    })
                })
                .collect::<AppResult<Vec<_>>>()?;
            Ok::<_, AppError>((index, excerpts))
        })
        .buffer_unordered(MAX_CONCURRENT_RETRIEVALS)
        .try_collect::<Vec<_>>()
        .await
        .map(|mut results| {
            results.sort_by_key(|(index, _)| *index);
            results.into_iter().map(|(_, excerpts)| excerpts).collect()
        })
}
