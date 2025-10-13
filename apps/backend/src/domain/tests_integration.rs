use super::*;
use crate::domain::bidding::{place_bid, Bid};
use crate::domain::fixtures::CardFixtures;
use crate::domain::scoring::apply_round_scoring;
use crate::domain::tricks::play_card;
use crate::domain::{Card, Rank, Suit, Trump};

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
fn happy_path_round_small() {
    // Hand size 3; deterministic hands
    let h0 = CardFixtures::parse_hardcoded(&["AS", "KH", "2C"]);
    let h1 = CardFixtures::parse_hardcoded(&["TS", "3H", "4C"]);
    let h2 = CardFixtures::parse_hardcoded(&["QS", "5D", "6C"]);
    let h3 = CardFixtures::parse_hardcoded(&["9S", "7H", "8C"]);
    let mut state = make_state_with_hands([h0, h1, h2, h3], 3, 0);
    // Bidding: p1 wins with 2 against ties by order
    place_bid(&mut state, 0, Bid(1)).unwrap();
    place_bid(&mut state, 1, Bid(2)).unwrap();
    place_bid(&mut state, 2, Bid(2)).unwrap();
    place_bid(&mut state, 3, Bid(1)).unwrap();
    assert_eq!(state.round.winning_bidder, Some(1));
    crate::domain::set_trump(&mut state, 1, Trump::Hearts).unwrap();
    // Trick 1: player 0 leads (dealer + 1)
    play_card(
        &mut state,
        0,
        Card {
            suit: Suit::Spades,
            rank: Rank::Ace,
        },
    )
    .unwrap();
    play_card(
        &mut state,
        1,
        Card {
            suit: Suit::Spades,
            rank: Rank::Ten,
        },
    )
    .unwrap();
    play_card(
        &mut state,
        2,
        Card {
            suit: Suit::Spades,
            rank: Rank::Queen,
        },
    )
    .unwrap();
    play_card(
        &mut state,
        3,
        Card {
            suit: Suit::Spades,
            rank: Rank::Nine,
        },
    )
    .unwrap();
    assert_eq!(state.leader, 0);
    // Trick 2: lead 0
    play_card(
        &mut state,
        0,
        Card {
            suit: Suit::Hearts,
            rank: Rank::King,
        },
    )
    .unwrap();
    play_card(
        &mut state,
        1,
        Card {
            suit: Suit::Hearts,
            rank: Rank::Three,
        },
    )
    .unwrap();
    play_card(
        &mut state,
        2,
        Card {
            suit: Suit::Diamonds,
            rank: Rank::Five,
        },
    )
    .unwrap();
    play_card(
        &mut state,
        3,
        Card {
            suit: Suit::Hearts,
            rank: Rank::Seven,
        },
    )
    .unwrap();
    assert_eq!(state.leader, 0);
    // Trick 3: lead 0
    play_card(
        &mut state,
        0,
        Card {
            suit: Suit::Clubs,
            rank: Rank::Two,
        },
    )
    .unwrap();
    play_card(
        &mut state,
        1,
        Card {
            suit: Suit::Clubs,
            rank: Rank::Four,
        },
    )
    .unwrap();
    play_card(
        &mut state,
        2,
        Card {
            suit: Suit::Clubs,
            rank: Rank::Six,
        },
    )
    .unwrap();
    play_card(
        &mut state,
        3,
        Card {
            suit: Suit::Clubs,
            rank: Rank::Eight,
        },
    )
    .unwrap();
    assert_eq!(state.phase, Phase::Scoring);
    apply_round_scoring(&mut state);
    // Player 0 bid 1, won 2 tricks: 2 points (no bonus)
    // Player 1 bid 2, won 0 tricks: 0 points (no bonus)
    // Player 2 bid 2, won 0 tricks: 0 points
    // Player 3 bid 1, won 1 trick: 1 point (exact bid = 10 point bonus)
    assert_eq!(state.scores_total, [2, 0, 0, 11]);
}
