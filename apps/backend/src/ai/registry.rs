//! How to register your AI
//!
//! 1) Implement `AiPlayer` for your type in its module.
//! 2) Add a new `AiFactory` entry to the static list with stable `name` and `version`.
//! 3) Keep ordering stable; avoid side effects in constructors.
//! 4) Determinism: same seed ⇒ same behavior (where applicable).
//! 5) Profile fields are out of scope in Step 1; they will be added later as metadata.

use crate::ai::{AiPlayer, Heuristic, RandomPlayer};

/// Factory definition for constructing AI implementations.
pub struct AiFactory {
    pub name: &'static str,
    pub version: &'static str,
    pub make: fn(seed: Option<u64>) -> Box<dyn AiPlayer + Send + Sync>,
}

static AI_FACTORIES: &[AiFactory] = &[
    AiFactory {
        name: RandomPlayer::NAME,
        version: RandomPlayer::VERSION,
        make: make_random_player,
    },
    AiFactory {
        name: Heuristic::NAME,
        version: Heuristic::VERSION,
        make: make_heuristic,
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

fn make_random_player(seed: Option<u64>) -> Box<dyn AiPlayer + Send + Sync> {
    Box::new(RandomPlayer::new(seed))
}

fn make_heuristic(seed: Option<u64>) -> Box<dyn AiPlayer + Send + Sync> {
    Box::new(Heuristic::new(seed))
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
        assert!(by_name("NotARealAI").is_none());
    }
}
