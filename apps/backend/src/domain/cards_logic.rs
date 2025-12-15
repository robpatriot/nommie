//! Card game logic: checking suits in hands, comparing card strength

use super::cards_types::{Card, Suit, Trump};

pub fn hand_has_suit(hand: &[Card], suit: Suit) -> bool {
    hand.iter().any(|c| c.suit == suit)
}

pub fn card_beats(a: Card, b: Card, lead: Suit, trump: Trump) -> bool {
    match trump {
        Trump::NoTrumps => {
            // No trump: only lead-suit cards can beat others
            let a_follows = a.suit == lead;
            let b_follows = b.suit == lead;
            if a_follows && !b_follows {
                return true;
            }
            if b_follows && !a_follows {
                return false;
            }
            if a_follows && b_follows {
                return a.rank > b.rank;
            }
            false
        }
        Trump::Clubs | Trump::Diamonds | Trump::Hearts | Trump::Spades => {
            let trump_suit = match trump {
                Trump::Clubs => Suit::Clubs,
                Trump::Diamonds => Suit::Diamonds,
                Trump::Hearts => Suit::Hearts,
                Trump::Spades => Suit::Spades,
                Trump::NoTrumps => unreachable!(), // This arm is already handled above
            };
            let a_trump = a.suit == trump_suit;
            let b_trump = b.suit == trump_suit;
            if a_trump && !b_trump {
                return true;
            }
            if b_trump && !a_trump {
                return false;
            }
            // Same trump status
            if a_trump && b_trump {
                return a.rank > b.rank;
            }
            // No trump: compare only if following lead
            let a_follows = a.suit == lead;
            let b_follows = b.suit == lead;
            if a_follows && !b_follows {
                return true;
            }
            if b_follows && !a_follows {
                return false;
            }
            if a_follows && b_follows {
                return a.rank > b.rank;
            }
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::cards_types::Rank;

    #[test]
    fn test_card_beats_logic() {
        use Rank::*;
        use Suit::*;
        let lead = Hearts;
        let trump = Trump::Spades;
        let ah = Card {
            suit: Hearts,
            rank: Ace,
        };
        let kh = Card {
            suit: Hearts,
            rank: King,
        };
        let ts = Card {
            suit: Spades,
            rank: Ten,
        };
        let th = Card {
            suit: Hearts,
            rank: Ten,
        };
        let td = Card {
            suit: Diamonds,
            rank: Ten,
        };

        assert!(card_beats(ah, kh, lead, trump));
        assert!(!card_beats(th, ah, lead, trump));
        assert!(card_beats(ts, ah, lead, trump));
        assert!(card_beats(ts, td, lead, trump));
        assert!(card_beats(ah, td, lead, trump));
    }

    #[test]
    fn test_card_beats_no_trump() {
        use Rank::*;
        use Suit::*;
        let lead = Hearts;
        let trump = Trump::NoTrumps;
        let ah = Card {
            suit: Hearts,
            rank: Ace,
        };
        let kh = Card {
            suit: Hearts,
            rank: King,
        };
        let ts = Card {
            suit: Spades,
            rank: Ten,
        };
        let th = Card {
            suit: Hearts,
            rank: Ten,
        };
        let td = Card {
            suit: Diamonds,
            rank: Ten,
        };

        // In no trump, only lead suit cards can beat others
        assert!(card_beats(ah, kh, lead, trump)); // both hearts, ace beats king
        assert!(!card_beats(th, ah, lead, trump)); // both hearts, ten doesn't beat ace
        assert!(!card_beats(ts, ah, lead, trump)); // spades can't beat hearts (lead suit)
        assert!(!card_beats(ts, td, lead, trump)); // neither is lead suit
        assert!(card_beats(ah, td, lead, trump)); // hearts beats diamonds (lead vs non-lead)
    }

    #[test]
    fn test_card_beats_trump_beats_lead() {
        // "Trump beats lead": lead=Hearts, trump=Spades; (2♠) must beat (A♥)
        let two_spades = Card {
            suit: Suit::Spades,
            rank: Rank::Two,
        };
        let ace_hearts = Card {
            suit: Suit::Hearts,
            rank: Rank::Ace,
        };
        assert!(card_beats(
            two_spades,
            ace_hearts,
            Suit::Hearts,
            Trump::Spades
        ));
    }

    #[test]
    fn test_card_beats_notrump_lead_wins_over_offsuit() {
        // "NoTrump: lead wins over off-suit": lead=Hearts, trump=NO_TRUMP; (A♠) must NOT beat (2♥)
        let ace_spades = Card {
            suit: Suit::Spades,
            rank: Rank::Ace,
        };
        let two_hearts = Card {
            suit: Suit::Hearts,
            rank: Rank::Two,
        };
        assert!(!card_beats(
            ace_spades,
            two_hearts,
            Suit::Hearts,
            Trump::NoTrumps
        ));
    }

    #[test]
    fn test_card_beats_within_lead_rank_decides() {
        // "Within lead, rank decides": lead=Diamonds, trump=Hearts; (Q♦) beats (J♦)
        let queen_diamonds = Card {
            suit: Suit::Diamonds,
            rank: Rank::Queen,
        };
        let jack_diamonds = Card {
            suit: Suit::Diamonds,
            rank: Rank::Jack,
        };
        assert!(card_beats(
            queen_diamonds,
            jack_diamonds,
            Suit::Diamonds,
            Trump::Hearts
        ));
    }

    #[test]
    fn test_card_beats_within_trump_rank_decides() {
        // "Within trump, rank decides": lead=Clubs, trump=Spades; (A♠) beats (Q♠)
        let ace_spades = Card {
            suit: Suit::Spades,
            rank: Rank::Ace,
        };
        let queen_spades = Card {
            suit: Suit::Spades,
            rank: Rank::Queen,
        };
        assert!(card_beats(
            ace_spades,
            queen_spades,
            Suit::Clubs,
            Trump::Spades
        ));
    }

    #[test]
    fn test_hand_has_suit() {
        let hand = vec![
            Card {
                suit: Suit::Clubs,
                rank: Rank::Two,
            },
            Card {
                suit: Suit::Diamonds,
                rank: Rank::Ace,
            },
        ];
        assert!(hand_has_suit(&hand, Suit::Clubs));
        assert!(!hand_has_suit(&hand, Suit::Hearts));
    }
}
