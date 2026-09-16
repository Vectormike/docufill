use uuid::Uuid;

pub fn secure_token() -> String {
    format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple())
}

pub fn verification_code() -> String {
    let bytes = *Uuid::new_v4().as_bytes();
    let value = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) % 1_000_000;
    format!("{value:06}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verification_codes_are_six_digits() {
        let code = verification_code();
        assert_eq!(code.len(), 6);
        assert!(code.chars().all(|character| character.is_ascii_digit()));
    }

    #[test]
    fn share_tokens_have_256_bits_of_uuid_material() {
        assert_eq!(secure_token().len(), 64);
    }
}
