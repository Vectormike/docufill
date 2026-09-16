use serde::Deserialize;

use crate::{AppError, AppResult, AppState};

const EMBEDDING_DIMENSIONS: usize = 1536;

#[derive(Deserialize)]
struct EmbeddingResponse {
    data: Vec<EmbeddingData>,
}

#[derive(Deserialize)]
struct EmbeddingData {
    index: usize,
    embedding: Vec<f32>,
}

pub(crate) async fn embed_texts(
    state: &AppState,
    inputs: &[String],
) -> AppResult<Option<Vec<Vec<f32>>>> {
    if inputs.is_empty() {
        return Ok(Some(Vec::new()));
    }
    let (Some(api_key), Some(model)) = (&state.config.ai_api_key, &state.config.embedding_model)
    else {
        return Ok(None);
    };

    let response = state
        .http
        .post(format!("{}/embeddings", state.config.ai_base_url))
        .bearer_auth(api_key)
        .json(&serde_json::json!({
            "model": model,
            "input": inputs,
        }))
        .send()
        .await
        .map_err(|_| AppError::Upstream)?;
    if !response.status().is_success() {
        return Err(AppError::Upstream);
    }

    let mut data = response
        .json::<EmbeddingResponse>()
        .await
        .map_err(|_| AppError::Upstream)?
        .data;
    data.sort_by_key(|item| item.index);
    if data.len() != inputs.len()
        || data
            .iter()
            .any(|item| item.embedding.len() != EMBEDDING_DIMENSIONS)
    {
        return Err(AppError::Upstream);
    }
    Ok(Some(data.into_iter().map(|item| item.embedding).collect()))
}
