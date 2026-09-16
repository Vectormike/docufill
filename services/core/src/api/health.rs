use axum::{Json, extract::State};
use serde::Serialize;

use crate::{AppResult, AppState};

#[derive(Serialize)]
pub struct Health {
    status: &'static str,
    database: &'static str,
}

pub async fn health(State(state): State<AppState>) -> AppResult<Json<Health>> {
    sqlx::query_scalar::<_, i32>("select 1")
        .fetch_one(&state.pool)
        .await?;
    Ok(Json(Health {
        status: "ok",
        database: "connected",
    }))
}
