#![cfg(test)]

use super::*;

use crate::domain::cards::{Card, Rank, Suit, parse_cards};
use crate::domain::bidding::{Bid, place_bid};
use crate::domain::scoring::apply_round_scoring;
use crate::domain::tricks::{legal_moves, play_card, resolve_current_trick};

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
        round: RoundState::new(),
    }
}

#[test]
fn bidding_legal_range_and_phase_turning() {
    let hands = [Vec::new(), Vec::new(), Vec::new(), Vec::new()];
    let mut state = make_state_with_hands(hands, 5, 0);

    let bids = crate::domain::bidding::legal_bids(&state, 0);
    assert_eq!(bids.first().unwrap().value(), 0);
    assert_eq!(bids.last().unwrap().value(), 5);

    assert!(place_bid(&mut state, 0, Bid(3)).is_ok());
    assert!(place_bid(&mut state, 1, Bid(4)).is_ok());
    assert!(place_bid(&mut state, 2, Bid(1)).is_ok());
    assert!(place_bid(&mut state, 3, Bid(4)).is_ok());
    // Highest is 4; tie between player 1 and 3; start is 0 so earliest in order is 1
    assert_eq!(state.phase, Phase::TrumpSelect);
    assert_eq!(state.round.winning_bidder, Some(1));
}

#[test]
fn trump_selection_only_by_winner() {
    let hands = [Vec::new(), Vec::new(), Vec::new(), Vec::new()];
    let mut state = make_state_with_hands(hands, 3, 0);
    for (p, b) in [(0, 0), (1, 2), (2, 2), (3, 1)] {
        assert!(place_bid(&mut state, p, Bid(b)).is_ok());
    }
    assert_eq!(state.round.winning_bidder, Some(1));
    // wrong player
    assert!(crate::domain::set_trump(&mut state, 0, Suit::Hearts).is_err());
    // correct player
    assert!(crate::domain::set_trump(&mut state, 1, Suit::Spades).is_ok());
    assert_eq!(state.phase, Phase::Trick { trick_no: 1 });
    assert_eq!(state.leader, 1);
    assert_eq!(state.turn, 1);
    assert_eq!(state.round.trump, Some(Suit::Spades));
    assert!(state.round.trick_plays.is_empty());
    assert!(state.round.trick_lead.is_none());
}

#[test]
fn legal_moves_follow_lead() {
    // Hands for a small test
    let h0 = parse_cards(&["AS", "KH", "2C"]);
    let h1 = parse_cards(&["TS", "3H", "4C"]);
    let h2 = parse_cards(&["QS", "5D", "6C"]);
    let h3 = parse_cards(&["9S", "7H", "8C"]);
    let mut state = make_state_with_hands([h0, h1, h2, h3], 3, 0);
    for p in 0..4 {
        assert!(place_bid(&mut state, p, Bid(0)).is_ok());
    }
    crate::domain::set_trump(&mut state, 0, Suit::Hearts).unwrap();
    // First to play can play any
    let lm0 = legal_moves(&state, 0);
    assert_eq!(lm0.len(), 3);
    // Play AS -> lead Spades
    play_card(&mut state, 0, Card { suit: Suit::Spades, rank: Rank::Ace }).unwrap();
    // Player 1 must follow spades if possible
    let lm1 = legal_moves(&state, 1);
    assert!(lm1.iter().all(|c| c.suit == Suit::Spades) && !lm1.is_empty());
}

#[test]
fn play_card_errors_and_trick_resolution() {
    let h0 = parse_cards(&["AS", "KH", "2C"]);
    let h1 = parse_cards(&["TS", "3H", "4C"]);
    let h2 = parse_cards(&["QS", "5D", "6C"]);
    let h3 = parse_cards(&["9S", "7H", "8C"]);
    let mut state = make_state_with_hands([h0, h1, h2, h3], 3, 0);
    for p in 0..4 { place_bid(&mut state, p, Bid(0)).unwrap(); }
    crate::domain::set_trump(&mut state, 0, Suit::Hearts).unwrap();
    // Out of turn
    assert_eq!(play_card(&mut state, 1, Card { suit: Suit::Spades, rank: Rank::Ten }).unwrap_err(), DomainError::OutOfTurn);
    // Not in hand
    assert_eq!(play_card(&mut state, 0, Card { suit: Suit::Diamonds, rank: Rank::Ace }).unwrap_err(), DomainError::CardNotInHand);
    // Play trick fully
    play_card(&mut state, 0, Card { suit: Suit::Spades, rank: Rank::Ace }).unwrap();
    play_card(&mut state, 1, Card { suit: Suit::Spades, rank: Rank::Ten }).unwrap();
    play_card(&mut state, 2, Card { suit: Suit::Spades, rank: Rank::Queen }).unwrap();
    play_card(&mut state, 3, Card { suit: Suit::Spades, rank: Rank::Nine }).unwrap();
    // Highest trump is none in trick; lead spades so Ace wins -> player 0 leads next
    assert_eq!(state.leader, 0);
    assert_eq!(state.turn, 0);
    assert_eq!(state.round.tricks_won[0], 1);
}

#[test]
fn resolve_trick_multiple_cases() {
    // Create a RoundState with a full trick
    let mut r = RoundState::new();
    r.trump = Some(Suit::Hearts);
    r.trick_lead = Some(Suit::Clubs);
    r.trick_plays = vec![
        (0, Card { suit: Suit::Clubs, rank: Rank::Ten }),
        (1, Card { suit: Suit::Spades, rank: Rank::Ace }),
        (2, Card { suit: Suit::Hearts, rank: Rank::Two }),
        (3, Card { suit: Suit::Hearts, rank: Rank::King }),
    ];
    // With trump hearts, player 3 wins (KH > 2H)
    assert_eq!(resolve_current_trick(&r), Some(3));

    // No trump played: highest of lead
    let mut r2 = RoundState::new();
    r2.trump = Some(Suit::Spades);
    r2.trick_lead = Some(Suit::Diamonds);
    r2.trick_plays = vec![
        (0, Card { suit: Suit::Diamonds, rank: Rank::Nine }),
        (1, Card { suit: Suit::Clubs, rank: Rank::Ace }),
        (2, Card { suit: Suit::Diamonds, rank: Rank::Queen }),
        (3, Card { suit: Suit::Hearts, rank: Rank::Two }),
    ];
    assert_eq!(resolve_current_trick(&r2), Some(2));
}

#[test]
fn scoring_bonus_only_on_exact_bid() {
    let hands = [Vec::new(), Vec::new(), Vec::new(), Vec::new()];
    let mut state = make_state_with_hands(hands, 2, 0);
    // Fake some tallies
    state.round.tricks_won = [2, 1, 0, 0];
    state.round.bids = [Some(2), Some(0), Some(1), Some(0)];
    state.phase = Phase::Scoring;
    apply_round_scoring(&mut state);
    assert_eq!(state.scores_total, [12, 1, 0, 0]);
    assert_eq!(state.phase, Phase::Complete);
}

#[test]
fn happy_path_round_small() {
    // Hand size 3; deterministic hands
    let h0 = parse_cards(&["AS", "KH", "2C"]);
    let h1 = parse_cards(&["TS", "3H", "4C"]);
    let h2 = parse_cards(&["QS", "5D", "6C"]);
    let h3 = parse_cards(&["9S", "7H", "8C"]);
    let mut state = make_state_with_hands([h0, h1, h2, h3], 3, 0);
    // Bidding: p1 wins with 2 against ties by order
    place_bid(&mut state, 0, Bid(1)).unwrap();
    place_bid(&mut state, 1, Bid(2)).unwrap();
    place_bid(&mut state, 2, Bid(2)).unwrap();
    place_bid(&mut state, 3, Bid(1)).unwrap();
    assert_eq!(state.round.winning_bidder, Some(1));
    crate::domain::set_trump(&mut state, 1, Suit::Hearts).unwrap();
    // Trick 1: lead 1
    play_card(&mut state, 1, Card { suit: Suit::Spades, rank: Rank::Ten }).unwrap();
    play_card(&mut state, 2, Card { suit: Suit::Spades, rank: Rank::Queen }).unwrap();
    play_card(&mut state, 3, Card { suit: Suit::Spades, rank: Rank::Nine }).unwrap();
    play_card(&mut state, 0, Card { suit: Suit::Spades, rank: Rank::Ace }).unwrap();
    assert_eq!(state.leader, 0);
    // Trick 2: lead 0
    play_card(&mut state, 0, Card { suit: Suit::Hearts, rank: Rank::King }).unwrap();
    play_card(&mut state, 1, Card { suit: Suit::Hearts, rank: Rank::Three }).unwrap();
    play_card(&mut state, 2, Card { suit: Suit::Diamonds, rank: Rank::Five }).unwrap();
    play_card(&mut state, 3, Card { suit: Suit::Hearts, rank: Rank::Seven }).unwrap();
    assert_eq!(state.leader, 0);
    // Trick 3: lead 0
    play_card(&mut state, 0, Card { suit: Suit::Clubs, rank: Rank::Two }).unwrap();
    play_card(&mut state, 1, Card { suit: Suit::Clubs, rank: Rank::Four }).unwrap();
    play_card(&mut state, 2, Card { suit: Suit::Clubs, rank: Rank::Six }).unwrap();
    play_card(&mut state, 3, Card { suit: Suit::Clubs, rank: Rank::Eight }).unwrap();
    assert_eq!(state.phase, Phase::Scoring);
    apply_round_scoring(&mut state);
    assert_eq!(state.scores_total, [3 + 10, 1, 0, 0]);
}


