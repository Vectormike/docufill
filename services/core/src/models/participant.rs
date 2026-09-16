use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

use super::DocumentField;

#[derive(Debug, Clone, Serialize, FromRow, ToSchema)]
pub struct ParticipantSummary {
    pub id: Uuid,
    pub role: String,
    pub display_name: String,
    pub contact_hint: String,
    pub status: String,
    pub token_expires_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateParticipant {
    pub role: String,
    pub display_name: String,
    pub contact: String,
    pub field_ids: Vec<Uuid>,
    #[serde(default)]
    pub send_invitation: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ParticipantInvitation {
    pub participant: ParticipantSummary,
    pub share_url: String,
    pub invitation_sent: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ParticipantLanding {
    pub document_subject: String,
    pub requester_name: String,
    pub participant_name: String,
    pub role: String,
    pub status: String,
    pub verification_required: bool,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct VerifyParticipant {
    pub code: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ParticipantSession {
    pub session_token: String,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ParticipantAssignment {
    pub document_subject: String,
    pub requester_name: String,
    pub participant_name: String,
    pub role: String,
    pub status: String,
    pub fields: Vec<DocumentField>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ParticipantAnswer {
    pub value: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ParticipantReceipt {
    pub participant_id: Uuid,
    pub document_subject: String,
    pub submitted_at: DateTime<Utc>,
    pub receipt_code: String,
}
