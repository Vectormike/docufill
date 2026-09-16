use docufill_core::{AppResult, AppState, api, config::Config};

#[tokio::main]
async fn main() -> AppResult<()> {
    init_tracing();
    let config = Config::from_env()?;
    let bind = config.api_bind;
    let state = AppState::connect(config).await?;
    let app = api::router(state)?;
    let listener = tokio::net::TcpListener::bind(bind)
        .await
        .map_err(docufill_core::AppError::internal)?;

    tracing::info!(%bind, "Docufill API listening");
    axum::serve(listener, app)
        .with_graceful_shutdown(docufill_core::shutdown_signal())
        .await
        .map_err(docufill_core::AppError::internal)
}

fn init_tracing() {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "docufill_core=info,tower_http=info".into());
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .json()
        .with_target(false)
        .init();
}
