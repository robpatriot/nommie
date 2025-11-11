//! Snapshot API tests covering all game phases.

use crate::domain::bidding::{place_bid, set_trump, Bid};
use crate::domain::fixtures::CardFixtures;
use crate::domain::rules::PLAYERS;
use crate::domain::snapshot::{snapshot, PhaseSnapshot, SeatPublic};
use crate::domain::state::{GameState, Phase, RoundState};
use crate::domain::tricks::play_card;
use crate::domain::{Card, Rank, Suit, Trump};

/// Build a minimal GameState in Init phase.
fn build_init_state() -> GameState {
    GameState {
        phase: Phase::Init,
        round_no: 0,
        hand_size: 0,
        hands: [vec![], vec![], vec![], vec![]],
        turn_start: 0,
        turn: 0,
        leader: 0,
        trick_no: 0,
        scores_total: [0, 0, 0, 0],
        round: RoundState::empty(),
    }
}

/// Start a round with a given round number and initial hands.
fn start_round(round_no: u8, hands: [Vec<Card>; 4]) -> GameState {
    let hand_size = hands[0].len() as u8;
    let dealer = if round_no == 0 {
        0
    } else {
        (round_no - 1) % PLAYERS as u8
    };
    let turn_start = ((dealer as usize + 1) % PLAYERS) as u8; // left-of-dealer

    GameState {
        phase: Phase::Bidding,
        round_no,
        hand_size,
        hands,
        turn_start,
        turn: turn_start,
        leader: turn_start,
        trick_no: 0,
        scores_total: [0, 0, 0, 0],
        round: RoundState::empty(),
    }
}

#[test]
fn init_snapshot_smoke() {
    let state = build_init_state();
    let snap = snapshot(&state);

    assert_eq!(snap.game.round_no, 0);
    assert_eq!(snap.game.dealer, 0);
    assert_eq!(
        snap.game.seating,
        [
            SeatPublic::empty(0),
            SeatPublic::empty(1),
            SeatPublic::empty(2),
            SeatPublic::empty(3)
        ]
    );
    assert_eq!(snap.game.scores_total, [0, 0, 0, 0]);

    match snap.phase {
        PhaseSnapshot::Init => {}
        _ => panic!("Expected Init phase"),
    }
}

#[test]
fn bidding_snapshot_legals() {
    // Round 1: dealer=0, turn_start=1 (left-of-dealer), hand_size=13
    let hands = [
        CardFixtures::parse_hardcoded(&[
            "AC", "2C", "3C", "4C", "5C", "6C", "7C", "8C", "9C", "TC", "JC", "QC", "KC",
        ]),
        CardFixtures::parse_hardcoded(&[
            "AD", "2D", "3D", "4D", "5D", "6D", "7D", "8D", "9D", "TD", "JD", "QD", "KD",
        ]),
        CardFixtures::parse_hardcoded(&[
            "AH", "2H", "3H", "4H", "5H", "6H", "7H", "8H", "9H", "TH", "JH", "QH", "KH",
        ]),
        CardFixtures::parse_hardcoded(&[
            "AS", "2S", "3S", "4S", "5S", "6S", "7S", "8S", "9S", "TS", "JS", "QS", "KS",
        ]),
    ];
    let state = start_round(1, hands);
    let snap = snapshot(&state);

    match snap.phase {
        PhaseSnapshot::Bidding(b) => {
            assert_eq!(b.to_act, 1); // left-of-dealer
            assert_eq!(b.bids, [None, None, None, None]);
            assert_eq!(b.min_bid, 0);
            assert_eq!(b.max_bid, 13);
            assert_eq!(b.round.hand_size, 13);
        }
        _ => panic!("Expected Bidding phase"),
    }
}

#[test]
fn trump_select_snapshot() {
    // Start round and complete bidding
    let hands = [
        CardFixtures::parse_hardcoded(&["AC", "2C", "3C"]),
        CardFixtures::parse_hardcoded(&["AD", "2D", "3D"]),
        CardFixtures::parse_hardcoded(&["AH", "2H", "3H"]),
        CardFixtures::parse_hardcoded(&["AS", "2S", "3S"]),
    ];
    let mut state = start_round(1, hands);

    // Place bids: player 1 bids 0, player 2 bids 1, player 3 bids 0, player 0 bids 0
    // Player 2 should win with bid of 1
    place_bid(&mut state, 1, Bid(0)).unwrap();
    place_bid(&mut state, 2, Bid(1)).unwrap();
    place_bid(&mut state, 3, Bid(0)).unwrap();
    place_bid(&mut state, 0, Bid(0)).unwrap();

    assert_eq!(state.phase, Phase::TrumpSelect);

    let snap = snapshot(&state);

    match snap.phase {
        PhaseSnapshot::TrumpSelect(t) => {
            assert_eq!(t.to_act, 2); // bid winner
            assert_eq!(t.round.bid_winner, Some(2));
            assert!(t.allowed_trumps.contains(&Trump::Clubs));
            assert!(t.allowed_trumps.contains(&Trump::Diamonds));
            assert!(t.allowed_trumps.contains(&Trump::Hearts));
            assert!(t.allowed_trumps.contains(&Trump::Spades));
            assert!(t.allowed_trumps.contains(&Trump::NoTrump));
        }
        _ => panic!("Expected TrumpSelect phase"),
    }
}

#[test]
fn trick_snapshot_legals() {
    // Start round, complete bidding, select trump, play some cards
    let hands = [
        CardFixtures::parse_hardcoded(&["AC", "2C", "3C"]),
        CardFixtures::parse_hardcoded(&["AD", "2D", "3D"]),
        CardFixtures::parse_hardcoded(&["AH", "2H", "3H"]),
        CardFixtures::parse_hardcoded(&["AS", "2S", "3S"]),
    ];
    let mut state = start_round(1, hands);

    // Complete bidding: player 2 wins with bid of 2
    place_bid(&mut state, 1, Bid(0)).unwrap();
    place_bid(&mut state, 2, Bid(2)).unwrap();
    place_bid(&mut state, 3, Bid(0)).unwrap();
    place_bid(&mut state, 0, Bid(0)).unwrap();

    // Select trump
    set_trump(&mut state, 2, Trump::Spades).unwrap();

    // Now in Trick phase
    assert!(matches!(state.phase, Phase::Trick { .. }));

    let snap = snapshot(&state);

    match snap.phase {
        PhaseSnapshot::Trick(t) => {
            assert_eq!(t.trick_no, 1);
            assert_eq!(t.leader, 1); // player to left of dealer (dealer=0, so 1) leads
            assert_eq!(t.to_act, 1);
            assert_eq!(t.current_trick.len(), 0);
            // Playable should be all cards in player 1's hand (no lead yet)
            assert_eq!(t.playable.len(), 3);
        }
        _ => panic!("Expected Trick phase"),
    }

    // Play first card (player 1 leads with AD)
    play_card(
        &mut state,
        1,
        Card {
            suit: Suit::Diamonds,
            rank: Rank::Ace,
        },
    )
    .unwrap();

    let snap = snapshot(&state);
    match snap.phase {
        PhaseSnapshot::Trick(t) => {
            assert_eq!(t.to_act, 2); // next player after 1
            assert_eq!(t.current_trick.len(), 1);
            // Player 2 has only Hearts, so is void in Diamonds and can play any card
            assert_eq!(t.playable.len(), 3);
            assert!(t.playable.iter().all(|c| c.suit == Suit::Hearts));
        }
        _ => panic!("Expected Trick phase"),
    }

    // Play second card (player 2 plays AH, void in diamonds)
    play_card(
        &mut state,
        2,
        Card {
            suit: Suit::Hearts,
            rank: Rank::Ace,
        },
    )
    .unwrap();

    // Play third card (player 3 has no diamonds, can play any spade)
    let snap = snapshot(&state);
    match snap.phase {
        PhaseSnapshot::Trick(t) => {
            assert_eq!(t.to_act, 3);
            // Player 3 is void in diamonds, can play any card
            assert_eq!(t.playable.len(), 3);
        }
        _ => panic!("Expected Trick phase"),
    }
}

#[test]
fn trick_snapshot_second_trick_turn_rotation() {
    // Hands configured so player 3 wins first trick with trump and leads the second trick
    let hands = [
        CardFixtures::parse_hardcoded(&["3D", "4C"]),
        CardFixtures::parse_hardcoded(&["AD", "5C"]),
        CardFixtures::parse_hardcoded(&["2D", "6C"]),
        CardFixtures::parse_hardcoded(&["AS", "7C"]),
    ];

    let mut state = start_round(1, hands);

    // Bidding: player 1 wins and selects Spades as trump
    place_bid(&mut state, 1, Bid(2)).unwrap();
    place_bid(&mut state, 2, Bid(0)).unwrap();
    place_bid(&mut state, 3, Bid(0)).unwrap();
    place_bid(&mut state, 0, Bid(0)).unwrap();
    set_trump(&mut state, 1, Trump::Spades).unwrap();

    // First trick: player 1 leads diamonds, player 3 wins with spade (trump)
    play_card(
        &mut state,
        1,
        Card {
            suit: Suit::Diamonds,
            rank: Rank::Ace,
        },
    )
    .unwrap();
    play_card(
        &mut state,
        2,
        Card {
            suit: Suit::Diamonds,
            rank: Rank::Two,
        },
    )
    .unwrap();
    play_card(
        &mut state,
        3,
        Card {
            suit: Suit::Spades,
            rank: Rank::Ace,
        },
    )
    .unwrap();
    play_card(
        &mut state,
        0,
        Card {
            suit: Suit::Diamonds,
            rank: Rank::Three,
        },
    )
    .unwrap();

    // After trick resolution, player 3 should lead and be next to act
    let snap = snapshot(&state);
    match snap.phase {
        PhaseSnapshot::Trick(t) => {
            assert_eq!(t.current_trick.len(), 0);
            assert_eq!(t.leader, 3);
            assert_eq!(t.to_act, 3);
        }
        _ => panic!("Expected Trick phase after first trick resolution"),
    }

    // Player 3 leads the second trick
    play_card(
        &mut state,
        3,
        Card {
            suit: Suit::Clubs,
            rank: Rank::Seven,
        },
    )
    .unwrap();

    let snap = snapshot(&state);
    match snap.phase {
        PhaseSnapshot::Trick(t) => {
            assert_eq!(t.current_trick.len(), 1);
            assert_eq!(t.leader, 3);
            assert_eq!(t.to_act, 0);
        }
        _ => panic!("Expected Trick phase during second trick"),
    }
}

#[test]
fn scoring_snapshot() {
    // Complete a full round with 2 cards per player
    let hands = [
        CardFixtures::parse_hardcoded(&["AC", "2C"]),
        CardFixtures::parse_hardcoded(&["AD", "2D"]),
        CardFixtures::parse_hardcoded(&["AH", "2H"]),
        CardFixtures::parse_hardcoded(&["AS", "2S"]),
    ];
    let mut state = start_round(1, hands);

    // Bidding: player 1 bids 1, others bid 0
    place_bid(&mut state, 1, Bid(1)).unwrap();
    place_bid(&mut state, 2, Bid(0)).unwrap();
    place_bid(&mut state, 3, Bid(0)).unwrap();
    place_bid(&mut state, 0, Bid(0)).unwrap();

    // Trump selection
    set_trump(&mut state, 1, Trump::Diamonds).unwrap();

    // Trick 1: player 1 leads AD (wins as trump)
    play_card(
        &mut state,
        1,
        Card {
            suit: Suit::Diamonds,
            rank: Rank::Ace,
        },
    )
    .unwrap();
    play_card(
        &mut state,
        2,
        Card {
            suit: Suit::Hearts,
            rank: Rank::Ace,
        },
    )
    .unwrap();
    play_card(
        &mut state,
        3,
        Card {
            suit: Suit::Spades,
            rank: Rank::Ace,
        },
    )
    .unwrap();
    play_card(
        &mut state,
        0,
        Card {
            suit: Suit::Clubs,
            rank: Rank::Ace,
        },
    )
    .unwrap();

    // Player 1 won, leads trick 2
    // Trick 2: player 1 leads 2D
    play_card(
        &mut state,
        1,
        Card {
            suit: Suit::Diamonds,
            rank: Rank::Two,
        },
    )
    .unwrap();
    play_card(
        &mut state,
        2,
        Card {
            suit: Suit::Hearts,
            rank: Rank::Two,
        },
    )
    .unwrap();
    play_card(
        &mut state,
        3,
        Card {
            suit: Suit::Spades,
            rank: Rank::Two,
        },
    )
    .unwrap();
    play_card(
        &mut state,
        0,
        Card {
            suit: Suit::Clubs,
            rank: Rank::Two,
        },
    )
    .unwrap();

    // After all tricks, should be in Scoring phase
    assert_eq!(state.phase, Phase::Scoring);

    let snap = snapshot(&state);
    match snap.phase {
        PhaseSnapshot::Scoring(s) => {
            // Player 0 bid 0, won 0 -> exact, gets 0+10=10
            // Player 1 bid 1, won 2 -> not exact, gets 2
            // Player 2 bid 0, won 0 -> exact, gets 0+10=10
            // Player 3 bid 0, won 0 -> exact, gets 0+10=10
            assert_eq!(s.round_scores[0], 10); // exact bid bonus
            assert_eq!(s.round_scores[1], 2); // 2 tricks, no bonus
            assert_eq!(s.round_scores[2], 10); // exact bid bonus
            assert_eq!(s.round_scores[3], 10); // exact bid bonus
        }
        _ => panic!("Expected Scoring phase"),
    }
}

#[test]
fn complete_and_gameover_snapshots() {
    // Create a state in Complete phase
    let hands = [
        CardFixtures::parse_hardcoded(&["AC"]),
        CardFixtures::parse_hardcoded(&["AD"]),
        CardFixtures::parse_hardcoded(&["AH"]),
        CardFixtures::parse_hardcoded(&["AS"]),
    ];
    let mut state = start_round(1, hands);

    // Complete bidding, trump, and single trick
    place_bid(&mut state, 1, Bid(0)).unwrap();
    place_bid(&mut state, 2, Bid(0)).unwrap();
    place_bid(&mut state, 3, Bid(1)).unwrap();
    place_bid(&mut state, 0, Bid(0)).unwrap();

    set_trump(&mut state, 3, Trump::NoTrump).unwrap();

    // Play single trick - player 1 leads (dealer+1)
    play_card(
        &mut state,
        1,
        Card {
            suit: Suit::Diamonds,
            rank: Rank::Ace,
        },
    )
    .unwrap();
    play_card(
        &mut state,
        2,
        Card {
            suit: Suit::Hearts,
            rank: Rank::Ace,
        },
    )
    .unwrap();
    play_card(
        &mut state,
        3,
        Card {
            suit: Suit::Spades,
            rank: Rank::Ace,
        },
    )
    .unwrap();
    play_card(
        &mut state,
        0,
        Card {
            suit: Suit::Clubs,
            rank: Rank::Ace,
        },
    )
    .unwrap();

    // Should be in Scoring
    assert_eq!(state.phase, Phase::Scoring);

    // Apply scoring manually to reach Complete
    crate::domain::scoring::apply_round_scoring(&mut state);
    assert_eq!(state.phase, Phase::Complete);

    let snap = snapshot(&state);
    match snap.phase {
        PhaseSnapshot::Complete(c) => {
            assert_eq!(c.round.tricks_won[1], 1); // Player 1 won with AD
            assert_eq!(c.round.bid_winner, Some(3));
        }
        _ => panic!("Expected Complete phase"),
    }

    // Move to GameOver by setting phase directly (simulating end of game)
    state.phase = Phase::GameOver;
    state.round_no = 26;

    let snap = snapshot(&state);
    match snap.phase {
        PhaseSnapshot::GameOver => {
            // Totals preserved - player 1 won the trick but bid 0, so 1 point (no bonus)
            // Player 3 bid 1 but won 0 tricks, so 0 points
            assert_eq!(snap.game.scores_total[1], 1); // 1 trick, bid was 0 so no bonus
        }
        _ => panic!("Expected GameOver phase"),
    }
}
