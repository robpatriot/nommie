use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::domain::errors::DomainError;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum Suit {
    Clubs,
    Diamonds,
    Hearts,
    Spades,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum Rank {
    Two,
    Three,
    Four,
    Five,
    Six,
    Seven,
    Eight,
    Nine,
    Ten,
    Jack,
    Queen,
    King,
    Ace,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct Card {
    pub suit: Suit,
    pub rank: Rank,
}

// Note: Ord/Eq on Card is only for stable sorting: suit order C<D<H<S then rank order.
// Do not use for trick resolution or game logic comparisons involving trump/lead.
impl Ord for Card {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match self.suit.cmp(&other.suit) {
            std::cmp::Ordering::Equal => self.rank.cmp(&other.rank),
            ord => ord,
        }
    }
}

impl PartialOrd for Card {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Serialize for Card {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let rank_char = match self.rank {
            Rank::Two => '2',
            Rank::Three => '3',
            Rank::Four => '4',
            Rank::Five => '5',
            Rank::Six => '6',
            Rank::Seven => '7',
            Rank::Eight => '8',
            Rank::Nine => '9',
            Rank::Ten => 'T',
            Rank::Jack => 'J',
            Rank::Queen => 'Q',
            Rank::King => 'K',
            Rank::Ace => 'A',
        };
        let suit_char = match self.suit {
            Suit::Clubs => 'C',
            Suit::Diamonds => 'D',
            Suit::Hearts => 'H',
            Suit::Spades => 'S',
        };
        let s = format!("{rank_char}{suit_char}");
        serializer.serialize_str(&s)
    }
}

impl<'de> Deserialize<'de> for Card {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        parse_card_str(&s).map_err(|e| serde::de::Error::custom(e.to_string()))
    }
}

fn parse_card_str(s: &str) -> Result<Card, DomainError> {
    if s.len() != 2 {
        return Err(DomainError::ParseCard(s.to_string()));
    }
    let mut chars = s.chars();
    let rank_ch = chars.next().unwrap();
    let suit_ch = chars.next().unwrap();
    // Validate via explicit match sets below; allow digit ranks (2-9)
    let rank = match rank_ch {
        '2' => Rank::Two,
        '3' => Rank::Three,
        '4' => Rank::Four,
        '5' => Rank::Five,
        '6' => Rank::Six,
        '7' => Rank::Seven,
        '8' => Rank::Eight,
        '9' => Rank::Nine,
        'T' => Rank::Ten,
        'J' => Rank::Jack,
        'Q' => Rank::Queen,
        'K' => Rank::King,
        'A' => Rank::Ace,
        _ => return Err(DomainError::ParseCard(s.to_string())),
    };
    let suit = match suit_ch {
        'C' => Suit::Clubs,
        'D' => Suit::Diamonds,
        'H' => Suit::Hearts,
        'S' => Suit::Spades,
        _ => return Err(DomainError::ParseCard(s.to_string())),
    };
    Ok(Card { suit, rank })
}

pub fn hand_has_suit(hand: &[Card], suit: Suit) -> bool {
    hand.iter().any(|c| c.suit == suit)
}

pub fn card_beats(a: Card, b: Card, lead: Suit, trump: Suit) -> bool {
    let a_trump = a.suit == trump;
    let b_trump = b.suit == trump;
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

#[cfg(test)]
pub fn parse_cards(tokens: &[&str]) -> Vec<Card> {
    tokens
        .iter()
        .map(|s| serde_json::from_str::<Card>(&format!("\"{s}\"")).expect("valid card token"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serde_roundtrip() {
        let cases = [
            (Rank::Ace, Suit::Spades, "AS"),
            (Rank::Ten, Suit::Diamonds, "TD"),
            (Rank::Three, Suit::Hearts, "3H"),
            (Rank::Nine, Suit::Clubs, "9C"),
        ];
        for (rank, suit, token) in cases {
            let c = Card { suit, rank };
            let s = serde_json::to_string(&c).unwrap();
            assert_eq!(s, format!("\"{token}\""));
            let decoded: Card = serde_json::from_str(&s).unwrap();
            assert_eq!(decoded, c);
        }
    }

    #[test]
    fn rejects_invalid_tokens() {
        for tok in ["1H", "11S", "Ah", "ZZ", "", "10H"] {
            let res: Result<Card, _> = serde_json::from_str(&format!("\"{tok}\""));
            assert!(res.is_err());
        }
    }

    #[test]
    fn test_card_beats_logic() {
        use Rank::*;
        use Suit::*;
        let lead = Hearts;
        let trump = Spades;
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
