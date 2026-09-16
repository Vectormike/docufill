pub(crate) fn is_sensitive_label(label: &str) -> bool {
    let normalized = label.to_ascii_lowercase();
    [
        "signature",
        "password",
        "passport",
        "nin",
        "bvn",
        "account number",
        "bank account",
        "tax id",
    ]
    .iter()
    .any(|needle| normalized.contains(needle))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn excludes_sensitive_identifiers_from_memory() {
        assert!(is_sensitive_label("NIN number"));
        assert!(is_sensitive_label("Bank account number"));
        assert!(!is_sensitive_label("Current employer"));
    }
}
