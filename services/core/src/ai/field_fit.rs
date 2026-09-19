pub(crate) fn identifier_family(text: &str) -> Option<&'static str> {
    const PHRASES: &[(&str, &str)] = &[
        ("bank verification", "bvn"),
        ("tax identification", "tax"),
        ("tax id", "tax"),
        ("account number", "account"),
        ("account no", "account"),
        ("national identification", "nin"),
        ("national id", "nin"),
        ("certificate number", "certificate"),
        ("registration number", "registration"),
        ("registration certificate", "registration"),
    ];
    for (phrase, family) in PHRASES {
        if text.contains(phrase) {
            return Some(family);
        }
    }

    let tokens: Vec<_> = text.split_whitespace().collect();
    if tokens.contains(&"bvn") {
        return Some("bvn");
    }
    if tokens.contains(&"tin") {
        return Some("tax");
    }
    if tokens.contains(&"nin") {
        return Some("nin");
    }
    if tokens.contains(&"passport") {
        return Some("passport");
    }
    if tokens
        .iter()
        .any(|token| *token == "licence" || *token == "license")
    {
        return Some("licence");
    }
    None
}

pub(crate) fn is_honorific_title(field: &str) -> bool {
    matches!(field, "title" | "honorific" | "salutation")
        || (field.starts_with("title ")
            && (field.contains("mr") || field.contains("mrs") || field.contains("miss")))
}

pub(crate) fn is_business_name_field(field: &str) -> bool {
    field.contains("business name") || field.contains("trade name") || field == "business"
}

pub(crate) fn is_previous_contact(field: &str) -> bool {
    (field.contains("old") || field.contains("previous") || field.contains("former"))
        && (field.contains("phone")
            || field.contains("mobile")
            || field.contains("email")
            || field.contains("address"))
}

pub(crate) fn value_fits_field(label: &str, kind: &str, value: &str) -> bool {
    let trimmed = value.trim();
    if trimmed.is_empty() || looks_like_labeled_dump(trimmed) {
        return false;
    }
    let label = normalize(label);
    if is_honorific_title(&label) {
        return looks_like_honorific(trimmed);
    }
    if identifier_family(&label).is_some() {
        return looks_like_identifier(trimmed);
    }
    if is_business_name_field(&label) {
        return !looks_like_employment_sentence(trimmed) && !looks_like_phone(trimmed);
    }
    match kind {
        "phone" => looks_like_phone(trimmed),
        "email" => looks_like_email(trimmed),
        _ => true,
    }
}

pub(crate) fn normalize(value: &str) -> String {
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

fn looks_like_identifier(value: &str) -> bool {
    if looks_like_person_name(value) || looks_like_phone(value) || looks_like_email(value) {
        return false;
    }
    let compact: String = value.chars().filter(char::is_ascii_alphanumeric).collect();
    if compact.len() < 6 {
        return false;
    }
    let digits = compact
        .chars()
        .filter(|character| character.is_ascii_digit())
        .count();
    digits * 2 >= compact.len()
}

fn looks_like_honorific(value: &str) -> bool {
    matches!(
        normalize(value).as_str(),
        "mr" | "mrs"
            | "miss"
            | "ms"
            | "dr"
            | "prof"
            | "professor"
            | "chief"
            | "engr"
            | "engineer"
            | "alhaji"
            | "alhaja"
            | "pastor"
            | "rev"
            | "reverend"
            | "hon"
            | "sir"
            | "lady"
            | "barr"
            | "barrister"
    )
}

fn looks_like_person_name(value: &str) -> bool {
    let words: Vec<_> = normalize(value)
        .split_whitespace()
        .map(str::to_owned)
        .collect();
    (2..=4).contains(&words.len())
        && words.iter().all(|word| {
            word.chars()
                .all(|character| character.is_ascii_alphabetic())
                && !matches!(
                    word.as_str(),
                    "employed"
                        | "occupation"
                        | "mobile"
                        | "current"
                        | "address"
                        | "at"
                        | "of"
                        | "the"
                )
        })
}

fn looks_like_phone(value: &str) -> bool {
    let normalized = normalize(value);
    if normalized.contains("mobile") || normalized.contains("phone") || normalized.contains("tel") {
        return true;
    }
    let digits: String = value
        .chars()
        .filter(|character| character.is_ascii_digit())
        .collect();
    let trimmed = value.trim();
    if trimmed.starts_with('+') && digits.len() >= 10 {
        return true;
    }
    digits.starts_with('0') && digits.len() == 11
}

fn looks_like_email(value: &str) -> bool {
    let trimmed = value.trim();
    trimmed.contains('@') && trimmed.contains('.')
}

fn looks_like_employment_sentence(value: &str) -> bool {
    let normalized = normalize(value);
    normalized.contains("employed")
        || normalized.contains("occupation")
        || normalized.contains("works at")
}

fn looks_like_labeled_dump(value: &str) -> bool {
    let Some((label, rest)) = value.split_once(':') else {
        return false;
    };
    let label = label.trim();
    if rest.trim().is_empty() || label.chars().all(|character| character.is_ascii_digit()) {
        return false;
    }
    let label = normalize(label);
    let words = label.split_whitespace().count();
    (1..=5).contains(&words)
        && label
            .chars()
            .any(|character| character.is_ascii_alphabetic())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bank_verification_aliases_to_bvn() {
        assert_eq!(identifier_family("bank verification number"), Some("bvn"));
        assert_eq!(identifier_family("bvn"), Some("bvn"));
        assert_eq!(identifier_family("tin"), Some("tax"));
        assert_eq!(identifier_family("full name"), None);
        assert_eq!(identifier_family("certification name"), None);
        assert_eq!(identifier_family("name of account"), None);
    }

    #[test]
    fn identifier_values_reject_names_and_phones() {
        assert!(value_fits_field(
            "Bank Verification Number",
            "text",
            "22123456789"
        ));
        assert!(!value_fits_field(
            "Bank Verification Number",
            "text",
            "Victor Jonah"
        ));
        assert!(!value_fits_field(
            "Bank Verification Number",
            "text",
            "08086249721"
        ));
        assert!(!value_fits_field("TIN", "text", "MOBILE: 08086249721"));
        assert!(value_fits_field("TIN", "text", "12345678-0001"));
    }

    #[test]
    fn title_only_accepts_honorifics() {
        assert!(value_fits_field("Title", "text", "Mr"));
        assert!(!value_fits_field("Title", "text", "Victor Jonah"));
    }

    #[test]
    fn business_name_rejects_employment_copy() {
        assert!(!value_fits_field(
            "Business Name",
            "text",
            "Employed at Bujeti"
        ));
        assert!(value_fits_field("Business Name", "text", "Bujeti"));
        assert!(!value_fits_field(
            "Certification Name",
            "text",
            "Date Of Birth: 1997-05-05"
        ));
    }
}
