//! Card parsing from string representations (e.g., "AS", "2C")

use std::str::FromStr;

use super::cards_types::{Card, Rank, Suit};
use crate::errors::domain::{DomainError, ValidationKind};

impl FromStr for Card {
    type Err = DomainError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.len() != 2 {
            return Err(DomainError::validation(
                ValidationKind::ParseCard,
                format!("Parse card: {s}"),
            ));
        }
        let mut chars = s.chars();
        let rank_ch = chars.next().ok_or_else(|| {
            DomainError::validation(ValidationKind::ParseCard, format!("Parse card: {s}"))
        })?;
        let suit_ch = chars.next().ok_or_else(|| {
            DomainError::validation(ValidationKind::ParseCard, format!("Parse card: {s}"))
        })?;
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
            _ => {
                return Err(DomainError::validation(
                    ValidationKind::ParseCard,
                    format!("Parse card: {s}"),
                ))
            }
        };
        let suit = match suit_ch {
            'C' => Suit::Clubs,
            'D' => Suit::Diamonds,
            'H' => Suit::Hearts,
            'S' => Suit::Spades,
            _ => {
                return Err(DomainError::validation(
                    ValidationKind::ParseCard,
                    format!("Parse card: {s}"),
                ))
            }
        };
        Ok(Card { suit, rank })
    }
}

/// Non-panicking helper to parse card tokens (e.g., "AS", "2C") into Card instances.
/// Returns Result<Vec<Card>, DomainError> if any token is invalid.
pub fn try_parse_cards<I, S>(tokens: I) -> Result<Vec<Card>, DomainError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    tokens
        .into_iter()
        .map(|s| s.as_ref().parse::<Card>())
        .collect()
}

/// Parse card from stored JSONB format (e.g., suit="CLUBS", rank="TWO").
/// This is used when reconstructing cards from database storage.
pub fn from_stored_format(suit_str: &str, rank_str: &str) -> Result<Card, DomainError> {
    let suit = match suit_str {
        "CLUBS" => Suit::Clubs,
        "DIAMONDS" => Suit::Diamonds,
        "HEARTS" => Suit::Hearts,
        "SPADES" => Suit::Spades,
        _ => {
            return Err(DomainError::validation(
                ValidationKind::ParseCard,
                format!("Invalid suit: {suit_str}"),
            ))
        }
    };

    let rank = match rank_str {
        "TWO" => Rank::Two,
        "THREE" => Rank::Three,
        "FOUR" => Rank::Four,
        "FIVE" => Rank::Five,
        "SIX" => Rank::Six,
        "SEVEN" => Rank::Seven,
        "EIGHT" => Rank::Eight,
        "NINE" => Rank::Nine,
        "TEN" => Rank::Ten,
        "JACK" => Rank::Jack,
        "QUEEN" => Rank::Queen,
        "KING" => Rank::King,
        "ACE" => Rank::Ace,
        _ => {
            return Err(DomainError::validation(
                ValidationKind::ParseCard,
                format!("Invalid rank: {rank_str}"),
            ))
        }
    };

    Ok(Card { suit, rank })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_stored_format() {
        // Test all suits
        assert_eq!(
            from_stored_format("CLUBS", "ACE").unwrap(),
            Card {
                suit: Suit::Clubs,
                rank: Rank::Ace
            }
        );
        assert_eq!(
            from_stored_format("DIAMONDS", "KING").unwrap(),
            Card {
                suit: Suit::Diamonds,
                rank: Rank::King
            }
        );
        assert_eq!(
            from_stored_format("HEARTS", "TWO").unwrap(),
            Card {
                suit: Suit::Hearts,
                rank: Rank::Two
            }
        );
        assert_eq!(
            from_stored_format("SPADES", "TEN").unwrap(),
            Card {
                suit: Suit::Spades,
                rank: Rank::Ten
            }
        );

        // Test parsing failures
        assert!(from_stored_format("INVALID", "ACE").is_err());
        assert!(from_stored_format("CLUBS", "INVALID").is_err());
        assert!(from_stored_format("clubs", "ACE").is_err()); // lowercase should fail
        assert!(from_stored_format("CLUBS", "ace").is_err()); // lowercase should fail
    }

    #[test]
    fn test_from_str_parsing() {
        // Test successful parsing
        assert_eq!(
            "AS".parse::<Card>().unwrap(),
            Card {
                suit: Suit::Spades,
                rank: Rank::Ace
            }
        );
        assert_eq!(
            "TD".parse::<Card>().unwrap(),
            Card {
                suit: Suit::Diamonds,
                rank: Rank::Ten
            }
        );
        assert_eq!(
            "9C".parse::<Card>().unwrap(),
            Card {
                suit: Suit::Clubs,
                rank: Rank::Nine
            }
        );
        assert_eq!(
            "2H".parse::<Card>().unwrap(),
            Card {
                suit: Suit::Hearts,
                rank: Rank::Two
            }
        );

        // Test parsing failures
        assert!("1H".parse::<Card>().is_err()); // invalid rank
        assert!("11S".parse::<Card>().is_err()); // too long
        assert!("Ah".parse::<Card>().is_err()); // lowercase suit
        assert!("ZZ".parse::<Card>().is_err()); // invalid rank and suit
        assert!("".parse::<Card>().is_err()); // empty string
        assert!("10H".parse::<Card>().is_err()); // too long
    }

    #[test]
    fn test_try_parse_cards() {
        // Test successful parsing
        let result = try_parse_cards(["AS", "TD", "9C"]);
        assert!(result.is_ok());
        let cards = result.unwrap();
        assert_eq!(cards.len(), 3);
        assert_eq!(
            cards[0],
            Card {
                suit: Suit::Spades,
                rank: Rank::Ace
            }
        );
        assert_eq!(
            cards[1],
            Card {
                suit: Suit::Diamonds,
                rank: Rank::Ten
            }
        );
        assert_eq!(
            cards[2],
            Card {
                suit: Suit::Clubs,
                rank: Rank::Nine
            }
        );

        // Test parsing failure
        let result = try_parse_cards(["AS", "1H", "9C"]);
        assert!(result.is_err());
    }

    #[test]
    fn rejects_invalid_tokens() {
        for tok in ["1H", "11S", "Ah", "ZZ", "", "10H"] {
            let res: Result<Card, _> = serde_json::from_str(&format!("\"{tok}\""));
            assert!(res.is_err());
        }
    }
}
