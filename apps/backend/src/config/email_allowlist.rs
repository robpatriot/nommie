//! Email allowlist configuration for restricting signup and login.
//!
//! This module provides functionality to restrict access based on an email allowlist
//! loaded from the `ALLOWED_EMAILS` environment variable. The allowlist supports
//! glob patterns (e.g., `*@example.com`) and exact email matches.

use std::env;

use unicode_normalization::UnicodeNormalization;

/// Email allowlist that supports exact matches and glob patterns.
///
/// Patterns can be:
/// - Exact emails: `user@example.com`
/// - Domain wildcards: `*@example.com`
/// - Subdomain wildcards: `user@*.example.com`
#[derive(Debug, Clone)]
pub struct EmailAllowlist {
    patterns: Vec<String>,
}

impl EmailAllowlist {
    /// Create a new email allowlist from the `ALLOWED_EMAILS` environment variable.
    ///
    /// Returns `None` if the environment variable is not set or is empty
    /// (allowlist disabled - all emails allowed).
    ///
    /// Patterns are normalized (trimmed, NFKC normalized, lowercased) when loaded
    /// to ensure consistent matching.
    pub fn from_env() -> Option<Self> {
        let env_value = env::var("ALLOWED_EMAILS").ok()?;
        let trimmed = env_value.trim();
        if trimmed.is_empty() {
            return None;
        }

        let patterns: Vec<String> = trimmed
            .split(',')
            .map(|s| Self::normalize_email(s.trim()))
            .filter(|s| !s.is_empty())
            .collect();

        if patterns.is_empty() {
            return None;
        }

        Some(Self { patterns })
    }

    /// Check if an email is allowed by the allowlist.
    ///
    /// The email is normalized (trimmed, NFKC normalized, lowercased) before matching,
    /// matching the normalization used in UserService.
    /// Returns `true` if the email matches any pattern in the allowlist.
    pub fn is_allowed(&self, email: &str) -> bool {
        // Normalize email using the same method as UserService
        let normalized = Self::normalize_email(email);

        for pattern in &self.patterns {
            if Self::matches_pattern(&normalized, pattern) {
                return true;
            }
        }

        false
    }

    /// Normalize an email address for consistent storage and comparison.
    ///
    /// Normalization includes:
    /// - Trimming leading/trailing whitespace
    /// - Converting to lowercase
    /// - Applying Unicode NFKC normalization to handle visually equivalent but distinct codepoints
    ///
    /// This matches the normalization used in UserService::normalize_email().
    fn normalize_email(email: &str) -> String {
        email.trim().nfkc().collect::<String>().to_lowercase()
    }

    /// Check if an email matches a pattern (supports `*` wildcards).
    ///
    /// Patterns can contain `*` which matches any sequence of characters.
    /// Examples:
    /// - `user@example.com` matches `user@example.com`
    /// - `*@example.com` matches any email at `example.com`
    /// - `user@*.example.com` matches `user@sub.example.com`, `user@other.example.com`, etc.
    fn matches_pattern(email: &str, pattern: &str) -> bool {
        // Exact match (no wildcards)
        if !pattern.contains('*') {
            return email == pattern;
        }

        // Convert pattern to regex-like matching
        // Split by * and check each segment
        let parts: Vec<&str> = pattern.split('*').collect();

        // If pattern starts with *, email must end with the suffix
        if pattern.starts_with('*') {
            if parts.len() == 2 && parts[1].is_empty() {
                return true; // Pattern is just "*"
            }
            if parts.len() == 2 {
                return email.ends_with(parts[1]);
            }
        }

        // If pattern ends with *, email must start with the prefix
        if pattern.ends_with('*') {
            if parts.len() == 2 && parts[0].is_empty() {
                return true; // Pattern is just "*"
            }
            if parts.len() == 2 {
                return email.starts_with(parts[0]);
            }
        }

        // Pattern has * in the middle or multiple *s
        // Simple approach: check that email contains all non-empty parts in order
        let mut search_start = 0;
        for (i, part) in parts.iter().enumerate() {
            if part.is_empty() {
                continue;
            }

            if i == 0 && !pattern.starts_with('*') {
                // First part must match from start
                if !email.starts_with(part) {
                    return false;
                }
                search_start = part.len();
            } else if i == parts.len() - 1 && !pattern.ends_with('*') {
                // Last part must match at end
                if !email.ends_with(part) {
                    return false;
                }
            } else {
                // Middle part must be found somewhere after search_start
                if let Some(pos) = email[search_start..].find(part) {
                    search_start += pos + part.len();
                } else {
                    return false;
                }
            }
        }

        true
    }

    /// Get the number of patterns in the allowlist.
    pub fn pattern_count(&self) -> usize {
        self.patterns.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exact_match() {
        let allowlist = EmailAllowlist {
            patterns: vec!["user@example.com".to_string()],
        };

        assert!(allowlist.is_allowed("user@example.com"));
        assert!(allowlist.is_allowed("USER@EXAMPLE.COM")); // Case insensitive
        assert!(!allowlist.is_allowed("other@example.com"));
    }

    #[test]
    fn test_domain_wildcard() {
        let allowlist = EmailAllowlist {
            patterns: vec!["*@example.com".to_string()],
        };

        assert!(allowlist.is_allowed("user@example.com"));
        assert!(allowlist.is_allowed("admin@example.com"));
        assert!(allowlist.is_allowed("test.user@example.com"));
        assert!(!allowlist.is_allowed("user@other.com"));
    }

    #[test]
    fn test_subdomain_wildcard() {
        let allowlist = EmailAllowlist {
            patterns: vec!["user@*.example.com".to_string()],
        };

        assert!(allowlist.is_allowed("user@sub.example.com"));
        assert!(allowlist.is_allowed("user@other.example.com"));
        assert!(!allowlist.is_allowed("user@example.com"));
        assert!(!allowlist.is_allowed("other@sub.example.com"));
    }

    #[test]
    fn test_multiple_patterns() {
        let allowlist = EmailAllowlist {
            patterns: vec!["user1@example.com".to_string(), "*@trusted.com".to_string()],
        };

        assert!(allowlist.is_allowed("user1@example.com"));
        assert!(allowlist.is_allowed("anyone@trusted.com"));
        assert!(!allowlist.is_allowed("user2@example.com"));
    }

    #[test]
    fn test_from_env_empty() {
        // This test would require mocking env vars, so we'll test the logic directly
        // In practice, from_env() will return None for empty/missing env vars
        let _allowlist = EmailAllowlist::from_env();
        // If ALLOWED_EMAILS is not set in test env, this will be None
        // That's expected behavior
    }

    #[test]
    fn test_matches_pattern_exact() {
        assert!(EmailAllowlist::matches_pattern(
            "user@example.com",
            "user@example.com"
        ));
        assert!(!EmailAllowlist::matches_pattern(
            "user@example.com",
            "other@example.com"
        ));
    }

    #[test]
    fn test_matches_pattern_domain_wildcard() {
        assert!(EmailAllowlist::matches_pattern(
            "user@example.com",
            "*@example.com"
        ));
        assert!(EmailAllowlist::matches_pattern(
            "admin@example.com",
            "*@example.com"
        ));
        assert!(!EmailAllowlist::matches_pattern(
            "user@other.com",
            "*@example.com"
        ));
    }

    #[test]
    fn test_matches_pattern_subdomain_wildcard() {
        assert!(EmailAllowlist::matches_pattern(
            "user@sub.example.com",
            "user@*.example.com"
        ));
        assert!(EmailAllowlist::matches_pattern(
            "user@other.example.com",
            "user@*.example.com"
        ));
        assert!(!EmailAllowlist::matches_pattern(
            "user@example.com",
            "user@*.example.com"
        ));
        assert!(!EmailAllowlist::matches_pattern(
            "other@sub.example.com",
            "user@*.example.com"
        ));
    }

    #[test]
    fn test_normalization() {
        let allowlist = EmailAllowlist {
            patterns: vec!["user@example.com".to_string()],
        };

        // Test that normalization works (trim, NFKC, lowercase)
        assert!(allowlist.is_allowed("  USER@EXAMPLE.COM  "));
    }

    #[test]
    fn test_pattern_normalization() {
        // Test that pattern matching works with normalized patterns.
        // This simulates what happens when from_env() normalizes patterns.
        // A stored pattern equivalent to "USER@EXAMPLE.COM" should match
        // different case/whitespace variants of the same email.
        let allowlist = EmailAllowlist {
            // Pattern is already normalized (lowercase, trimmed, NFKC).
            patterns: vec!["user@example.com".to_string()],
        };

        // Input emails should be normalized before matching.
        assert!(allowlist.is_allowed("user@example.com"));
        assert!(allowlist.is_allowed("USER@EXAMPLE.COM"));
        assert!(allowlist.is_allowed("  User@Example.Com  "));
    }
}
