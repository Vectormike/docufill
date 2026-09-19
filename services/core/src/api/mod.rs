mod account;
pub(crate) mod documents;
mod health;
mod participants;
mod profile;
mod signatures;

use axum::{
    Json, Router,
    http::{HeaderName, HeaderValue, Method},
    routing::{delete, get, patch, post},
};
use tower_http::{
    cors::{AllowOrigin, CorsLayer},
    limit::RequestBodyLimitLayer,
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer},
    trace::TraceLayer,
};
use utoipa::OpenApi;

use crate::{AppError, AppResult, AppState, models::*};

#[derive(OpenApi)]
#[openapi(
    components(schemas(
        Profile,
        UpdateProfile,
        ProfileFact,
        UpsertProfileFact,
        ProfileVault,
        SignatureSummary,
        CreateSignature,
        DeleteAccount,
        DocumentSummary,
        CreateDocument,
        DocumentField,
        DocumentDetail,
        CopilotSummary,
        UpdateFieldAnswer,
        AssignField,
        UpdateFieldLayout,
        AnswerTrust,
        CompleteDocument,
        CompletedDocument,
        CompletionQueued,
        DownloadUrl,
        SetMemoryConsent,
        AddTextContext,
        ParticipantSummary,
        CreateParticipant,
        ParticipantInvitation,
        ParticipantLanding,
        VerifyParticipant,
        ParticipantSession,
        ParticipantAssignment,
        ParticipantAnswer,
        ParticipantReceipt
    )),
    info(
        title = "Docufill API",
        version = "0.1.0",
        description = "Privacy-first document completion API"
    )
)]
pub struct ApiDoc;

pub fn router(state: AppState) -> AppResult<Router> {
    let request_id_header = HeaderName::from_static("x-request-id");
    let origins = state
        .config
        .cors_origins()?
        .into_iter()
        .map(|origin| {
            origin
                .parse::<HeaderValue>()
                .map_err(|_| AppError::configuration("WEB_ORIGIN is invalid"))
        })
        .collect::<AppResult<Vec<_>>>()?;
    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::list(origins))
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PATCH,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([
            axum::http::header::AUTHORIZATION,
            axum::http::header::CONTENT_TYPE,
            HeaderName::from_static("x-participant-session"),
        ]);

    Ok(Router::new()
        .route("/health", get(health::health))
        .route("/openapi.json", get(openapi))
        .route(
            "/v1/account",
            get(account::export_account).delete(account::delete_account),
        )
        .route(
            "/v1/profile",
            get(profile::get_profile).patch(profile::update_profile),
        )
        .route("/v1/profile/facts", post(profile::upsert_fact))
        .route("/v1/profile/facts/{fact_id}", delete(profile::delete_fact))
        .route(
            "/v1/profile/signature",
            post(signatures::create_signature).delete(signatures::revoke_signature),
        )
        .route(
            "/v1/documents",
            get(documents::list_documents).post(documents::create_document),
        )
        .route(
            "/v1/documents/{document_id}",
            get(documents::get_document).delete(documents::delete_document),
        )
        .route(
            "/v1/documents/{document_id}/fields/{field_id}",
            patch(documents::update_field).delete(documents::clear_field),
        )
        .route(
            "/v1/documents/{document_id}/fields/{field_id}/assignment",
            patch(documents::assign_field),
        )
        .route(
            "/v1/documents/{document_id}/fields/{field_id}/layout",
            patch(documents::update_field_layout),
        )
        .route(
            "/v1/documents/{document_id}/context",
            post(documents::add_text_context),
        )
        .route(
            "/v1/documents/{document_id}/memory",
            patch(documents::set_memory_consent),
        )
        .route(
            "/v1/documents/{document_id}/complete",
            post(documents::complete_document),
        )
        .route(
            "/v1/documents/{document_id}/preview",
            get(documents::preview_document).post(documents::request_preview),
        )
        .route(
            "/v1/documents/{document_id}/download",
            get(documents::download_document),
        )
        .route(
            "/v1/documents/{document_id}/participants",
            get(participants::list_participants).post(participants::create_participant),
        )
        .route(
            "/v1/documents/{document_id}/participants/{participant_id}",
            delete(participants::revoke_participant),
        )
        .route("/v1/share/{token}", get(participants::participant_landing))
        .route(
            "/v1/share/{token}/verify",
            post(participants::verify_participant),
        )
        .route(
            "/v1/share/{token}/verification-code",
            post(participants::request_verification_code),
        )
        .route(
            "/v1/share/{token}/assignment",
            get(participants::participant_assignment),
        )
        .route(
            "/v1/share/{token}/fields/{field_id}",
            patch(participants::participant_answer),
        )
        .route(
            "/v1/share/{token}/submit",
            post(participants::submit_participant),
        )
        .layer(PropagateRequestIdLayer::new(request_id_header.clone()))
        .layer(SetRequestIdLayer::new(request_id_header, MakeRequestUuid))
        .layer(RequestBodyLimitLayer::new(2 * 1024 * 1024))
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .with_state(state))
}

async fn openapi() -> Json<utoipa::openapi::OpenApi> {
    Json(openapi_document())
}

pub fn openapi_document() -> utoipa::openapi::OpenApi {
    ApiDoc::openapi()
}
