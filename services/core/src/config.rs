use std::{env, net::SocketAddr, path::PathBuf};

use crate::{AppError, AppResult};

#[derive(Clone, Debug)]
pub struct Config {
    pub app_env: String,
    pub api_bind: SocketAddr,
    pub web_origin: String,
    pub database_url: String,
    pub supabase_url: String,
    pub supabase_anon_key: String,
    pub supabase_service_role_key: String,
    pub documents_bucket: String,
    pub signatures_bucket: String,
    pub context_bucket: String,
    pub encryption_key: String,
    pub ai_base_url: String,
    pub ai_api_key: Option<String>,
    pub ai_model: Option<String>,
    pub embedding_model: Option<String>,
    pub public_app_url: String,
    pub email_api_key: Option<String>,
    pub email_from: Option<String>,
    pub run_migrations: bool,
    pub max_document_bytes: usize,
    pub max_document_pages: u16,
    pub pdfium_lib_path: Option<PathBuf>,
    pub unicode_font_path: Option<PathBuf>,
    pub privacy_contact_email: Option<String>,
    pub dpia_completed: bool,
    pub legal_review_approved: bool,
    pub ai_terms_approved: bool,
}

impl Config {
    pub fn from_env() -> AppResult<Self> {
        if dotenvy::dotenv().is_err() {
            dotenvy::from_filename("services/core/.env").ok();
        }

        let config = Self {
            app_env: optional("APP_ENV", "development"),
            api_bind: optional("API_BIND", "127.0.0.1:8080")
                .parse()
                .map_err(|_| AppError::configuration("API_BIND must be a socket address"))?,
            web_origin: optional("WEB_ORIGIN", "http://localhost:5173"),
            database_url: required("DATABASE_URL")?,
            supabase_url: required("SUPABASE_URL")?.trim_end_matches('/').to_owned(),
            supabase_anon_key: required("SUPABASE_ANON_KEY")?,
            supabase_service_role_key: required("SUPABASE_SERVICE_ROLE_KEY")?,
            documents_bucket: optional("DOCUMENTS_BUCKET", "documents"),
            signatures_bucket: optional("SIGNATURES_BUCKET", "signatures"),
            context_bucket: optional("CONTEXT_BUCKET", "document-context"),
            encryption_key: required("ENCRYPTION_KEY")?,
            ai_base_url: optional("AI_BASE_URL", "https://api.openai.com/v1")
                .trim_end_matches('/')
                .to_owned(),
            ai_api_key: non_empty("AI_API_KEY"),
            ai_model: non_empty("AI_MODEL"),
            embedding_model: non_empty("EMBEDDING_MODEL"),
            public_app_url: optional("PUBLIC_APP_URL", "http://localhost:5173")
                .trim_end_matches('/')
                .to_owned(),
            email_api_key: non_empty("EMAIL_API_KEY"),
            email_from: non_empty("EMAIL_FROM"),
            run_migrations: optional("RUN_MIGRATIONS", "false") == "true",
            max_document_bytes: optional("MAX_DOCUMENT_BYTES", "26214400")
                .parse()
                .map_err(|_| AppError::configuration("MAX_DOCUMENT_BYTES must be an integer"))?,
            max_document_pages: optional("MAX_DOCUMENT_PAGES", "100")
                .parse()
                .map_err(|_| AppError::configuration("MAX_DOCUMENT_PAGES must be an integer"))?,
            pdfium_lib_path: non_empty("PDFIUM_LIB_PATH").map(PathBuf::from),
            unicode_font_path: non_empty("UNICODE_FONT_PATH").map(PathBuf::from),
            privacy_contact_email: non_empty("PRIVACY_CONTACT_EMAIL"),
            dpia_completed: flag("DPIA_COMPLETED"),
            legal_review_approved: flag("LEGAL_REVIEW_APPROVED"),
            ai_terms_approved: flag("AI_DATA_PROCESSING_TERMS_APPROVED"),
        };
        config.validate()?;
        Ok(config)
    }

    fn validate(&self) -> AppResult<()> {
        if !matches!(self.app_env.as_str(), "development" | "test" | "production") {
            return Err(AppError::configuration(
                "APP_ENV must be development, test, or production",
            ));
        }
        if !(1..=100).contains(&self.max_document_pages) {
            return Err(AppError::configuration(
                "MAX_DOCUMENT_PAGES must be between 1 and 100",
            ));
        }
        if self.max_document_bytes == 0 || self.max_document_bytes > 25 * 1024 * 1024 {
            return Err(AppError::configuration(
                "MAX_DOCUMENT_BYTES must be between 1 byte and 25 MB",
            ));
        }
        if self.app_env != "production" {
            return Ok(());
        }

        let production_ready = self.web_origin.starts_with("https://")
            && self.public_app_url.starts_with("https://")
            && self.supabase_url.starts_with("https://")
            && self.email_api_key.is_some()
            && self.email_from.is_some()
            && self.ai_api_key.is_some()
            && self.ai_model.is_some()
            && self.embedding_model.is_some()
            && self.privacy_contact_email.is_some()
            && self.unicode_font_path.is_some()
            && self.dpia_completed
            && self.legal_review_approved
            && self.ai_terms_approved;
        if !production_ready {
            return Err(AppError::configuration(
                "production requires HTTPS origins, email and AI configuration, a Unicode font, a privacy contact, completed DPIA, legal review, and approved AI data-processing terms",
            ));
        }
        Ok(())
    }
}

fn required(key: &str) -> AppResult<String> {
    non_empty(key).ok_or_else(|| AppError::configuration(format!("{key} is required")))
}

fn optional(key: &str, default: &str) -> String {
    non_empty(key).unwrap_or_else(|| default.to_owned())
}

fn non_empty(key: &str) -> Option<String> {
    env::var(key).ok().filter(|value| !value.trim().is_empty())
}

fn flag(key: &str) -> bool {
    non_empty(key).is_some_and(|value| value.eq_ignore_ascii_case("true"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn configuration(environment: &str) -> Config {
        Config {
            app_env: environment.to_owned(),
            api_bind: "127.0.0.1:8080".parse().expect("socket"),
            web_origin: "https://app.example.com".to_owned(),
            database_url: "postgresql://local".to_owned(),
            supabase_url: "https://project.supabase.co".to_owned(),
            supabase_anon_key: "anon".to_owned(),
            supabase_service_role_key: "service".to_owned(),
            documents_bucket: "documents".to_owned(),
            signatures_bucket: "signatures".to_owned(),
            context_bucket: "context".to_owned(),
            encryption_key: "key".to_owned(),
            ai_base_url: "https://api.openai.com/v1".to_owned(),
            ai_api_key: Some("secret".to_owned()),
            ai_model: Some("model".to_owned()),
            embedding_model: Some("embedding".to_owned()),
            public_app_url: "https://app.example.com".to_owned(),
            email_api_key: Some("secret".to_owned()),
            email_from: Some("Docufill <mail@example.com>".to_owned()),
            run_migrations: false,
            max_document_bytes: 25 * 1024 * 1024,
            max_document_pages: 100,
            pdfium_lib_path: None,
            unicode_font_path: Some(PathBuf::from("/fonts/NotoSans-Regular.ttf")),
            privacy_contact_email: Some("privacy@example.com".to_owned()),
            dpia_completed: true,
            legal_review_approved: true,
            ai_terms_approved: true,
        }
    }

    #[test]
    fn blocks_production_without_launch_approvals() {
        let mut config = configuration("production");
        config.dpia_completed = false;
        assert!(matches!(config.validate(), Err(AppError::Configuration(_))));
    }

    #[test]
    fn accepts_complete_production_configuration() {
        assert!(configuration("production").validate().is_ok());
    }
}
