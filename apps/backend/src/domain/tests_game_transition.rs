use crate::domain::game_transition::{derive_game_transitions, GameTransition};

#[test]
fn derive_transitions_empty_when_turn_unchanged_none() {
    let t = derive_game_transitions(None, None);
    assert!(t.is_empty());
}

#[test]
fn derive_transitions_empty_when_turn_unchanged_some() {
    let t = derive_game_transitions(Some(2), Some(2));
    assert!(t.is_empty());
}

#[test]
fn derive_transitions_emits_turn_became_on_none_to_some() {
    let t = derive_game_transitions(None, Some(1));
    assert_eq!(t, vec![GameTransition::TurnBecame { player_id: 1 }]);
}

#[test]
fn derive_transitions_emits_turn_became_on_some_to_different_some() {
    let t = derive_game_transitions(Some(0), Some(3));
    assert_eq!(t, vec![GameTransition::TurnBecame { player_id: 3 }]);
}

#[test]
fn derive_transitions_empty_on_some_to_none() {
    // Edge-triggered only: "turn became player X" (not "turn cleared").
    let t = derive_game_transitions(Some(0), None);
    assert!(t.is_empty());
}
