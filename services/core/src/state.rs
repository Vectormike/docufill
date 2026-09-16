use std::{sync::Arc, time::Duration};

use sqlx::{PgPool, postgres::PgPoolOptions};

use crate::{
    AppError, AppResult, auth::AuthService, config::Config, notifications::NotificationService,
    security::CryptoService, storage::StorageService,
};

const DATABASE_MAX_CONNECTIONS: u32 = 10;
const DATABASE_ACQUIRE_TIMEOUT: Duration = Duration::from_secs(10);
const DATABASE_IDLE_TIMEOUT: Duration = Duration::from_secs(60);
const DATABASE_MAX_LIFETIME: Duration = Duration::from_secs(5 * 60);

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub pool: PgPool,
    pub http: reqwest::Client,
    pub auth: AuthService,
    pub crypto: CryptoService,
    pub storage: StorageService,
    pub notifications: NotificationService,
}

impl AppState {
    pub async fn connect(config: Config) -> AppResult<Self> {
        let pool = database_pool_options()
            .connect(&config.database_url)
            .await
            .map_err(AppError::internal)?;

        if config.run_migrations {
            sqlx::migrate!("../../supabase/migrations")
                .run(&pool)
                .await
                .map_err(AppError::internal)?;
        }

        let http = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(3))
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(AppError::internal)?;
        let auth = AuthService::new(&config)?;
        let crypto = CryptoService::from_base64(&config.encryption_key)?;
        let storage = StorageService::new(&config)?;
        let notifications = NotificationService::new(&config);

        Ok(Self {
            config: Arc::new(config),
            pool,
            http,
            auth,
            crypto,
            storage,
            notifications,
        })
    }
}

fn database_pool_options() -> PgPoolOptions {
    PgPoolOptions::new()
        .min_connections(0)
        .max_connections(DATABASE_MAX_CONNECTIONS)
        .acquire_timeout(DATABASE_ACQUIRE_TIMEOUT)
        .idle_timeout(DATABASE_IDLE_TIMEOUT)
        .max_lifetime(DATABASE_MAX_LIFETIME)
        .test_before_acquire(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn database_pool_recycles_connections() {
        let options = database_pool_options();

        assert_eq!(options.get_min_connections(), 0);
        assert_eq!(options.get_max_connections(), DATABASE_MAX_CONNECTIONS);
        assert_eq!(options.get_acquire_timeout(), DATABASE_ACQUIRE_TIMEOUT);
        assert_eq!(options.get_idle_timeout(), Some(DATABASE_IDLE_TIMEOUT));
        assert_eq!(options.get_max_lifetime(), Some(DATABASE_MAX_LIFETIME));
        assert!(options.get_test_before_acquire());
    }
}
