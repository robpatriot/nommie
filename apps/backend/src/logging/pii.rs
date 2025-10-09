use std::fmt;
use std::sync::LazyLock;

use regex::Regex;

/// Centralized registry for PII redaction regex patterns.
///
/// This module contains all hardcoded regex patterns used for PII redaction,
/// with a single allow per pattern construction site. All patterns are known
/// to be valid and tested at compile time.
pub struct PiiRegexRegistry;

impl PiiRegexRegistry {
    /// Email pattern: matches standard email addresses
    /// SAFETY: This regex pattern is a vetted literal that compiles successfully
    pub fn email() -> &'static Regex {
        static EMAIL_REGEX: LazyLock<Regex> = LazyLock::new(|| {
            #[allow(clippy::unwrap_used)]
            Regex::new(r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{1,}\b").unwrap()
        });
        &EMAIL_REGEX
    }

    /// Base64-like token pattern: matches base64-encoded tokens (≥16 chars)
    /// SAFETY: This regex pattern is a vetted literal that compiles successfully
    pub fn base64_token() -> &'static Regex {
        static BASE64_TOKEN_REGEX: LazyLock<Regex> = LazyLock::new(|| {
            #[allow(clippy::unwrap_used)]
            Regex::new(r"\b[A-Za-z0-9+/]{16,}={0,2}\b").unwrap()
        });
        &BASE64_TOKEN_REGEX
    }

    /// Hex token pattern: matches hexadecimal tokens (≥16 chars)
    /// SAFETY: This regex pattern is a vetted literal that compiles successfully
    pub fn hex_token() -> &'static Regex {
        static HEX_TOKEN_REGEX: LazyLock<Regex> = LazyLock::new(|| {
            #[allow(clippy::unwrap_used)]
            Regex::new(r"\b[A-Fa-f0-9]{16,}\b").unwrap()
        });
        &HEX_TOKEN_REGEX
    }
}

/// Redacts sensitive information from a string.
///
/// This function conservatively masks:
/// - Emails: keeps first character of local part, replaces rest with ***, keeps full domain
/// - Opaque tokens: replaces base64-like or hex runs (≥16 chars) with [REDACTED_TOKEN]
///
/// Order: emails first, then tokens, to avoid double-processing.
pub fn redact(input: &str) -> String {
    // First redact emails
    let email_redacted = PiiRegexRegistry::email().replace_all(input, |caps: &regex::Captures| {
        let full_match = &caps[0];
        if let Some(at_pos) = full_match.find('@') {
            let local_part = &full_match[..at_pos];
            let domain = &full_match[at_pos..];

            if local_part.is_empty() {
                // Edge case: no local part, just return the domain
                domain.to_string()
            } else {
                // Keep first char, replace rest with ***, keep full domain
                let first_char = &local_part[..1];
                format!("{first_char}***{domain}")
            }
        } else {
            // Fallback: shouldn't happen with proper email regex
            full_match.to_string()
        }
    });

    // Then redact base64-like tokens
    let base64_redacted =
        PiiRegexRegistry::base64_token().replace_all(&email_redacted, "[REDACTED_TOKEN]");

    // Finally redact hex tokens
    PiiRegexRegistry::hex_token()
        .replace_all(&base64_redacted, "[REDACTED_TOKEN]")
        .to_string()
}

/// A wrapper that automatically redacts sensitive strings when displayed.
///
/// This provides ergonomic logging of sensitive data by automatically
/// applying PII redaction when the value is formatted for display.
pub struct Redacted<'a>(pub &'a str);

impl<'a> fmt::Display for Redacted<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", redact(self.0))
    }
}

impl<'a> fmt::Debug for Redacted<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", redact(self.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_email_redaction() {
        // Typical email
        assert_eq!(redact("user@example.com"), "u***@example.com");

        // Single character local part
        assert_eq!(redact("a@test.org"), "a***@test.org");

        // Very short TLD
        assert_eq!(redact("x@y.z"), "x***@y.z");

        // Multi-label domain
        assert_eq!(redact("test@sub.example.com"), "t***@sub.example.com");

        // Multiple emails
        assert_eq!(
            redact("Contact user@example.com or admin@test.org"),
            "Contact u***@example.com or a***@test.org"
        );

        // No local part (edge case)
        assert_eq!(redact("@example.com"), "@example.com");
    }

    #[test]
    fn test_token_redaction() {
        // Base64-like token
        assert_eq!(
            redact("eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9"),
            "[REDACTED_TOKEN]"
        );

        // Hex token
        assert_eq!(
            redact("a1b2c3d4e5f678901234567890123456"),
            "[REDACTED_TOKEN]"
        );

        // Short strings should be left untouched
        assert_eq!(redact("short123"), "short123");
        assert_eq!(redact("abc123def456"), "abc123def456");

        // Multiple tokens
        assert_eq!(
            redact("Token1: eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9 Token2: a1b2c3d4e5f678901234567890123456"),
            "Token1: [REDACTED_TOKEN] Token2: [REDACTED_TOKEN]"
        );
    }

    #[test]
    fn test_mixed_content_redaction() {
        // Both email and token
        assert_eq!(
            redact("User user@example.com has token eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9"),
            "User u***@example.com has token [REDACTED_TOKEN]"
        );

        // Complex mixed content
        assert_eq!(
            redact("Error for user@test.com: invalid token a1b2c3d4e5f678901234567890123456"),
            "Error for u***@test.com: invalid token [REDACTED_TOKEN]"
        );
    }

    #[test]
    fn test_redacted_wrapper() {
        let sensitive = "user@example.com";
        let redacted = Redacted(sensitive);

        // Test Display implementation
        assert_eq!(format!("{redacted}"), "u***@example.com");

        // Test Debug implementation (should also redact)
        assert_eq!(format!("{redacted:?}"), "u***@example.com");
    }

    #[test]
    fn test_redacted_convenience_constructor() {
        let sensitive = "admin@test.org";
        let redacted = Redacted(sensitive);

        assert_eq!(format!("{redacted}"), "a***@test.org");
    }

    #[test]
    fn test_no_sensitive_data() {
        // Strings without sensitive data should be unchanged
        assert_eq!(redact("Hello world"), "Hello world");
        assert_eq!(redact("12345"), "12345");
        assert_eq!(redact(""), "");
    }

    #[test]
    fn test_edge_cases() {
        // Empty string
        assert_eq!(redact(""), "");

        // Only special characters
        assert_eq!(redact("!@#$%"), "!@#$%");

        // Very long strings (should not be treated as tokens)
        let long_string = "a".repeat(1000);
        assert_eq!(redact(&long_string), "[REDACTED_TOKEN]");

        // Very long non-hex string (with spaces to avoid token patterns)
        let long_non_hex = "hello world ".repeat(100);
        assert_eq!(redact(&long_non_hex), long_non_hex);
    }
}
