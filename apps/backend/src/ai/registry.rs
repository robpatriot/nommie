//! How to register your AI
//!
//! 1) Implement `AiPlayer` for your type in its module.
//! 2) Add a new `AiFactory` entry to the static list with stable `name` and `version`.
//! 3) Keep ordering stable; avoid side effects in constructors.
//! 4) Determinism: same seed ⇒ same behavior (where applicable).
//! 5) Include profile metadata (display_name, playstyle, memory_level, etc.) in the factory.

use crate::ai::{AiPlayer, Heuristic, RandomPlayer, Reckoner, Strategic, Tactician};

/// The default AI used when adding AI players to games.
/// This is the single source of truth for the default AI selection.
pub const DEFAULT_AI_NAME: &str = Tactician::NAME;

/// Factory definition for constructing AI implementations.
pub struct AiFactory {
    pub name: &'static str,
    pub version: &'static str,
    pub make: fn(seed: Option<u64>) -> Box<dyn AiPlayer + Send + Sync>,
    /// Default profile metadata for this AI type.
    pub profile: AiProfileDefaults,
}

/// Default profile metadata for an AI type.
pub struct AiProfileDefaults {
    /// Variant name (typically "default")
    pub variant: &'static str,
    /// Display name for the AI profile
    pub display_name: &'static str,
    /// Optional playstyle descriptor
    pub playstyle: Option<&'static str>,
    /// Optional difficulty level
    pub difficulty: Option<i32>,
    /// Optional JSON configuration
    pub config: Option<serde_json::Value>,
    /// Optional memory level (0-100)
    pub memory_level: Option<i32>,
}

static AI_FACTORIES: &[AiFactory] = &[
    AiFactory {
        name: RandomPlayer::NAME,
        version: RandomPlayer::VERSION,
        make: make_random_player,
        profile: AiProfileDefaults {
            variant: "default",
            display_name: "Random Player",
            playstyle: Some("random"),
            difficulty: None,
            config: None,
            memory_level: Some(50),
        },
    },
    AiFactory {
        name: Heuristic::NAME,
        version: Heuristic::VERSION,
        make: make_heuristic,
        profile: AiProfileDefaults {
            variant: "default",
            display_name: "Heuristic",
            playstyle: Some("heuristic"),
            difficulty: None,
            config: None,
            memory_level: Some(80),
        },
    },
    AiFactory {
        name: Strategic::NAME,
        version: Strategic::VERSION,
        make: make_strategic,
        profile: AiProfileDefaults {
            variant: "default",
            display_name: "Strategic",
            playstyle: Some("strategic"),
            difficulty: None,
            config: None,
            memory_level: Some(90),
        },
    },
    AiFactory {
        name: Reckoner::NAME,
        version: Reckoner::VERSION,
        make: make_reckoner,
        profile: AiProfileDefaults {
            variant: "default",
            display_name: "Reckoner",
            playstyle: Some("strategic"),
            difficulty: None,
            config: None,
            memory_level: Some(90),
        },
    },
    AiFactory {
        name: Tactician::NAME,
        version: Tactician::VERSION,
        make: make_tactician,
        profile: AiProfileDefaults {
            variant: "default",
            display_name: "Tactician",
            playstyle: Some("tactical"),
            difficulty: None,
            config: None,
            memory_level: Some(80),
        },
    },
];

/// Returns the statically registered AI factories.
pub fn registered_ais() -> &'static [AiFactory] {
    AI_FACTORIES
}

/// Finds a registered AI factory by its name.
pub fn by_name(name: &str) -> Option<&'static AiFactory> {
    registered_ais().iter().find(|factory| factory.name == name)
}

/// Returns the default AI factory.
/// Returns None only if DEFAULT_AI_NAME is misconfigured (should never happen).
pub fn default_ai() -> Option<&'static AiFactory> {
    by_name(DEFAULT_AI_NAME)
}

fn make_random_player(seed: Option<u64>) -> Box<dyn AiPlayer + Send + Sync> {
    Box::new(RandomPlayer::new(seed))
}

fn make_heuristic(seed: Option<u64>) -> Box<dyn AiPlayer + Send + Sync> {
    Box::new(Heuristic::new(seed))
}

fn make_strategic(seed: Option<u64>) -> Box<dyn AiPlayer + Send + Sync> {
    Box::new(Strategic::new(seed))
}

fn make_reckoner(seed: Option<u64>) -> Box<dyn AiPlayer + Send + Sync> {
    Box::new(Reckoner::new(seed))
}

fn make_tactician(seed: Option<u64>) -> Box<dyn AiPlayer + Send + Sync> {
    Box::new(Tactician::new(seed))
}

#[cfg(test)]
mod ai_registry_smoke {
    use super::*;

    #[test]
    fn enumerates_registered_ais() {
        let ais = registered_ais();
        assert!(
            !ais.is_empty(),
            "registered_ais should include at least one AI factory"
        );
        assert!(
            ais.iter().any(|factory| factory.name == RandomPlayer::NAME),
            "RandomPlayer factory should be present"
        );
        assert!(
            ais.iter().any(|factory| factory.name == Heuristic::NAME),
            "Heuristic factory should be present"
        );
        assert!(
            ais.iter().any(|factory| factory.name == Strategic::NAME),
            "Strategic factory should be present"
        );
        assert!(
            ais.iter().any(|factory| factory.name == Reckoner::NAME),
            "Reckoner factory should be present"
        );
        assert!(
            ais.iter().any(|factory| factory.name == Tactician::NAME),
            "Tactician factory should be present"
        );
    }

    #[test]
    fn constructs_random_player_with_seed() {
        let factory =
            by_name(RandomPlayer::NAME).expect("RandomPlayer must be discoverable through by_name");

        let ai_a = (factory.make)(Some(123));
        let ai_b = (factory.make)(Some(123));

        let _: &(dyn AiPlayer + Send + Sync) = ai_a.as_ref();
        let _: &(dyn AiPlayer + Send + Sync) = ai_b.as_ref();
    }

    #[test]
    fn lookup_helper_behaves() {
        assert!(by_name(RandomPlayer::NAME).is_some());
        assert!(by_name(Heuristic::NAME).is_some());
        assert!(by_name(Strategic::NAME).is_some());
        assert!(by_name(Reckoner::NAME).is_some());
        assert!(by_name(Tactician::NAME).is_some());
        assert!(by_name("NotARealAI").is_none());
    }
}
