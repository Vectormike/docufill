use std::time::Duration;

use axum::{
    extract::FromRequestParts,
    http::{header::AUTHORIZATION, request::Parts},
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use reqwest::header::{HeaderMap, HeaderValue};
use serde::Deserialize;
use uuid::Uuid;

use crate::{AppError, AppResult, AppState, config::Config};

#[derive(Clone)]
pub struct AuthService {
    client: reqwest::Client,
    user_url: String,
    anon_key: HeaderValue,
}

#[derive(Debug, Clone)]
pub struct AuthUser {
    pub id: Uuid,
    pub email: Option<String>,
    pub access_token: String,
    pub authenticated_at: DateTime<Utc>,
    pub authentication_method: String,
}

#[derive(Deserialize)]
struct SupabaseUser {
    id: Uuid,
    email: Option<String>,
    last_sign_in_at: Option<DateTime<Utc>>,
}

#[derive(Deserialize)]
struct JwtClaims {
    #[serde(default)]
    amr: Vec<AuthenticationMethodReference>,
}

#[derive(Deserialize)]
struct AuthenticationMethodReference {
    method: String,
    timestamp: i64,
}

impl AuthService {
    pub fn new(config: &Config) -> AppResult<Self> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .map_err(AppError::internal)?;
        let anon_key = HeaderValue::from_str(&config.supabase_anon_key)
            .map_err(|_| AppError::configuration("SUPABASE_ANON_KEY is invalid"))?;

        Ok(Self {
            client,
            user_url: format!("{}/auth/v1/user", config.supabase_url),
            anon_key,
        })
    }

    pub async fn authenticate(&self, token: &str) -> AppResult<AuthUser> {
        let mut headers = HeaderMap::new();
        headers.insert("apikey", self.anon_key.clone());

        let response = self
            .client
            .get(&self.user_url)
            .headers(headers)
            .bearer_auth(token)
            .send()
            .await
            .map_err(|_| AppError::Upstream)?;

        if rejects_the_token(response.status()) {
            return Err(AppError::Unauthorized);
        }
        if !response.status().is_success() {
            return Err(AppError::Upstream);
        }

        let user: SupabaseUser = response.json().await.map_err(|_| AppError::Upstream)?;
        let jwt_authentication = token_authentication(token)?;
        let (authenticated_at, authentication_method) = jwt_authentication
            .or_else(|| {
                user.last_sign_in_at
                    .map(|timestamp| (timestamp, "supabase".to_owned()))
            })
            .ok_or(AppError::Unauthorized)?;
        Ok(AuthUser {
            id: user.id,
            email: user.email,
            access_token: token.to_owned(),
            authenticated_at,
            authentication_method,
        })
    }
}

impl AuthUser {
    pub fn has_recent_authentication(&self) -> bool {
        Utc::now() - self.authenticated_at <= ChronoDuration::minutes(15)
    }
}

/// Supabase answers 401 when a token is absent and 403 when it is expired or
/// malformed, so both mean the caller must sign in again rather than that the
/// identity provider is unavailable.
fn rejects_the_token(status: reqwest::StatusCode) -> bool {
    status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN
}

fn token_authentication(token: &str) -> AppResult<Option<(DateTime<Utc>, String)>> {
    let payload = token.split('.').nth(1).ok_or(AppError::Unauthorized)?;
    let decoded = URL_SAFE_NO_PAD
        .decode(payload)
        .map_err(|_| AppError::Unauthorized)?;
    let claims: JwtClaims = serde_json::from_slice(&decoded).map_err(|_| AppError::Unauthorized)?;
    claims
        .amr
        .into_iter()
        .max_by_key(|reference| reference.timestamp)
        .map(|reference| {
            DateTime::from_timestamp(reference.timestamp, 0)
                .map(|timestamp| (timestamp, reference.method))
                .ok_or(AppError::Unauthorized)
        })
        .transpose()
}

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let header = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .ok_or(AppError::Unauthorized)?;
        let token = header
            .strip_prefix("Bearer ")
            .filter(|value| !value.is_empty())
            .ok_or(AppError::Unauthorized)?;

        state.auth.authenticate(token).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn token(payload: serde_json::Value) -> String {
        format!(
            "header.{}.signature",
            URL_SAFE_NO_PAD.encode(serde_json::to_vec(&payload).expect("claims"))
        )
    }

    #[test]
    fn uses_authentication_reference_not_token_issue_time() {
        let parsed = token_authentication(&token(serde_json::json!({
            "iat": 9_999_999_999_i64,
            "amr": [
                { "method": "oauth", "timestamp": 1_700_000_000_i64 },
                { "method": "totp", "timestamp": 1_700_000_100_i64 }
            ]
        })))
        .expect("valid token")
        .expect("authentication reference");

        assert_eq!(parsed.0.timestamp(), 1_700_000_100);
        assert_eq!(parsed.1, "totp");
    }

    #[test]
    fn expired_tokens_ask_for_sign_in_instead_of_reporting_an_outage() {
        assert!(rejects_the_token(reqwest::StatusCode::UNAUTHORIZED));
        assert!(rejects_the_token(reqwest::StatusCode::FORBIDDEN));
        assert!(!rejects_the_token(reqwest::StatusCode::TOO_MANY_REQUESTS));
        assert!(!rejects_the_token(
            reqwest::StatusCode::INTERNAL_SERVER_ERROR
        ));
    }

    #[test]
    fn missing_authentication_reference_requires_server_fallback() {
        assert!(
            token_authentication(&token(serde_json::json!({ "iat": 1_700_000_000_i64 })))
                .expect("valid token")
                .is_none()
        );
    }
}
