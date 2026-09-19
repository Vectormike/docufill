pub(crate) fn is_sensitive_label(label: &str) -> bool {
    let normalized = label.to_ascii_lowercase();
    [
        "signature",
        "password",
        "passport",
        "nin",
        "bvn",
        "bank verification",
        "account number",
        "bank account",
        "tax id",
        "tax identification",
        "tin",
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
        assert!(is_sensitive_label("TIN"));
        assert!(is_sensitive_label("Bank Verification Number"));
        assert!(!is_sensitive_label("Current employer"));
    }
}
