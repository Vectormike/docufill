pub fn requires_owner_answer(_label: &str, _kind: &str) -> bool {
    false
}

pub fn is_extra_party(value: &str) -> bool {
    extra_party_key(value).is_some()
}

pub fn extra_party_key(value: &str) -> Option<String> {
    let normalized = normalize(value);
    if normalized.contains("additional signatory") || normalized.contains("other signator") {
        return Some("additional".to_owned());
    }
    if let Some(instance) = numbered_instance(&normalized) {
        return (instance != "1").then(|| instance.to_owned());
    }
    signatory_letter(&normalized)
        .filter(|letter| *letter != "a")
        .map(str::to_owned)
}

pub fn numbered_instance(value: &str) -> Option<&str> {
    value.split_whitespace().rev().find(|token| {
        !token.is_empty() && token.chars().all(|character| character.is_ascii_digit())
    })
}

fn signatory_letter(value: &str) -> Option<&'static str> {
    ["signatory a", "signatory b", "signatory c", "signatory d"]
        .into_iter()
        .find(|needle| value.contains(needle))
        .and_then(|needle| needle.rsplit_once(' ').map(|(_, letter)| letter))
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
    fn owner_answers_are_optional() {
        assert!(!requires_owner_answer("First Name", "text"));
        assert!(!requires_owner_answer("Name of Signatory 1", "text"));
        assert!(!requires_owner_answer("Name of Signatory 2", "text"));
        assert!(!requires_owner_answer("Date of Birth", "date"));
        assert!(!requires_owner_answer("Male", "checkbox"));
    }

    #[test]
    fn extra_party_facts_must_match_the_same_party() {
        assert_eq!(extra_party_key("Name of Signatory 2"), Some("2".to_owned()));
        assert_eq!(extra_party_key("Signatory B name"), Some("b".to_owned()));
        assert_eq!(extra_party_key("full name"), None);
        assert_eq!(extra_party_key("Name of Signatory 1"), None);
    }
}
