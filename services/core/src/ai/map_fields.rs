use std::{collections::HashSet, time::Duration};

use reqwest::Client;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::field_fit::{
    identifier_family, is_business_name_field, is_honorific_title, is_previous_contact, normalize,
    value_fits_field,
};
use crate::{AppError, AppResult, config::Config, fields::extra_party_key};

const MIN_AI_CONFIDENCE: f64 = 0.7;
const GENERIC_TOKENS: [&str; 17] = [
    "a", "an", "and", "are", "current", "date", "for", "full", "in", "name", "number", "of", "or",
    "the", "to", "your", "address",
];

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
    #[serde(skip)]
    pub deterministic: bool,
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
        if matches!(field.kind.as_str(), "signature" | "checkbox" | "radio") {
            return Ok(None);
        }
        if let Some(mapping) = deterministic_match(field, facts)
            .filter(|mapping| value_fits_field(&field.label, &field.kind, &mapping.value))
        {
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
                        "content": "Map the field using only supplied profile facts. Return {\"mappings\":[]} when no fact semantically answers the question. Never invent, repurpose, or guess a value, especially identifiers. Generic words such as name, number, date, or address are not evidence. For a compound field, you may return a reviewable partial draft containing only directly relevant known parts; the explanation must state what is still missing. Confidence measures whether the included value is semantically correct, not whether the compound answer is complete. Return JSON with field_id, value, source_fact_ids, confidence from 0 to 1, and a short explanation."
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

        let is_grounded = valid_ai_mapping(&mapping, field, &allowed_ids);
        Ok(is_grounded.then_some(mapping))
    }
}

fn valid_ai_mapping(
    mapping: &GroundedMapping,
    field: &FieldForMapping,
    allowed_ids: &HashSet<Uuid>,
) -> bool {
    mapping.field_id == field.id
        && !mapping.source_fact_ids.is_empty()
        && mapping
            .source_fact_ids
            .iter()
            .all(|id| allowed_ids.contains(id))
        && (MIN_AI_CONFIDENCE..=1.0).contains(&mapping.confidence)
        && !mapping.deterministic
        && value_fits_field(&field.label, &field.kind, &mapping.value)
}

fn deterministic_match(
    field: &FieldForMapping,
    facts: &[FactForMapping],
) -> Option<GroundedMapping> {
    let field_key = normalize(&field.label);
    facts.iter().find_map(|fact| {
        let fact_key = normalize(&fact.key);
        let owner_name_alias = fact_key == "full name"
            && [
                "applicant name",
                "prospective tenant name",
                "declarant name",
                "name of signatory",
                "name of signatory 1",
                "signatory 1 name",
                "signatory name",
                "certification name",
            ]
            .contains(&field_key.as_str());
        let owner_given_name = fact_key == "full name"
            && field_key == "first name"
            && fact.value.split_whitespace().count() >= 1;
        let owner_surname = fact_key == "full name"
            && matches!(field_key.as_str(), "surname" | "last name")
            && fact.value.split_whitespace().count() >= 2;
        let owner_phone_alias = fact_key == "phone number"
            && [
                "telephone number",
                "telephone no",
                "mobile number",
                "mobile phone no",
                "mobile phone number",
                "whatsapp number",
                "whatsapp no",
                "whats app number",
                "whats app no",
            ]
            .contains(&field_key.as_str());
        let account_name_alias = fact_key == "account name"
            && ["name of account", "account name"].contains(&field_key.as_str());
        let account_number_alias = fact_key == "account number"
            && ["account number", "account no"].contains(&field_key.as_str());
        let identifier_alias = identifier_family(&field_key)
            .is_some_and(|family| identifier_family(&fact_key) == Some(family));
        let value = if owner_given_name {
            fact.value.split_whitespace().next().unwrap_or(&fact.value)
        } else if owner_surname {
            fact.value
                .split_whitespace()
                .next_back()
                .unwrap_or(&fact.value)
        } else {
            fact.value.as_str()
        };
        (fact.allow_exact
            && (field_key == fact_key
                || owner_name_alias
                || owner_given_name
                || owner_surname
                || owner_phone_alias
                || account_name_alias
                || account_number_alias
                || identifier_alias))
            .then(|| GroundedMapping {
                field_id: field.id,
                value: value.to_owned(),
                source_fact_ids: vec![fact.id],
                confidence: 1.0,
                explanation: "Exact confirmed profile fact".to_owned(),
                deterministic: true,
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
    let tokens = semantic_tokens(&context);

    facts
        .iter()
        .filter(|fact| {
            let fact_key = normalize(&fact.key);
            fact_is_compatible(&context, &fact_key)
                && !(is_previous_contact(&context) && !fact.allow_exact)
                && (identifier_family(&context).is_some()
                    || semantic_tokens(&fact_key)
                        .iter()
                        .any(|token| tokens.contains(token)))
                && value_fits_field(&field.label, &field.kind, &fact.value)
        })
        .take(6)
        .collect()
}

fn semantic_tokens(value: &str) -> HashSet<String> {
    value
        .split_whitespace()
        .filter(|token| token.len() > 2 && !GENERIC_TOKENS.contains(token))
        .map(|token| match token {
            "mobile" | "telephone" => "phone",
            "company" | "employed" | "employment" | "establishment" | "organisation"
            | "organization" | "workplace" => "employer",
            "job" | "position" | "profession" | "role" => "occupation",
            other => other,
        })
        .map(str::to_owned)
        .collect()
}

fn fact_is_compatible(field: &str, fact: &str) -> bool {
    if extra_party_key(field).is_some() && extra_party_key(fact) != extra_party_key(field) {
        return false;
    }
    if third_party_role(field).is_some_and(|role| third_party_role(fact) != Some(role)) {
        return false;
    }
    if is_honorific_title(field) {
        return is_honorific_title(fact);
    }
    if is_business_name_field(field) {
        return is_business_name_field(fact);
    }
    if is_previous_contact(field) {
        return is_previous_contact(fact);
    }
    match identifier_family(field) {
        Some(family) => identifier_family(fact) == Some(family),
        None => identifier_family(fact).is_none(),
    }
}

fn third_party_role(value: &str) -> Option<&'static str> {
    [
        ("guarantor", "guarantor"),
        ("spouse", "spouse"),
        ("next of kin", "next of kin"),
        ("emergency contact", "emergency contact"),
        ("former landlord", "former landlord"),
    ]
    .into_iter()
    .find_map(|(needle, role)| value.contains(needle).then_some(role))
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
        assert!(mapped.deterministic);
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

    #[test]
    fn generic_number_does_not_match_phone_to_an_identifier() {
        let field = FieldForMapping {
            id: Uuid::new_v4(),
            label: "Business registration certificate number".to_owned(),
            instructions: None,
            kind: "text".to_owned(),
        };
        let phone = FactForMapping {
            id: Uuid::new_v4(),
            key: "phone number".to_owned(),
            value: "+2348000000000".to_owned(),
            allow_exact: true,
        };
        assert!(select_relevant_facts(&field, &[phone]).is_empty());
    }

    #[test]
    fn telephone_is_compatible_with_phone() {
        let field = FieldForMapping {
            id: Uuid::new_v4(),
            label: "Telephone Number".to_owned(),
            instructions: None,
            kind: "phone".to_owned(),
        };
        let phone = FactForMapping {
            id: Uuid::new_v4(),
            key: "phone number".to_owned(),
            value: "+2348000000000".to_owned(),
            allow_exact: true,
        };
        let facts = [phone];
        assert_eq!(select_relevant_facts(&field, &facts).len(), 1);
        assert!(deterministic_match(&field, &facts).is_some());
    }

    #[test]
    fn semantic_job_wording_selects_occupation() {
        let field = FieldForMapping {
            id: Uuid::new_v4(),
            label: "Current position held".to_owned(),
            instructions: None,
            kind: "text".to_owned(),
        };
        let occupation = FactForMapping {
            id: Uuid::new_v4(),
            key: "occupation".to_owned(),
            value: "Backend Engineer".to_owned(),
            allow_exact: true,
        };
        assert_eq!(
            select_relevant_facts(&field, &[occupation])[0].key,
            "occupation"
        );
    }

    #[test]
    fn protected_identifiers_require_the_same_fact_type() {
        let field = FieldForMapping {
            id: Uuid::new_v4(),
            label: "Business registration certificate number".to_owned(),
            instructions: None,
            kind: "text".to_owned(),
        };
        let employer = FactForMapping {
            id: Uuid::new_v4(),
            key: "employer name".to_owned(),
            value: "Bujeti".to_owned(),
            allow_exact: true,
        };
        assert!(select_relevant_facts(&field, &[employer]).is_empty());
    }

    #[test]
    fn owner_details_do_not_fill_guarantor_fields() {
        let field = FieldForMapping {
            id: Uuid::new_v4(),
            label: "Guarantor 1 Telephone Number".to_owned(),
            instructions: None,
            kind: "phone".to_owned(),
        };
        let phone = FactForMapping {
            id: Uuid::new_v4(),
            key: "phone number".to_owned(),
            value: "+2348000000000".to_owned(),
            allow_exact: true,
        };
        assert!(select_relevant_facts(&field, &[phone]).is_empty());
    }

    #[test]
    fn full_name_only_uses_safe_owner_aliases() {
        let fact = FactForMapping {
            id: Uuid::new_v4(),
            key: "full name".to_owned(),
            value: "Victor Jonah".to_owned(),
            allow_exact: true,
        };
        let owner = FieldForMapping {
            id: Uuid::new_v4(),
            label: "Prospective tenant name".to_owned(),
            instructions: None,
            kind: "text".to_owned(),
        };
        let guarantor = FieldForMapping {
            id: Uuid::new_v4(),
            label: "Guarantor full name".to_owned(),
            instructions: None,
            kind: "text".to_owned(),
        };
        assert!(deterministic_match(&owner, std::slice::from_ref(&fact)).is_some());
        assert!(deterministic_match(&guarantor, &[fact]).is_none());
    }

    #[test]
    fn first_signatory_can_use_owner_name() {
        let fact = FactForMapping {
            id: Uuid::new_v4(),
            key: "full name".to_owned(),
            value: "Victor Jonah".to_owned(),
            allow_exact: true,
        };
        let first = FieldForMapping {
            id: Uuid::new_v4(),
            label: "Name of Signatory 1".to_owned(),
            instructions: None,
            kind: "text".to_owned(),
        };
        let second = FieldForMapping {
            id: Uuid::new_v4(),
            label: "Name of Signatory 2".to_owned(),
            instructions: None,
            kind: "text".to_owned(),
        };
        assert!(deterministic_match(&first, std::slice::from_ref(&fact)).is_some());
        assert!(deterministic_match(&second, std::slice::from_ref(&fact)).is_none());
        assert!(select_relevant_facts(&second, std::slice::from_ref(&fact)).is_empty());
    }

    #[test]
    fn later_signatory_phone_does_not_use_owner_phone() {
        let field = FieldForMapping {
            id: Uuid::new_v4(),
            label: "Mobile Phone No 2".to_owned(),
            instructions: None,
            kind: "phone".to_owned(),
        };
        let phone = FactForMapping {
            id: Uuid::new_v4(),
            key: "phone number".to_owned(),
            value: "+2348000000000".to_owned(),
            allow_exact: true,
        };
        assert!(select_relevant_facts(&field, &[phone]).is_empty());
    }

    #[test]
    fn signature_fields_are_not_auto_filled() {
        let field = FieldForMapping {
            id: Uuid::new_v4(),
            label: "Specimen Signature 1".to_owned(),
            instructions: None,
            kind: "signature".to_owned(),
        };
        let name = FactForMapping {
            id: Uuid::new_v4(),
            key: "full name".to_owned(),
            value: "Victor Jonah".to_owned(),
            allow_exact: true,
        };
        assert!(deterministic_match(&field, &[name]).is_none());
    }

    #[test]
    fn rejects_low_confidence_ai_answers() {
        let field_id = Uuid::new_v4();
        let fact_id = Uuid::new_v4();
        let mapping = GroundedMapping {
            field_id,
            value: "Uncertain".to_owned(),
            source_fact_ids: vec![fact_id],
            confidence: 0.1,
            explanation: "Weak overlap".to_owned(),
            deterministic: false,
        };
        assert!(!valid_ai_mapping(
            &mapping,
            &FieldForMapping {
                id: field_id,
                label: "Employer".to_owned(),
                instructions: None,
                kind: "text".to_owned(),
            },
            &HashSet::from([fact_id])
        ));
    }

    #[test]
    fn bvn_and_tin_do_not_take_name_or_phone() {
        let name = FactForMapping {
            id: Uuid::new_v4(),
            key: "full name".to_owned(),
            value: "Victor Jonah".to_owned(),
            allow_exact: true,
        };
        let phone = FactForMapping {
            id: Uuid::new_v4(),
            key: "phone number".to_owned(),
            value: "+2348086249721".to_owned(),
            allow_exact: true,
        };
        let memory_name = FactForMapping {
            id: Uuid::new_v4(),
            key: "BVN".to_owned(),
            value: "Victor Jonah".to_owned(),
            allow_exact: false,
        };
        let memory_phone = FactForMapping {
            id: Uuid::new_v4(),
            key: "TIN".to_owned(),
            value: "MOBILE: 08086249721".to_owned(),
            allow_exact: false,
        };
        let bvn = FieldForMapping {
            id: Uuid::new_v4(),
            label: "Bank Verification Number".to_owned(),
            instructions: None,
            kind: "text".to_owned(),
        };
        let tin = FieldForMapping {
            id: Uuid::new_v4(),
            label: "TIN".to_owned(),
            instructions: None,
            kind: "text".to_owned(),
        };
        let facts = [name, phone, memory_name, memory_phone];
        assert!(select_relevant_facts(&bvn, &facts).is_empty());
        assert!(select_relevant_facts(&tin, &facts).is_empty());
        assert!(deterministic_match(&bvn, &facts).is_none());
        assert!(deterministic_match(&tin, &facts).is_none());

        let real_bvn = FactForMapping {
            id: Uuid::new_v4(),
            key: "bvn".to_owned(),
            value: "22123456789".to_owned(),
            allow_exact: true,
        };
        let mapped = deterministic_match(&bvn, std::slice::from_ref(&real_bvn)).expect("bvn");
        assert_eq!(mapped.value, "22123456789");
        assert_eq!(select_relevant_facts(&bvn, &[real_bvn]).len(), 1);
    }

    #[test]
    fn title_and_business_name_stay_empty_without_matching_facts() {
        let name = FactForMapping {
            id: Uuid::new_v4(),
            key: "full name".to_owned(),
            value: "Victor Jonah".to_owned(),
            allow_exact: true,
        };
        let employer = FactForMapping {
            id: Uuid::new_v4(),
            key: "employer".to_owned(),
            value: "Employed at Bujeti".to_owned(),
            allow_exact: true,
        };
        let title = FieldForMapping {
            id: Uuid::new_v4(),
            label: "Title".to_owned(),
            instructions: None,
            kind: "text".to_owned(),
        };
        let business = FieldForMapping {
            id: Uuid::new_v4(),
            label: "Business Name".to_owned(),
            instructions: None,
            kind: "text".to_owned(),
        };
        let facts = [name, employer];
        assert!(select_relevant_facts(&title, &facts).is_empty());
        assert!(select_relevant_facts(&business, &facts).is_empty());
    }

    #[test]
    fn old_phone_does_not_use_current_phone() {
        let field = FieldForMapping {
            id: Uuid::new_v4(),
            label: "Old Phone No".to_owned(),
            instructions: None,
            kind: "phone".to_owned(),
        };
        let phone = FactForMapping {
            id: Uuid::new_v4(),
            key: "phone number".to_owned(),
            value: "+2348086249721".to_owned(),
            allow_exact: true,
        };
        assert!(select_relevant_facts(&field, std::slice::from_ref(&phone)).is_empty());
        let memory = FactForMapping {
            id: Uuid::new_v4(),
            key: "Old Phone No".to_owned(),
            value: "08086249721".to_owned(),
            allow_exact: false,
        };
        assert!(select_relevant_facts(&field, &[phone, memory]).is_empty());
    }

    #[test]
    fn first_and_last_name_split_from_full_name() {
        let fact = FactForMapping {
            id: Uuid::new_v4(),
            key: "full name".to_owned(),
            value: "Victor Jonah".to_owned(),
            allow_exact: true,
        };
        let first = FieldForMapping {
            id: Uuid::new_v4(),
            label: "First Name".to_owned(),
            instructions: None,
            kind: "text".to_owned(),
        };
        let surname = FieldForMapping {
            id: Uuid::new_v4(),
            label: "Surname".to_owned(),
            instructions: None,
            kind: "text".to_owned(),
        };
        assert_eq!(
            deterministic_match(&first, std::slice::from_ref(&fact))
                .expect("first")
                .value,
            "Victor"
        );
        assert_eq!(
            deterministic_match(&surname, std::slice::from_ref(&fact))
                .expect("surname")
                .value,
            "Jonah"
        );
    }

    #[test]
    fn rejects_ai_name_on_an_identifier() {
        let field = FieldForMapping {
            id: Uuid::new_v4(),
            label: "BVN".to_owned(),
            instructions: None,
            kind: "text".to_owned(),
        };
        let fact_id = Uuid::new_v4();
        let mapping = GroundedMapping {
            field_id: field.id,
            value: "Victor Jonah".to_owned(),
            source_fact_ids: vec![fact_id],
            confidence: 0.9,
            explanation: "Memory overlap".to_owned(),
            deterministic: false,
        };
        assert!(!valid_ai_mapping(
            &mapping,
            &field,
            &HashSet::from([fact_id])
        ));
    }
}
