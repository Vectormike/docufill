use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, FromRow, ToSchema)]
pub struct DocumentSummary {
    pub id: Uuid,
    pub subject: String,
    pub original_name: String,
    pub status: String,
    pub page_count: Option<i32>,
    pub progress: i16,
    pub memory_consent: bool,
    pub completed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateDocument {
    pub subject: String,
    pub original_name: String,
    pub original_storage_path: String,
    pub content_hash: String,
}

#[derive(Debug, Clone, Serialize, FromRow, ToSchema)]
pub struct DocumentField {
    pub id: Uuid,
    pub participant_id: Option<Uuid>,
    pub field_key: String,
    pub label: String,
    pub instructions: Option<String>,
    pub kind: String,
    pub page_number: i32,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub font_size: Option<f64>,
    pub alignment: String,
    pub value: Option<String>,
    pub value_preview: Option<String>,
    pub source: String,
    pub confidence: Option<f64>,
    pub source_explanation: Option<String>,
    pub source_reference_count: i32,
    pub confirmed_at: Option<DateTime<Utc>>,
    pub sort_order: i32,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct DocumentDetail {
    #[serde(flatten)]
    pub document: DocumentSummary,
    pub fields: Vec<DocumentField>,
    pub copilot: CopilotSummary,
}

#[derive(Debug, Default, Serialize, ToSchema)]
pub struct CopilotSummary {
    pub total_fields: usize,
    pub grounded_fields: usize,
    pub questions_needing_input: usize,
    pub ambiguous_fields: usize,
    pub signature_fields: usize,
    pub participant_sections: usize,
}

impl CopilotSummary {
    pub fn from_fields(fields: &[DocumentField]) -> Self {
        Self {
            total_fields: fields.len(),
            grounded_fields: fields
                .iter()
                .filter(|field| matches!(field.source.as_str(), "profile" | "derived" | "user"))
                .count(),
            questions_needing_input: fields
                .iter()
                .filter(|field| field.source == "missing")
                .count(),
            ambiguous_fields: fields
                .iter()
                .filter(|field| field.source == "ai_draft")
                .count(),
            signature_fields: fields
                .iter()
                .filter(|field| field.kind == "signature")
                .count(),
            participant_sections: fields
                .iter()
                .filter_map(|field| field.participant_id)
                .collect::<std::collections::HashSet<_>>()
                .len(),
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateFieldAnswer {
    pub value: String,
    pub source: AnswerTrust,
    #[serde(default)]
    pub confirmed: bool,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct AssignField {
    pub participant_id: Option<Uuid>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateFieldLayout {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub font_size: Option<f64>,
    pub alignment: String,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum AnswerTrust {
    Profile,
    Derived,
    User,
    Participant,
    AiDraft,
}

impl AnswerTrust {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Profile => "profile",
            Self::Derived => "derived",
            Self::User => "user",
            Self::Participant => "participant",
            Self::AiDraft => "ai_draft",
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CompleteDocument {
    pub apply_signature_id: Option<Uuid>,
    pub consent_text: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CompletedDocument {
    pub document_id: Uuid,
    pub status: String,
    pub download_path: String,
    pub document_hash: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CompletionQueued {
    pub document_id: Uuid,
    pub status: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct DownloadUrl {
    pub url: String,
    pub expires_in: u32,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct SetMemoryConsent {
    pub enabled: bool,
    #[serde(default)]
    pub approved_field_ids: Vec<Uuid>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct AddTextContext {
    pub content: String,
}

#[derive(Debug, Clone, Serialize, FromRow, ToSchema)]
pub struct ProcessingJob {
    pub id: Uuid,
    pub document_id: Uuid,
    pub owner_id: Uuid,
    pub kind: String,
    pub status: String,
    pub payload: serde_json::Value,
    pub attempts: i16,
}
