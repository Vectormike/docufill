use std::time::Duration;

use base64::{Engine, engine::general_purpose::STANDARD};
use reqwest::Client;
use serde::Deserialize;

use crate::{
    AppError, AppResult,
    config::Config,
    pdf::{ExtractedField, RenderedPage, infer_kind},
};

const MAX_FIELDS_PER_PAGE: usize = 40;

#[derive(Debug, Default)]
pub struct DetectedLayout {
    pub title: Option<String>,
    pub fields: Vec<ExtractedField>,
}

#[derive(Clone)]
pub struct FieldDetector {
    client: Client,
    endpoint: String,
    api_key: Option<String>,
    model: Option<String>,
}

#[derive(Deserialize)]
struct CompletionResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: Message,
}

#[derive(Deserialize)]
struct Message {
    content: String,
}

#[derive(Deserialize)]
struct DetectionEnvelope {
    title: Option<String>,
    #[serde(default)]
    fields: Vec<RawField>,
}

#[derive(Deserialize)]
struct RawField {
    label: String,
    #[serde(default)]
    kind: Option<String>,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
}

impl FieldDetector {
    pub fn new(config: &Config) -> AppResult<Self> {
        Ok(Self {
            client: Client::builder()
                .connect_timeout(Duration::from_secs(5))
                .timeout(Duration::from_secs(60))
                .build()
                .map_err(AppError::internal)?,
            endpoint: format!("{}/chat/completions", config.ai_base_url),
            api_key: config.ai_api_key.clone(),
            model: config.ai_model.clone(),
        })
    }

    pub async fn detect(&self, pages: &[RenderedPage]) -> AppResult<DetectedLayout> {
        let (Some(api_key), Some(model)) = (&self.api_key, &self.model) else {
            return Err(AppError::Validation(
                "This looks like a scanned form, and field detection is not configured".to_owned(),
            ));
        };

        let mut detected = DetectedLayout::default();
        for page in pages {
            let envelope = self.detect_page(api_key, model, page).await?;
            if detected.title.is_none() {
                detected.title = envelope
                    .title
                    .filter(|title| (4..=120).contains(&title.chars().count()));
            }
            for (index, field) in envelope
                .fields
                .into_iter()
                .take(MAX_FIELDS_PER_PAGE)
                .enumerate()
            {
                if let Some(extracted) = placement_from_normalized(page, index, field) {
                    detected.fields.push(extracted);
                }
            }
        }
        Ok(detected)
    }

    async fn detect_page(
        &self,
        api_key: &str,
        model: &str,
        page: &RenderedPage,
    ) -> AppResult<DetectionEnvelope> {
        let image = format!(
            "data:image/jpeg;base64,{}",
            STANDARD.encode(&page.jpeg_bytes)
        );
        let response = self
            .client
            .post(&self.endpoint)
            .bearer_auth(api_key)
            .json(&serde_json::json!({
                "model": model,
                "temperature": 0,
                "response_format": { "type": "json_object" },
                "messages": [
                    {
                        "role": "system",
                        "content": "Find every writable blank on this printed form page. Return JSON {\"title\": string|null, \"fields\":[{\"label\",\"kind\",\"x\",\"y\",\"width\",\"height\"}]}. Coordinates are 0-1 fractions of the page, origin top-left, and must cover the writable area not the printed label. kind must be one of text, multiline, date, number, email, phone, address, choice, checkbox, radio, signature, declaration. Use one field for a whole row of character boxes. Number repeated roles (Name of Signatory 1). Skip logos, instructions, and For Bank Use boxes."
                    },
                    {
                        "role": "user",
                        "content": [
                            { "type": "text", "text": format!("Page {} of a scanned form.", page.page_number) },
                            { "type": "image_url", "image_url": { "url": image, "detail": "high" } }
                        ]
                    }
                ]
            }))
            .send()
            .await
            .map_err(|_| AppError::Upstream)?;

        if !response.status().is_success() {
            return Err(AppError::Upstream);
        }
        let completion: CompletionResponse =
            response.json().await.map_err(|_| AppError::Upstream)?;
        let content = completion
            .choices
            .first()
            .ok_or(AppError::Upstream)?
            .message
            .content
            .as_str();
        serde_json::from_str(content).map_err(|_| AppError::Upstream)
    }
}

fn placement_from_normalized(
    page: &RenderedPage,
    index: usize,
    field: RawField,
) -> Option<ExtractedField> {
    let label = field.label.split_whitespace().collect::<Vec<_>>().join(" ");
    if label
        .chars()
        .filter(|character| character.is_alphabetic())
        .count()
        < 3
    {
        return None;
    }
    if !(0.0..=1.0).contains(&field.x)
        || !(0.0..=1.0).contains(&field.y)
        || field.width <= 0.02
        || field.height <= 0.008
        || field.x + field.width > 1.02
        || field.y + field.height > 1.02
    {
        return None;
    }

    let width = (field.width * page.page_width).min(page.page_width - 8.0);
    let height = (field.height * page.page_height).min(page.page_height - 8.0);
    let x = (field.x * page.page_width).clamp(0.0, page.page_width - width);
    let y = (page.page_height - (field.y + field.height) * page.page_height)
        .clamp(0.0, page.page_height - height);
    let kind = sanitize_kind(field.kind.as_deref().unwrap_or_else(|| infer_kind(&label)));

    (width >= 18.0 && height >= 10.0).then(|| ExtractedField {
        key: format!("scan-{}-{}", page.page_number, index),
        label,
        kind: kind.to_owned(),
        page_number: page.page_number,
        x,
        y,
        width,
        height,
    })
}

fn sanitize_kind(kind: &str) -> &'static str {
    match kind.trim().to_ascii_lowercase().as_str() {
        "multiline" => "multiline",
        "date" => "date",
        "number" => "number",
        "email" => "email",
        "phone" => "phone",
        "address" => "address",
        "choice" => "choice",
        "checkbox" => "checkbox",
        "radio" => "radio",
        "signature" => "signature",
        "declaration" => "declaration",
        _ => "text",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn page() -> RenderedPage {
        RenderedPage {
            page_number: 1,
            page_width: 595.0,
            page_height: 842.0,
            jpeg_bytes: Vec::new(),
        }
    }

    #[test]
    fn converts_top_left_fractions_to_pdf_points() {
        let field = placement_from_normalized(
            &page(),
            0,
            RawField {
                label: "Name of Account".to_owned(),
                kind: Some("text".to_owned()),
                x: 0.10,
                y: 0.20,
                width: 0.80,
                height: 0.05,
            },
        )
        .expect("field");

        assert_eq!(field.key, "scan-1-0");
        assert!((field.x - 59.5).abs() < 0.2);
        assert!((field.width - 476.0).abs() < 0.2);
        assert!((field.height - 42.1).abs() < 0.2);
        assert!((field.y - (842.0 - 0.25 * 842.0)).abs() < 0.3);
    }

    #[test]
    fn rejects_tiny_or_off_page_boxes() {
        assert!(
            placement_from_normalized(
                &page(),
                0,
                RawField {
                    label: "Name".to_owned(),
                    kind: None,
                    x: 0.1,
                    y: 0.1,
                    width: 0.01,
                    height: 0.01,
                },
            )
            .is_none()
        );
    }
}
