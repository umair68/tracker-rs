/// Verify API key using constant-time comparison to prevent timing attacks
/// 
/// This function compares two strings in constant time to prevent timing attacks
/// that could be used to guess the API key character by character.
pub fn verify_api_key(provided: &str, expected: &str) -> bool {
    provided.as_bytes().len() == expected.as_bytes().len()
        && provided
            .as_bytes()
            .iter()
            .zip(expected.as_bytes().iter())
            .fold(0u8, |acc, (a, b)| acc | (a ^ b))
            == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verify_api_key_valid() {
        assert!(verify_api_key("test-key", "test-key"));
    }

    #[test]
    fn test_verify_api_key_invalid() {
        assert!(!verify_api_key("wrong-key", "test-key"));
    }

    #[test]
    fn test_verify_api_key_different_length() {
        assert!(!verify_api_key("short", "much-longer-key"));
    }

    #[test]
    fn test_verify_api_key_empty() {
        assert!(verify_api_key("", ""));
    }

    #[test]
    fn test_verify_api_key_case_sensitive() {
        assert!(!verify_api_key("Test-Key", "test-key"));
    }

    #[test]
    fn test_verify_api_key_special_chars() {
        assert!(verify_api_key("key-with-$pecial!", "key-with-$pecial!"));
    }
}
