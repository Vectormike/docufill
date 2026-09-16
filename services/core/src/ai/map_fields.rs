use std::{collections::HashSet, time::Duration};

use reqwest::Client;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{AppError, AppResult, config::Config};

#[derive(Debug, Clone)]
pub struct FactForMapping {
    pub id: Uuid,
    pub key: String,
    pub value: String,
    pub allow_exact: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct FieldForMapping {
    pub id: Uuid,
    pub label: String,
    pub instructions: Option<String>,
    pub kind: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GroundedMapping {
    pub field_id: Uuid,
    pub value: String,
    pub source_fact_ids: Vec<Uuid>,
    pub confidence: f64,
    pub explanation: String,
}

#[derive(Clone)]
pub struct AiMapper {
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
struct MappingEnvelope {
    mappings: Vec<GroundedMapping>,
}

impl AiMapper {
    pub fn new(config: &Config) -> AppResult<Self> {
        Ok(Self {
            client: Client::builder()
                .connect_timeout(Duration::from_secs(3))
                .timeout(Duration::from_secs(20))
                .build()
                .map_err(AppError::internal)?,
            endpoint: format!("{}/chat/completions", config.ai_base_url),
            api_key: config.ai_api_key.clone(),
            model: config.ai_model.clone(),
        })
    }

    pub async fn map(
        &self,
        field: &FieldForMapping,
        facts: &[FactForMapping],
    ) -> AppResult<Option<GroundedMapping>> {
        if let Some(mapping) = deterministic_match(field, facts) {
            return Ok(Some(mapping));
        }

        let (Some(api_key), Some(model)) = (&self.api_key, &self.model) else {
            return Ok(None);
        };
        let relevant = select_relevant_facts(field, facts);
        if relevant.is_empty() {
            return Ok(None);
        }

        let allowed_ids: HashSet<Uuid> = relevant.iter().map(|fact| fact.id).collect();
        let fact_payload: Vec<_> = relevant
            .iter()
            .map(|fact| {
                serde_json::json!({
                    "id": fact.id,
                    "key": fact.key,
                    "value": fact.value,
                })
            })
            .collect();
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
                        "content": "Map the field using only supplied facts. Return {\"mappings\":[]} when facts are insufficient. Never invent a value. Return JSON with field_id, value, source_fact_ids, confidence from 0 to 1, and a short explanation."
                    },
                    {
                        "role": "user",
                        "content": serde_json::json!({
                            "field": field,
                            "facts": fact_payload
                        }).to_string()
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
        let envelope: MappingEnvelope =
            serde_json::from_str(content).map_err(|_| AppError::Upstream)?;
        let Some(mapping) = envelope.mappings.into_iter().next() else {
            return Ok(None);
        };

        let is_grounded = mapping.field_id == field.id
            && !mapping.source_fact_ids.is_empty()
            && mapping
                .source_fact_ids
                .iter()
                .all(|id| allowed_ids.contains(id))
            && (0.0..=1.0).contains(&mapping.confidence);
        Ok(is_grounded.then_some(mapping))
    }
}

fn deterministic_match(
    field: &FieldForMapping,
    facts: &[FactForMapping],
) -> Option<GroundedMapping> {
    let field_key = normalize(&field.label);
    facts.iter().find_map(|fact| {
        let fact_key = normalize(&fact.key);
        (fact.allow_exact && (field_key == fact_key || field_key.ends_with(&fact_key))).then(|| {
            GroundedMapping {
                field_id: field.id,
                value: fact.value.clone(),
                source_fact_ids: vec![fact.id],
                confidence: 1.0,
                explanation: "Exact confirmed profile fact".to_owned(),
            }
        })
    })
}

fn select_relevant_facts<'a>(
    field: &FieldForMapping,
    facts: &'a [FactForMapping],
) -> Vec<&'a FactForMapping> {
    let context = normalize(&format!(
        "{} {}",
        field.label,
        field.instructions.as_deref().unwrap_or_default()
    ));
    let tokens: HashSet<&str> = context
        .split_whitespace()
        .filter(|token| token.len() > 2)
        .collect();

    facts
        .iter()
        .filter(|fact| {
            normalize(&fact.key)
                .split_whitespace()
                .any(|token| tokens.contains(token))
        })
        .take(6)
        .collect()
}

fn normalize(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_matching_does_not_need_ai() {
        let field = FieldForMapping {
            id: Uuid::new_v4(),
            label: "Employer".to_owned(),
            instructions: None,
            kind: "text".to_owned(),
        };
        let fact = FactForMapping {
            id: Uuid::new_v4(),
            key: "employer".to_owned(),
            value: "Bujeti".to_owned(),
            allow_exact: true,
        };

        let mapped = deterministic_match(&field, &[fact]).expect("mapping");
        assert_eq!(mapped.value, "Bujeti");
        assert_eq!(mapped.confidence, 1.0);
    }

    #[test]
    fn relevant_selection_excludes_unrelated_facts() {
        let field = FieldForMapping {
            id: Uuid::new_v4(),
            label: "Current employer".to_owned(),
            instructions: None,
            kind: "text".to_owned(),
        };
        let employer = FactForMapping {
            id: Uuid::new_v4(),
            key: "employer name".to_owned(),
            value: "Bujeti".to_owned(),
            allow_exact: true,
        };
        let passport = FactForMapping {
            id: Uuid::new_v4(),
            key: "passport number".to_owned(),
            value: "private".to_owned(),
            allow_exact: true,
        };

        let facts = [employer, passport];
        let selected = select_relevant_facts(&field, &facts);
        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].key, "employer name");
    }
}
