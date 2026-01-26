use crate::domain::bidding::{legal_bids, place_bid, set_trump, Bid};
use crate::domain::state::Phase;
use crate::domain::test_state_helpers::{make_game_state, MakeGameStateArgs};
use crate::domain::Trump;

#[test]
fn bidding_legal_range_and_phase_turning() {
    let hands = [Vec::new(), Vec::new(), Vec::new(), Vec::new()];

    let mut state = make_game_state(
        hands,
        MakeGameStateArgs {
            phase: Phase::Bidding,
            round_no: Some(1),
            hand_size: Some(5),
            dealer: Some(3),
            ..Default::default()
        },
    );

    let bids = legal_bids(&state, 0);
    assert_eq!(bids.first().unwrap().0, 0);
    assert_eq!(bids.last().unwrap().0, 5);

    assert!(place_bid(&mut state, 0, Bid(3)).is_ok());
    assert!(place_bid(&mut state, 1, Bid(4)).is_ok());
    assert!(place_bid(&mut state, 2, Bid(1)).is_ok());
    assert!(place_bid(&mut state, 3, Bid(4)).is_ok());
    // Highest is 4; tie between player 1 and 3; start is 0 so earliest in order is 1
    assert_eq!(state.phase, Phase::TrumpSelect);
    assert_eq!(state.round.winning_bidder, Some(1));
}

#[test]
fn dealer_bid_rejected_when_sum_would_equal_hand_size() {
    // hand_size = 2, dealer = 0 -> bid order: 1, 2, 3, 0 (dealer bids last)
    let hands = [Vec::new(), Vec::new(), Vec::new(), Vec::new()];
    let mut state = make_game_state(
        hands,
        MakeGameStateArgs {
            phase: Phase::Bidding,
            round_no: Some(1),
            hand_size: Some(2),
            dealer: Some(0),
            ..Default::default()
        },
    );

    // First three bids (non-final bidder)
    assert!(place_bid(&mut state, 1, Bid(0)).is_ok());
    assert!(place_bid(&mut state, 2, Bid(1)).is_ok());
    assert!(place_bid(&mut state, 3, Bid(0)).is_ok());

    // Final bidder is the dealer (player 0).
    // A bid of 1 would make sum == hand_size (0 + 1 + 0 + 1 = 2), so it must fail.
    let err = place_bid(&mut state, 0, Bid(1)).unwrap_err();
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
    let mut state = make_game_state(
        hands,
        MakeGameStateArgs {
            phase: Phase::Bidding,
            round_no: Some(1),
            hand_size: Some(3),
            dealer: Some(3),
            ..Default::default()
        },
    );

    for (p, b) in [(0, 0), (1, 2), (2, 2), (3, 1)] {
        assert!(place_bid(&mut state, p, Bid(b)).is_ok());
    }
    assert_eq!(state.round.winning_bidder, Some(1));
    // wrong player
    assert!(set_trump(&mut state, 0, Trump::Hearts).is_err());
    // correct player
    assert!(set_trump(&mut state, 1, Trump::Spades).is_ok());
    assert_eq!(state.phase, Phase::Trick { trick_no: 1 });
    // First trick is led by player to left of dealer (turn_start)
    assert_eq!(state.leader, Some(0));
    assert_eq!(state.turn, Some(0));
    assert_eq!(state.round.trump, Some(Trump::Spades));
    assert!(state.round.trick_plays.is_empty());
    assert!(state.round.trick_lead.is_none());
}

#[test]
fn trump_selection_allows_no_trump() {
    let hands = [Vec::new(), Vec::new(), Vec::new(), Vec::new()];
    let mut state = make_game_state(
        hands,
        MakeGameStateArgs {
            phase: Phase::Bidding,
            round_no: Some(1),
            hand_size: Some(3),
            dealer: Some(3),
            ..Default::default()
        },
    );

    for (p, b) in [(0, 0), (1, 2), (2, 2), (3, 1)] {
        assert!(place_bid(&mut state, p, Bid(b)).is_ok());
    }
    assert_eq!(state.round.winning_bidder, Some(1));

    // Winning bidder can select NoTrumps
    assert!(set_trump(&mut state, 1, Trump::NoTrumps).is_ok());
    assert_eq!(state.phase, Phase::Trick { trick_no: 1 });
    assert_eq!(state.round.trump, Some(Trump::NoTrumps));
}
