use std::time::Duration;

use reqwest::header::{HeaderMap, HeaderValue};
use serde::Deserialize;
use url::Url;

use crate::{AppError, AppResult, config::Config};

#[derive(Clone)]
pub struct StorageService {
    client: reqwest::Client,
    base_url: Url,
    headers: HeaderMap,
}

#[derive(Deserialize)]
struct SignedUrlResponse {
    #[serde(rename = "signedURL")]
    signed_url: String,
}

impl StorageService {
    pub fn new(config: &Config) -> AppResult<Self> {
        let client = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(3))
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(AppError::internal)?;
        let mut headers = HeaderMap::new();
        let key = HeaderValue::from_str(&config.supabase_service_role_key)
            .map_err(|_| AppError::configuration("SUPABASE_SERVICE_ROLE_KEY is invalid"))?;
        headers.insert("apikey", key.clone());
        headers.insert(
            reqwest::header::AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", config.supabase_service_role_key))
                .map_err(|_| AppError::configuration("SUPABASE_SERVICE_ROLE_KEY is invalid"))?,
        );

        Ok(Self {
            client,
            base_url: Url::parse(&format!("{}/storage/v1/", config.supabase_url))
                .map_err(|_| AppError::configuration("SUPABASE_URL is invalid"))?,
            headers,
        })
    }

    pub async fn upload_encrypted(
        &self,
        bucket: &str,
        path: &str,
        payload: Vec<u8>,
    ) -> AppResult<()> {
        self.upload(bucket, path, payload, "application/octet-stream")
            .await
    }

    pub async fn upload(
        &self,
        bucket: &str,
        path: &str,
        payload: Vec<u8>,
        content_type: &str,
    ) -> AppResult<()> {
        let url = self.object_url("object", bucket, path)?;
        let response = self
            .client
            .post(url)
            .headers(self.headers.clone())
            .header("content-type", content_type)
            .header("x-upsert", "false")
            .body(payload)
            .send()
            .await
            .map_err(|_| AppError::Upstream)?;

        if response.status() == reqwest::StatusCode::CONFLICT {
            return Err(AppError::Conflict);
        }
        if !response.status().is_success() {
            return Err(AppError::Upstream);
        }
        Ok(())
    }

    pub async fn download(&self, bucket: &str, path: &str) -> AppResult<Vec<u8>> {
        let url = self.object_url("object", bucket, path)?;
        let response = self
            .client
            .get(url)
            .headers(self.headers.clone())
            .send()
            .await
            .map_err(|_| AppError::Upstream)?;

        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(AppError::NotFound);
        }
        if !response.status().is_success() {
            return Err(AppError::Upstream);
        }
        Ok(response
            .bytes()
            .await
            .map_err(|_| AppError::Upstream)?
            .to_vec())
    }

    pub async fn signed_download_url(
        &self,
        bucket: &str,
        path: &str,
        expires_in: u32,
    ) -> AppResult<String> {
        let url = self.object_url("object/sign", bucket, path)?;
        let response = self
            .client
            .post(url)
            .headers(self.headers.clone())
            .json(&serde_json::json!({ "expiresIn": expires_in.min(900) }))
            .send()
            .await
            .map_err(|_| AppError::Upstream)?;

        if !response.status().is_success() {
            return Err(AppError::Upstream);
        }
        let signed: SignedUrlResponse = response.json().await.map_err(|_| AppError::Upstream)?;
        absolute_url(&self.base_url, &signed.signed_url)
    }

    pub async fn delete(&self, bucket: &str, path: &str) -> AppResult<()> {
        let url = self.object_url("object", bucket, path)?;
        let response = self
            .client
            .delete(url)
            .headers(self.headers.clone())
            .send()
            .await
            .map_err(|_| AppError::Upstream)?;
        if !response.status().is_success() && response.status() != reqwest::StatusCode::NOT_FOUND {
            return Err(AppError::Upstream);
        }
        Ok(())
    }

    fn object_url(&self, operation: &str, bucket: &str, path: &str) -> AppResult<Url> {
        let mut url = self.base_url.clone();
        {
            let mut segments = url
                .path_segments_mut()
                .map_err(|_| AppError::configuration("SUPABASE_URL cannot be a base URL"))?;
            segments.pop_if_empty();
            for segment in operation.split('/') {
                segments.push(segment);
            }
            segments.push(bucket);
            for segment in path.split('/').filter(|segment| !segment.is_empty()) {
                segments.push(segment);
            }
        }
        Ok(url)
    }
}

/// Supabase returns signed URLs relative to the storage API root, so they must be
/// resolved against that root rather than the project origin.
fn absolute_url(base: &Url, signed_url: &str) -> AppResult<String> {
    base.join(signed_url.trim_start_matches('/'))
        .map(|url| url.to_string())
        .map_err(|_| AppError::Upstream)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signed_urls_keep_the_storage_api_prefix() {
        let base = Url::parse("https://project.supabase.co/storage/v1/").unwrap();

        assert_eq!(
            absolute_url(&base, "/object/sign/documents/owner/preview.pdf?token=abc").unwrap(),
            "https://project.supabase.co/storage/v1/object/sign/documents/owner/preview.pdf?token=abc"
        );
    }
}
