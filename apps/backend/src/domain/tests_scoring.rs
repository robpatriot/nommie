use crate::domain::scoring::apply_round_scoring;
use crate::domain::state::{GameState, Phase, PlayerId, RoundState};
use crate::domain::{Card, Trump};

fn make_state_with_hands(hands: [Vec<Card>; 4], hand_size: u8, turn_start: PlayerId) -> GameState {
    GameState {
        phase: Phase::Bidding,
        round_no: 1,
        hand_size,
        hands,
        turn_start,
        turn: turn_start,
        leader: turn_start,
        trick_no: 0,
        scores_total: [0; 4],
        round: RoundState::empty(),
    }
}

#[test]
fn scoring_bonus_only_on_exact_bid() {
    let hands = [Vec::new(), Vec::new(), Vec::new(), Vec::new()];
    let mut state = make_state_with_hands(hands, 3, 0);
    // Fake some tallies - sum must equal hand_size
    state.round.tricks_won = [2, 1, 0, 0]; // Sum = 3, matches hand_size = 3
    state.round.bids = [Some(2), Some(0), Some(1), Some(0)];
    state.phase = Phase::Scoring;
    let _result = apply_round_scoring(&mut state);
    assert_eq!(state.scores_total, [12, 1, 0, 10]);
    assert_eq!(state.phase, Phase::Complete);
}

#[test]
fn scoring_exact_bid_bonus_applied_once() {
    // "Exact-bid bonus applied once": bids [3,2,4,1], tricks [3,2,7,1] → totals [13,12,7,11]
    let hands = [Vec::new(), Vec::new(), Vec::new(), Vec::new()];
    let mut state = make_state_with_hands(hands, 13, 0);
    state.round.bids = [Some(3), Some(2), Some(4), Some(1)];
    state.round.tricks_won = [3, 2, 7, 1];
    state.phase = Phase::Scoring;

    // Verify sum-of-tricks invariant before scoring
    let tricks_sum: u8 = state.round.tricks_won.iter().sum();
    assert_eq!(tricks_sum, state.hand_size);

    let _result = apply_round_scoring(&mut state);

    // Expected: [3+10, 2+10, 7+0, 1+10] = [13, 12, 7, 11]
    assert_eq!(state.scores_total, [13, 12, 7, 11]);
    assert_eq!(state.phase, Phase::Complete);
}

#[test]
fn scoring_notrump_does_not_affect_scoring_math() {
    // "NoTrumps does not affect scoring math": bids [0,5,8,0], tricks [0,5,8,0] → totals [10,15,18,10]
    let hands = [Vec::new(), Vec::new(), Vec::new(), Vec::new()];
    let mut state = make_state_with_hands(hands, 13, 0);
    state.round.bids = [Some(0), Some(5), Some(8), Some(0)];
    state.round.tricks_won = [0, 5, 8, 0];
    state.round.trump = Some(Trump::NoTrumps); // NoTrumps setting
    state.phase = Phase::Scoring;

    // Verify sum-of-tricks invariant before scoring
    let tricks_sum: u8 = state.round.tricks_won.iter().sum();
    assert_eq!(tricks_sum, state.hand_size);

    let _result = apply_round_scoring(&mut state);

    // Expected: [0+10, 5+10, 8+10, 0+10] = [10, 15, 18, 10]
    assert_eq!(state.scores_total, [10, 15, 18, 10]);
    assert_eq!(state.phase, Phase::Complete);
}

#[test]
fn scoring_idempotence_scoring_applies_once_only() {
    // "Idempotence: scoring applies once only": call apply_round_scoring twice;
    // the second call (after phase is Complete) must leave totals unchanged
    let hands = [Vec::new(), Vec::new(), Vec::new(), Vec::new()];
    let mut state = make_state_with_hands(hands, 5, 0);
    state.round.bids = [Some(2), Some(1), Some(2), Some(0)];
    state.round.tricks_won = [2, 1, 2, 0];
    state.phase = Phase::Scoring;

    // Verify sum-of-tricks invariant before scoring
    let tricks_sum: u8 = state.round.tricks_won.iter().sum();
    assert_eq!(tricks_sum, state.hand_size);

    // First scoring call
    let _result1 = apply_round_scoring(&mut state);
    let scores_after_first = state.scores_total;
    assert_eq!(state.phase, Phase::Complete);

    // Second scoring call should be no-op
    let _result2 = apply_round_scoring(&mut state);
    assert_eq!(state.scores_total, scores_after_first);
    assert_eq!(state.phase, Phase::Complete);
}

#[test]
fn scoring_sum_of_tricks_invariant_violation_release_variant() {
    // Construct a state with hand_size = N and intentionally make
    // tricks_won sum != N. Assert on the sum inside the test **before**
    // invoking scoring. This documents the invariant.
    let hands = [Vec::new(), Vec::new(), Vec::new(), Vec::new()];
    let mut state = make_state_with_hands(hands, 5, 0);
    state.round.bids = [Some(2), Some(1), Some(2), Some(0)];
    state.round.tricks_won = [3, 1, 2, 0]; // Sum = 6, but hand_size = 5
    state.phase = Phase::Scoring;

    // Verify sum-of-tricks invariant violation before scoring
    let tricks_sum: u8 = state.round.tricks_won.iter().sum();
    assert_ne!(
        tricks_sum, state.hand_size,
        "Test intentionally violates sum-of-tricks invariant"
    );

    // In non-debug (release) builds, scoring should not panic and produce deterministic totals
    #[cfg(not(debug_assertions))]
    {
        let _result = apply_round_scoring(&mut state);
        assert_eq!(state.scores_total, [13, 11, 12, 10]);
    }
}

#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "Sum of tricks")]
fn scoring_sum_of_tricks_invariant_violation_debug_variant() {
    // Construct the same invalid state as the release variant
    let hands = [Vec::new(), Vec::new(), Vec::new(), Vec::new()];
    let mut state = make_state_with_hands(hands, 5, 0);
    state.round.bids = [Some(2), Some(1), Some(2), Some(0)];
    state.round.tricks_won = [3, 1, 2, 0]; // Sum = 6, but hand_size = 5
    state.phase = Phase::Scoring;

    // Sanity-check the invariant is indeed violated
    let tricks_sum: u8 = state.round.tricks_won.iter().sum();
    assert_ne!(tricks_sum, state.hand_size);

    // In debug builds, this should panic due to the internal debug assertion
    let _result = apply_round_scoring(&mut state);
}
