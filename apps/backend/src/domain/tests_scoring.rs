use crate::domain::scoring::apply_round_scoring;
use crate::domain::state::{Phase, PlayerId};
use crate::domain::test_state_helpers::{make_game_state, MakeGameStateArgs};
use crate::domain::{Card, Trump};

fn make_scoring_state_with_hands(
    hands: [Vec<Card>; 4],
    hand_size: u8,
    turn_start: PlayerId,
) -> crate::domain::state::GameState {
    // Preserve the old "turn_start" meaning ("round start seat") under the new model:
    // round start seat = next_player(dealer)  =>  dealer = prev_player(turn_start)
    let dealer: PlayerId = ((turn_start + 3) % 4) as PlayerId;

    make_game_state(
        hands,
        MakeGameStateArgs {
            phase: Phase::Scoring,
            round_no: Some(1),
            hand_size: Some(hand_size),
            dealer: Some(dealer),
            turn: None,
            leader: None,
            trick_no: Some(0),
            scores_total: [0; 4],
        },
    )
}

#[test]
fn scoring_bonus_only_on_exact_bid() {
    let hands = [Vec::new(), Vec::new(), Vec::new(), Vec::new()];
    let mut state = make_scoring_state_with_hands(hands, 3, 0);

    // Fake some tallies - sum must equal hand_size
    state.round.tricks_won = [2, 1, 0, 0]; // Sum = 3, matches hand_size = 3
    state.round.bids = [Some(2), Some(0), Some(1), Some(0)];

    let _result = apply_round_scoring(&mut state);
    assert_eq!(state.scores_total, [12, 1, 0, 10]);
    assert_eq!(state.phase, Phase::Complete);
}

#[test]
fn scoring_exact_bid_bonus_applied_once() {
    // "Exact-bid bonus applied once": bids [3,2,4,1], tricks [3,2,7,1] → totals [13,12,7,11]
    let hands = [Vec::new(), Vec::new(), Vec::new(), Vec::new()];
    let mut state = make_scoring_state_with_hands(hands, 13, 0);

    state.round.bids = [Some(3), Some(2), Some(4), Some(1)];
    state.round.tricks_won = [3, 2, 7, 1];

    // Verify sum-of-tricks invariant before scoring
    let tricks_sum: u8 = state.round.tricks_won.iter().sum();
    assert_eq!(
        tricks_sum,
        state
            .hand_size
            .expect("hand_size should be Some in Scoring phase")
    );

    let _result = apply_round_scoring(&mut state);

    // Expected: [3+10, 2+10, 7+0, 1+10] = [13, 12, 7, 11]
    assert_eq!(state.scores_total, [13, 12, 7, 11]);
    assert_eq!(state.phase, Phase::Complete);
}

#[test]
fn scoring_notrump_does_not_affect_scoring_math() {
    // "NoTrumps does not affect scoring math": bids [0,5,8,0], tricks [0,5,8,0] → totals [10,15,18,10]
    let hands = [Vec::new(), Vec::new(), Vec::new(), Vec::new()];
    let mut state = make_scoring_state_with_hands(hands, 13, 0);

    state.round.bids = [Some(0), Some(5), Some(8), Some(0)];
    state.round.tricks_won = [0, 5, 8, 0];
    state.round.trump = Some(Trump::NoTrumps); // NoTrumps setting

    // Verify sum-of-tricks invariant before scoring
    let tricks_sum: u8 = state.round.tricks_won.iter().sum();
    assert_eq!(
        tricks_sum,
        state
            .hand_size
            .expect("hand_size should be Some in Scoring phase")
    );

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
    let mut state = make_scoring_state_with_hands(hands, 5, 0);

    state.round.bids = [Some(2), Some(1), Some(2), Some(0)];
    state.round.tricks_won = [2, 1, 2, 0];

    // Verify sum-of-tricks invariant before scoring
    let tricks_sum: u8 = state.round.tricks_won.iter().sum();
    assert_eq!(
        tricks_sum,
        state
            .hand_size
            .expect("hand_size should be Some in Scoring phase")
    );

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
fn scoring_sum_of_tricks_invariant_violation_is_noop() {
    let hands = [Vec::new(), Vec::new(), Vec::new(), Vec::new()];
    let mut state = make_scoring_state_with_hands(hands, 5, 0);

    state.round.bids = [Some(2), Some(1), Some(2), Some(0)];
    state.round.tricks_won = [3, 1, 2, 0]; // Sum = 6, but hand_size = 5

    let tricks_sum: u8 = state.round.tricks_won.iter().sum();
    let hand_size = state
        .hand_size
        .expect("hand_size should be Some in Scoring phase");
    assert_ne!(
        tricks_sum, hand_size,
        "test intentionally violates invariant"
    );

    let scores_before = state.scores_total;
    let phase_before = state.phase;

    let result = apply_round_scoring(&mut state);

    assert_eq!(
        state.phase, phase_before,
        "should no-op and stay in Scoring"
    );
    assert_eq!(state.scores_total, scores_before, "scores must not change");
    assert_eq!(result.round_score_deltas, [0, 0, 0, 0], "no-op deltas");
}
