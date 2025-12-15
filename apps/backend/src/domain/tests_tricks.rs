use crate::domain::bidding::{place_bid, set_trump, Bid};
use crate::domain::state::{GameState, Phase, PlayerId, RoundState};
use crate::domain::tricks::{legal_moves, play_card, resolve_current_trick};
use crate::domain::{Card, Rank, Suit, Trump};
use crate::errors::domain::{DomainError, ValidationKind};

fn parse_cards(tokens: &[&str]) -> Vec<Card> {
    tokens
        .iter()
        .map(|t| t.parse::<Card>().expect("hardcoded valid card token"))
        .collect()
}

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
fn legal_moves_follow_lead() {
    // Hands for a small test
    let h0 = parse_cards(&["AS", "KH", "2C"]);
    let h1 = parse_cards(&["TS", "3H", "4C"]);
    let h2 = parse_cards(&["QS", "5D", "6C"]);
    let h3 = parse_cards(&["9S", "7H", "8C"]);
    let mut state = make_state_with_hands([h0, h1, h2, h3], 3, 0);
    for p in 0..4 {
        assert!(place_bid(&mut state, p, Bid(0), None).is_ok());
    }
    set_trump(&mut state, 0, Trump::Hearts).unwrap();
    // First to play can play any
    let lm0 = legal_moves(&state, 0);
    assert_eq!(lm0.len(), 3);
    // Play AS -> lead Spades
    play_card(
        &mut state,
        0,
        Card {
            suit: Suit::Spades,
            rank: Rank::Ace,
        },
    )
    .unwrap();
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
    for p in 0..4 {
        place_bid(&mut state, p, Bid(0), None).unwrap();
    }
    set_trump(&mut state, 0, Trump::Hearts).unwrap();
    // Out of turn
    assert_eq!(
        play_card(
            &mut state,
            1,
            Card {
                suit: Suit::Spades,
                rank: Rank::Ten
            }
        )
        .unwrap_err(),
        DomainError::validation(ValidationKind::OutOfTurn, "Out of turn")
    );
    // Not in hand
    assert_eq!(
        play_card(
            &mut state,
            0,
            Card {
                suit: Suit::Diamonds,
                rank: Rank::Ace
            }
        )
        .unwrap_err(),
        DomainError::validation(ValidationKind::CardNotInHand, "Card not in hand")
    );
    // Play trick fully
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
    // Highest trump is none in trick; lead spades so Ace wins -> player 0 leads next
    assert_eq!(state.leader, 0);
    assert_eq!(state.turn, 0);
    assert_eq!(state.round.tricks_won[0], 1);
}

#[test]
fn resolve_trick_multiple_cases() {
    // Create a RoundState with a full trick
    let mut r = RoundState::empty();
    r.trump = Some(Trump::Hearts);
    r.trick_lead = Some(Suit::Clubs);
    r.trick_plays = vec![
        (
            0,
            Card {
                suit: Suit::Clubs,
                rank: Rank::Ten,
            },
        ),
        (
            1,
            Card {
                suit: Suit::Spades,
                rank: Rank::Ace,
            },
        ),
        (
            2,
            Card {
                suit: Suit::Hearts,
                rank: Rank::Two,
            },
        ),
        (
            3,
            Card {
                suit: Suit::Hearts,
                rank: Rank::King,
            },
        ),
    ];
    // With trump hearts, player 3 wins (KH > 2H)
    assert_eq!(resolve_current_trick(&r), Some(3));

    // No trump played: highest of lead
    let mut r2 = RoundState::empty();
    r2.trump = Some(Trump::Spades);
    r2.trick_lead = Some(Suit::Diamonds);
    r2.trick_plays = vec![
        (
            0,
            Card {
                suit: Suit::Diamonds,
                rank: Rank::Nine,
            },
        ),
        (
            1,
            Card {
                suit: Suit::Clubs,
                rank: Rank::Ace,
            },
        ),
        (
            2,
            Card {
                suit: Suit::Diamonds,
                rank: Rank::Queen,
            },
        ),
        (
            3,
            Card {
                suit: Suit::Hearts,
                rank: Rank::Two,
            },
        ),
    ];
    assert_eq!(resolve_current_trick(&r2), Some(2));
}

#[test]
fn trick_resolution_no_trump() {
    // With NoTrumps, only the lead suit matters
    let mut r = RoundState::empty();
    r.trump = Some(Trump::NoTrumps);
    r.trick_lead = Some(Suit::Diamonds);
    r.trick_plays = vec![
        (
            0,
            Card {
                suit: Suit::Diamonds,
                rank: Rank::Nine,
            },
        ),
        (
            1,
            Card {
                suit: Suit::Spades,
                rank: Rank::Ace,
            },
        ), // Highest card, but wrong suit
        (
            2,
            Card {
                suit: Suit::Diamonds,
                rank: Rank::Queen,
            },
        ),
        (
            3,
            Card {
                suit: Suit::Hearts,
                rank: Rank::Ace,
            },
        ), // Another high card, wrong suit
    ];
    // Player 2 wins with QD (highest of lead suit)
    assert_eq!(resolve_current_trick(&r), Some(2));

    // Another case: all follow lead suit
    let mut r2 = RoundState::empty();
    r2.trump = Some(Trump::NoTrumps);
    r2.trick_lead = Some(Suit::Clubs);
    r2.trick_plays = vec![
        (
            0,
            Card {
                suit: Suit::Clubs,
                rank: Rank::Two,
            },
        ),
        (
            1,
            Card {
                suit: Suit::Clubs,
                rank: Rank::Ten,
            },
        ),
        (
            2,
            Card {
                suit: Suit::Clubs,
                rank: Rank::King,
            },
        ),
        (
            3,
            Card {
                suit: Suit::Clubs,
                rank: Rank::Ace,
            },
        ),
    ];
    // Player 3 wins with AC (highest of lead suit)
    assert_eq!(resolve_current_trick(&r2), Some(3));
}

#[test]
fn resolve_trick_trump_arrives_late_and_wins() {
    // "Trump arrives late and wins": lead=Diamonds, trump=Spades; plays:
    // 9♦, K♦, 2♠, A♦ → player who played 2♠ wins
    let mut r = RoundState::empty();
    r.trump = Some(Trump::Spades);
    r.trick_lead = Some(Suit::Diamonds);
    r.trick_plays = vec![
        (
            0,
            Card {
                suit: Suit::Diamonds,
                rank: Rank::Nine,
            },
        ),
        (
            1,
            Card {
                suit: Suit::Diamonds,
                rank: Rank::King,
            },
        ),
        (
            2,
            Card {
                suit: Suit::Spades,
                rank: Rank::Two,
            },
        ),
        (
            3,
            Card {
                suit: Suit::Diamonds,
                rank: Rank::Ace,
            },
        ),
    ];

    // Verify trick_lead invariant
    assert_eq!(r.trick_plays[0].1.suit, r.trick_lead.unwrap());

    // Player 2 wins with 2♠ (trump beats lead)
    assert_eq!(resolve_current_trick(&r), Some(2));
}

#[test]
fn resolve_trick_multiple_trumps_highest_trump_wins() {
    // "Multiple trumps: highest trump wins": lead=Hearts, trump=Spades; plays
    // include Q♠, 3♠, A♠ → A♠ wins
    let mut r = RoundState::empty();
    r.trump = Some(Trump::Spades);
    r.trick_lead = Some(Suit::Hearts);
    r.trick_plays = vec![
        (
            0,
            Card {
                suit: Suit::Hearts,
                rank: Rank::Two,
            },
        ),
        (
            1,
            Card {
                suit: Suit::Spades,
                rank: Rank::Queen,
            },
        ),
        (
            2,
            Card {
                suit: Suit::Spades,
                rank: Rank::Three,
            },
        ),
        (
            3,
            Card {
                suit: Suit::Spades,
                rank: Rank::Ace,
            },
        ),
    ];

    // Verify trick_lead invariant
    assert_eq!(r.trick_plays[0].1.suit, r.trick_lead.unwrap());

    // Player 3 wins with A♠ (highest trump)
    assert_eq!(resolve_current_trick(&r), Some(3));
}

#[test]
fn resolve_trick_notrump_only_lead_matters() {
    // "NoTrumps: only lead matters even if off-suit ranks are higher": lead=Clubs,
    // trump=NO_TRUMPS; only one clubs card vs three off-suits; the lone clubs card wins
    let mut r = RoundState::empty();
    r.trump = Some(Trump::NoTrumps);
    r.trick_lead = Some(Suit::Clubs);
    r.trick_plays = vec![
        (
            0,
            Card {
                suit: Suit::Clubs,
                rank: Rank::Two,
            },
        ),
        (
            1,
            Card {
                suit: Suit::Spades,
                rank: Rank::Ace,
            },
        ),
        (
            2,
            Card {
                suit: Suit::Diamonds,
                rank: Rank::King,
            },
        ),
        (
            3,
            Card {
                suit: Suit::Hearts,
                rank: Rank::Queen,
            },
        ),
    ];

    // Verify trick_lead invariant
    assert_eq!(r.trick_plays[0].1.suit, r.trick_lead.unwrap());

    // Player 0 wins with 2♣ (only clubs card, lead suit)
    assert_eq!(resolve_current_trick(&r), Some(0));
}
