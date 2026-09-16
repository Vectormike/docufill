use docufill_core::{AppResult, AppState, config::Config, worker::Worker};

#[tokio::main]
async fn main() -> AppResult<()> {
    init_tracing();
    let state = AppState::connect(Config::from_env()?).await?;
    Worker::new(state).run().await
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
