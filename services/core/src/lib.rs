pub mod ai;
pub mod api;
pub mod auth;
pub mod config;
pub mod error;
pub mod memory;
pub mod models;
pub mod notifications;
pub mod pdf;
pub mod profile;
pub mod security;
pub mod state;
pub mod storage;
pub mod worker;

pub use error::{AppError, AppResult};
pub use state::AppState;

pub async fn shutdown_signal() {
    let ctrl_c = async {
        if let Err(error) = tokio::signal::ctrl_c().await {
            tracing::error!(?error, "failed to install Ctrl+C handler");
        }
    };

    #[cfg(unix)]
    {
        let terminate = async {
            match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
                Ok(mut signal) => {
                    signal.recv().await;
                }
                Err(error) => tracing::error!(?error, "failed to install SIGTERM handler"),
            }
        };
        tokio::select! {
            _ = ctrl_c => {}
            _ = terminate => {}
        }
    }

    #[cfg(not(unix))]
    ctrl_c.await;
}
