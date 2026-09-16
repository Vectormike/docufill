use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryKind {
    ConfirmedFact,
    ConfirmedAnswer,
    ApprovedExcerpt,
}

#[derive(Debug, Clone, Serialize)]
pub struct MemoryItem {
    pub owner_id: Uuid,
    pub source_id: Uuid,
    pub source_document_id: Option<Uuid>,
    pub kind: MemoryKind,
    pub key: String,
    pub value: String,
    pub confirmed_at: DateTime<Utc>,
}

pub fn retrieve_for_field<'a>(
    owner_id: Uuid,
    query: &str,
    items: &'a [MemoryItem],
    limit: usize,
) -> Vec<&'a MemoryItem> {
    let query_tokens = tokens(query);
    let mut matches: Vec<_> = items
        .iter()
        .filter(|item| item.owner_id == owner_id)
        .filter_map(|item| {
            let key_tokens = tokens(&item.key);
            let overlap = key_tokens
                .iter()
                .filter(|token| query_tokens.contains(token))
                .count();
            (overlap > 0).then_some((item, overlap))
        })
        .collect();

    matches.sort_by(|(left, left_score), (right, right_score)| {
        left.kind
            .cmp(&right.kind)
            .then_with(|| right_score.cmp(left_score))
            .then_with(|| right.confirmed_at.cmp(&left.confirmed_at))
    });
    matches
        .into_iter()
        .take(limit.min(20))
        .map(|(item, _)| item)
        .collect()
}

fn tokens(value: &str) -> Vec<String> {
    value
        .split(|character: char| !character.is_alphanumeric())
        .filter(|token| token.len() > 2)
        .map(str::to_lowercase)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn never_returns_another_owners_memory() {
        let owner_id = Uuid::new_v4();
        let other_owner = Uuid::new_v4();
        let now = Utc::now();
        let items = vec![
            MemoryItem {
                owner_id,
                source_id: Uuid::new_v4(),
                source_document_id: None,
                kind: MemoryKind::ConfirmedFact,
                key: "current employer".to_owned(),
                value: "Allowed".to_owned(),
                confirmed_at: now,
            },
            MemoryItem {
                owner_id: other_owner,
                source_id: Uuid::new_v4(),
                source_document_id: None,
                kind: MemoryKind::ConfirmedFact,
                key: "current employer".to_owned(),
                value: "Leaked".to_owned(),
                confirmed_at: now,
            },
        ];

        let result = retrieve_for_field(owner_id, "Employer name", &items, 5);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].value, "Allowed");
    }
}
