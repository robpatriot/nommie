//! AI configuration handling.
//!
//! Provides typed interface for AI configuration, extracting standard fields
//! from the database JSON config while preserving AI-specific custom fields.

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

/// Standard configuration for AI players.
///
/// This struct extracts common configuration fields from the JSON config
/// stored in `ai_profiles.config`, while preserving AI-specific fields
/// in the `custom` field.
///
/// # Standard Fields
///
/// - `seed`: Optional RNG seed for deterministic behavior. If provided,
///   the AI should use this to seed its random number generator for
///   reproducible decision-making. Useful for testing and debugging.
///
/// # Example JSON Config
///
/// Simple random AI with seed:
/// ```json
/// {"seed": 12345}
/// ```
///
/// AI with seed and memory recency:
/// ```json
/// {
///   "seed": 12345,
///   "memory_recency": true
/// }
/// ```
///
/// AI with seed and custom fields:
/// ```json
/// {
///   "seed": 12345,
///   "memory_recency": false,
///   "aggression": 0.7,
///   "risk_tolerance": "high"
/// }
/// ```
///
/// # Usage
///
/// ```rust,ignore
/// let config = AiConfig::from_json(profile.config.as_ref());
/// let seed = config.seed();
/// let custom_field = config.get_custom("aggression");
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiConfig {
    /// Optional RNG seed for deterministic AI behavior.
    ///
    /// When present, AI implementations should use this to seed their
    /// random number generator, ensuring reproducible decisions across runs.
    /// This is particularly useful for:
    /// - Testing AI behavior
    /// - Debugging specific game scenarios
    /// - Replaying games with consistent AI decisions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<u64>,

    /// Optional memory recency bias for AI memory system.
    ///
    /// When enabled, AIs will remember recent tricks better than older tricks
    /// within the current round. This provides more realistic human-like memory
    /// where recent events are clearer than distant ones.
    ///
    /// - `true`: Apply 1.1x boost to memory recall for last 3 tricks
    /// - `false` or `None`: Uniform memory across all tricks (default)
    ///
    /// Currently uses gentle recency bias (10% boost for recent, no penalty for old).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory_recency: Option<bool>,

    /// AI-specific configuration.
    ///
    /// This preserves any custom fields from the original JSON config
    /// that aren't part of the standard AiConfig schema. AI implementations
    /// can query this for their own configuration needs.
    #[serde(flatten)]
    pub custom: JsonValue,
}

impl AiConfig {
    /// Create an AiConfig from optional JSON value.
    ///
    /// Extracts standard fields (like `seed`) while preserving all other
    /// fields in `custom`. If the input is `None`, returns an empty config.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let json = serde_json::json!({"seed": 123, "difficulty": 5});
    /// let config = AiConfig::from_json(Some(&json));
    /// assert_eq!(config.seed, Some(123));
    /// assert_eq!(config.get_custom("difficulty"), Some(&json!(5)));
    /// ```
    pub fn from_json(config: Option<&JsonValue>) -> Self {
        match config {
            Some(json) => {
                // Try to deserialize, falling back to empty config on error
                serde_json::from_value(json.clone()).unwrap_or_else(|_| Self {
                    seed: None,
                    memory_recency: None,
                    custom: JsonValue::Object(serde_json::Map::new()),
                })
            }
            None => Self {
                seed: None,
                memory_recency: None,
                custom: JsonValue::Object(serde_json::Map::new()),
            },
        }
    }

    /// Get the RNG seed, if configured.
    pub fn seed(&self) -> Option<u64> {
        self.seed
    }

    /// Check if memory recency bias is enabled.
    ///
    /// Returns true if explicitly enabled, false otherwise (default: false).
    pub fn memory_recency(&self) -> bool {
        self.memory_recency.unwrap_or(false)
    }

    /// Get a custom configuration field by key.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let difficulty = config.get_custom("difficulty")
    ///     .and_then(|v| v.as_i64())
    ///     .unwrap_or(5);
    /// ```
    pub fn get_custom(&self, key: &str) -> Option<&JsonValue> {
        self.custom.get(key)
    }

    /// Create an empty configuration (no seed, no custom fields).
    pub fn empty() -> Self {
        Self {
            seed: None,
            memory_recency: None,
            custom: JsonValue::Object(serde_json::Map::new()),
        }
    }

    /// Create a configuration with just a seed.
    pub fn with_seed(seed: u64) -> Self {
        Self {
            seed: Some(seed),
            memory_recency: None,
            custom: JsonValue::Object(serde_json::Map::new()),
        }
    }
}

impl Default for AiConfig {
    fn default() -> Self {
        Self::empty()
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn test_from_json_with_seed_only() {
        let json = json!({"seed": 12345});
        let config = AiConfig::from_json(Some(&json));

        assert_eq!(config.seed(), Some(12345));
    }

    #[test]
    fn test_from_json_with_seed_and_custom() {
        let json = json!({
            "seed": 67890,
            "difficulty": 7,
            "playstyle": "aggressive"
        });
        let config = AiConfig::from_json(Some(&json));

        assert_eq!(config.seed(), Some(67890));
        assert_eq!(config.get_custom("difficulty"), Some(&json!(7)));
        assert_eq!(config.get_custom("playstyle"), Some(&json!("aggressive")));
    }

    #[test]
    fn test_from_json_none() {
        let config = AiConfig::from_json(None);

        assert_eq!(config.seed(), None);
        assert!(config.get_custom("anything").is_none());
    }

    #[test]
    fn test_from_json_no_seed() {
        let json = json!({"difficulty": 5});
        let config = AiConfig::from_json(Some(&json));

        assert_eq!(config.seed(), None);
        assert_eq!(config.get_custom("difficulty"), Some(&json!(5)));
    }

    #[test]
    fn test_with_seed() {
        let config = AiConfig::with_seed(99999);

        assert_eq!(config.seed(), Some(99999));
    }

    #[test]
    fn test_empty() {
        let config = AiConfig::empty();

        assert_eq!(config.seed(), None);
        assert!(config.get_custom("anything").is_none());
    }
}
