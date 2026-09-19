use docufill_core::{
    AppError, AppResult, AppState, config::Config, models::ProcessingJob, worker::Worker,
};

#[tokio::main]
async fn main() -> AppResult<()> {
    init_tracing();
    let state = AppState::connect(Config::from_env()?).await?;
    if let Ok(document_id) = std::env::var("DOCUFILL_EXTRACT_DOCUMENT_ID") {
        return extract_once(state, &document_id).await;
    }
    Worker::new(state).run().await
}

async fn extract_once(state: AppState, document_id: &str) -> AppResult<()> {
    let document_id = uuid::Uuid::parse_str(document_id).map_err(|_| {
        AppError::Validation("DOCUFILL_EXTRACT_DOCUMENT_ID is not a UUID".to_owned())
    })?;
    let owner_id =
        sqlx::query_scalar::<_, uuid::Uuid>("select owner_id from public.documents where id = $1")
            .bind(document_id)
            .fetch_one(&state.pool)
            .await?;
    let job = ProcessingJob {
        id: uuid::Uuid::new_v4(),
        document_id,
        owner_id,
        kind: "extract".to_owned(),
        status: "running".to_owned(),
        payload: serde_json::json!({}),
        attempts: 1,
    };
    Worker::new(state).extract_document(&job).await
}

fn init_tracing() {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "docufill_core=info".into());
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .json()
        .with_target(false)
        .init();
}
