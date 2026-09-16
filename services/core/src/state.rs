use std::{sync::Arc, time::Duration};

use sqlx::{PgPool, postgres::PgPoolOptions};

use crate::{
    AppError, AppResult, auth::AuthService, config::Config, notifications::NotificationService,
    security::CryptoService, storage::StorageService,
};

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
        let pool = PgPoolOptions::new()
            .min_connections(1)
            .max_connections(20)
            .acquire_timeout(Duration::from_secs(5))
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
