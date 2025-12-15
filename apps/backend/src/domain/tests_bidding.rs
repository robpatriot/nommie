use crate::domain::bidding::{legal_bids_for_hand_size, place_bid, set_trump, Bid};
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
fn bidding_legal_range_and_phase_turning() {
    let hands = [Vec::new(), Vec::new(), Vec::new(), Vec::new()];
    let mut state = make_state_with_hands(hands, 5, 0);

    let bids = legal_bids_for_hand_size(state.hand_size);
    assert_eq!(bids.first().unwrap().0, 0);
    assert_eq!(bids.last().unwrap().0, 5);

    assert!(place_bid(&mut state, 0, Bid(3), None).is_ok());
    assert!(place_bid(&mut state, 1, Bid(4), None).is_ok());
    assert!(place_bid(&mut state, 2, Bid(1), None).is_ok());
    assert!(place_bid(&mut state, 3, Bid(4), None).is_ok());
    // Highest is 4; tie between player 1 and 3; start is 0 so earliest in order is 1
    assert_eq!(state.phase, Phase::TrumpSelect);
    assert_eq!(state.round.winning_bidder, Some(1));
}

#[test]
fn dealer_bid_rejected_when_sum_would_equal_hand_size() {
    // hand_size = 2, turn_start = 0 -> bid order: 0, 1, 2, 3
    let hands = [Vec::new(), Vec::new(), Vec::new(), Vec::new()];
    let mut state = make_state_with_hands(hands, 2, 0);

    // First three bids (non-final bidder)
    assert!(place_bid(&mut state, 0, Bid(0), None).is_ok());
    assert!(place_bid(&mut state, 1, Bid(1), None).is_ok());
    assert!(place_bid(&mut state, 2, Bid(0), None).is_ok());

    // At this point, bids_count == 3 and it's the final bidder's turn (player 3).
    // A bid of 1 would make sum == hand_size (0 + 1 + 0 + 1 = 2), so it must fail.
    let err = place_bid(&mut state, 3, Bid(1), None).unwrap_err();
    match err {
        crate::errors::domain::DomainError::Validation(
            crate::errors::domain::ValidationKind::InvalidBid,
            msg,
        ) => {
            assert!(
                msg.contains("Dealer cannot bid"),
                "unexpected error message: {msg}"
            );
        }
        other => panic!("expected InvalidBid validation error, got: {other:?}"),
    }
}

#[test]
fn trump_selection_only_by_winner() {
    let hands = [Vec::new(), Vec::new(), Vec::new(), Vec::new()];
    let mut state = make_state_with_hands(hands, 3, 0);
    for (p, b) in [(0, 0), (1, 2), (2, 2), (3, 1)] {
        assert!(place_bid(&mut state, p, Bid(b), None).is_ok());
    }
    assert_eq!(state.round.winning_bidder, Some(1));
    // wrong player
    assert!(set_trump(&mut state, 0, Trump::Hearts).is_err());
    // correct player
    assert!(set_trump(&mut state, 1, Trump::Spades).is_ok());
    assert_eq!(state.phase, Phase::Trick { trick_no: 1 });
    // First trick is led by player to left of dealer (turn_start)
    assert_eq!(state.leader, 0);
    assert_eq!(state.turn, 0);
    assert_eq!(state.round.trump, Some(Trump::Spades));
    assert!(state.round.trick_plays.is_empty());
    assert!(state.round.trick_lead.is_none());
}

#[test]
fn trump_selection_allows_no_trump() {
    let hands = [Vec::new(), Vec::new(), Vec::new(), Vec::new()];
    let mut state = make_state_with_hands(hands, 3, 0);
    for (p, b) in [(0, 0), (1, 2), (2, 2), (3, 1)] {
        assert!(place_bid(&mut state, p, Bid(b), None).is_ok());
    }
    assert_eq!(state.round.winning_bidder, Some(1));

    // Winning bidder can select NoTrumps
    assert!(set_trump(&mut state, 1, Trump::NoTrumps).is_ok());
    assert_eq!(state.phase, Phase::Trick { trick_no: 1 });
    assert_eq!(state.round.trump, Some(Trump::NoTrumps));
}
