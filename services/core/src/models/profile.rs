use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, FromRow, ToSchema)]
pub struct Profile {
    pub id: Uuid,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub onboarding_completed: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateProfile {
    pub display_name: Option<String>,
    pub onboarding_completed: Option<bool>,
}

#[derive(Debug, Clone, Serialize, FromRow, ToSchema)]
pub struct ProfileFact {
    pub id: Uuid,
    pub namespace: String,
    pub fact_key: String,
    pub value_preview: Option<String>,
    pub value_type: String,
    pub sensitivity: String,
    pub source_type: String,
    pub source_document_id: Option<Uuid>,
    pub confirmed_at: DateTime<Utc>,
    pub usage_count: i64,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpsertProfileFact {
    pub namespace: String,
    pub fact_key: String,
    pub value: String,
    #[serde(default = "default_value_type")]
    pub value_type: String,
    #[serde(default = "default_sensitivity")]
    pub sensitivity: String,
    pub source_document_id: Option<Uuid>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ProfileVault {
    pub profile: Profile,
    pub facts: Vec<ProfileFact>,
    pub signature: Option<SignatureSummary>,
}

#[derive(Debug, Clone, Serialize, FromRow, ToSchema)]
pub struct SignatureSummary {
    pub id: Uuid,
    pub kind: String,
    pub version: i32,
    pub created_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateSignature {
    pub kind: String,
    pub image_data_url: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct DerivedEmploymentDuration {
    pub years: i32,
    pub months: u32,
    pub display: String,
    pub start_date: String,
    pub as_of: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct DeleteAccount {
    pub confirmation: String,
}

fn default_value_type() -> String {
    "text".to_owned()
}

fn default_sensitivity() -> String {
    "personal".to_owned()
}
