use crate::domain::bidding::{place_bid, set_trump, Bid};
use crate::domain::scoring::apply_round_scoring;
use crate::domain::state::{Phase, PlayerId};
use crate::domain::test_state_helpers::{make_game_state, MakeGameStateArgs};
use crate::domain::tricks::play_card;
use crate::domain::{Card, Rank, Suit, Trump};

fn parse_cards(tokens: &[&str]) -> Vec<Card> {
    tokens
        .iter()
        .map(|t| t.parse::<Card>().expect("hardcoded valid card token"))
        .collect()
}
#[test]
fn happy_path_round_small() {
    // Hand size 3; deterministic hands
    let h0 = parse_cards(&["AS", "KH", "2C"]);
    let h1 = parse_cards(&["TS", "3H", "4C"]);
    let h2 = parse_cards(&["QS", "5D", "6C"]);
    let h3 = parse_cards(&["9S", "7H", "8C"]);

    let hand_size = 3;
    let turn_start: PlayerId = 0;

    // New model: dealer anchors the round; round-start seat is derived as next_player(dealer).
    let dealer: PlayerId = ((turn_start + 3) % 4) as PlayerId;

    let mut state = make_game_state(
        [h0, h1, h2, h3],
        MakeGameStateArgs {
            phase: Phase::Bidding,
            round_no: Some(1),
            hand_size: Some(hand_size),
            dealer: Some(dealer),

            // In Bidding, "turn" is who must act (place a bid).
            turn: Some(turn_start),

            // Bidding usually doesn't need leader; set if your domain expects it.
            leader: None,

            // If your tests care about this being 0, keep it; otherwise None is fine for Bidding.
            trick_no: Some(0),

            ..Default::default()
        },
    );
    // Bidding: p1 wins with 2 against ties by order
    place_bid(&mut state, 0, Bid(1)).unwrap();
    place_bid(&mut state, 1, Bid(2)).unwrap();
    place_bid(&mut state, 2, Bid(2)).unwrap();
    place_bid(&mut state, 3, Bid(1)).unwrap();
    assert_eq!(state.round.winning_bidder, Some(1));
    set_trump(&mut state, 1, Trump::Hearts).unwrap();
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
    assert_eq!(state.leader, Some(0));
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
    assert_eq!(state.leader, Some(0));
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
