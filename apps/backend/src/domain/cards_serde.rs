//! Serialization and deserialization for card types

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::cards_types::{Card, Rank, Suit, Trump};

// Suit serde
impl Serialize for Suit {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = match self {
            Suit::Clubs => "CLUBS",
            Suit::Diamonds => "DIAMONDS",
            Suit::Hearts => "HEARTS",
            Suit::Spades => "SPADES",
        };
        serializer.serialize_str(s)
    }
}

impl<'de> Deserialize<'de> for Suit {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match s.as_str() {
            "CLUBS" => Ok(Suit::Clubs),
            "DIAMONDS" => Ok(Suit::Diamonds),
            "HEARTS" => Ok(Suit::Hearts),
            "SPADES" => Ok(Suit::Spades),
            _ => Err(serde::de::Error::custom(format!("Invalid suit: {s}"))),
        }
    }
}

// Trump serde
impl Serialize for Trump {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = match self {
            Trump::Clubs => "CLUBS",
            Trump::Diamonds => "DIAMONDS",
            Trump::Hearts => "HEARTS",
            Trump::Spades => "SPADES",
            Trump::NoTrumps => "NO_TRUMPS",
        };
        serializer.serialize_str(s)
    }
}

impl<'de> Deserialize<'de> for Trump {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match s.as_str() {
            "CLUBS" => Ok(Trump::Clubs),
            "DIAMONDS" => Ok(Trump::Diamonds),
            "HEARTS" => Ok(Trump::Hearts),
            "SPADES" => Ok(Trump::Spades),
            "NO_TRUMPS" => Ok(Trump::NoTrumps),
            _ => Err(serde::de::Error::custom(format!("Invalid trump: {s}"))),
        }
    }
}

// Card serde (compact 2-character format like "AS", "2C")
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
        s.parse::<Card>()
            .map_err(|e| serde::de::Error::custom(e.to_string()))
    }
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
    fn suit_serde() {
        // Test SCREAMING_SNAKE_CASE serialization for Suit
        assert_eq!(serde_json::to_string(&Suit::Clubs).unwrap(), "\"CLUBS\"");
        assert_eq!(
            serde_json::to_string(&Suit::Diamonds).unwrap(),
            "\"DIAMONDS\""
        );
        assert_eq!(serde_json::to_string(&Suit::Hearts).unwrap(), "\"HEARTS\"");
        assert_eq!(serde_json::to_string(&Suit::Spades).unwrap(), "\"SPADES\"");

        // Test deserialization
        assert_eq!(
            serde_json::from_str::<Suit>("\"CLUBS\"").unwrap(),
            Suit::Clubs
        );
        assert_eq!(
            serde_json::from_str::<Suit>("\"DIAMONDS\"").unwrap(),
            Suit::Diamonds
        );
        assert_eq!(
            serde_json::from_str::<Suit>("\"HEARTS\"").unwrap(),
            Suit::Hearts
        );
        assert_eq!(
            serde_json::from_str::<Suit>("\"SPADES\"").unwrap(),
            Suit::Spades
        );
    }

    #[test]
    fn trump_serde() {
        // Test SCREAMING_SNAKE_CASE serialization
        assert_eq!(serde_json::to_string(&Trump::Clubs).unwrap(), "\"CLUBS\"");
        assert_eq!(
            serde_json::to_string(&Trump::Diamonds).unwrap(),
            "\"DIAMONDS\""
        );
        assert_eq!(serde_json::to_string(&Trump::Hearts).unwrap(), "\"HEARTS\"");
        assert_eq!(serde_json::to_string(&Trump::Spades).unwrap(), "\"SPADES\"");
        assert_eq!(
            serde_json::to_string(&Trump::NoTrumps).unwrap(),
            "\"NO_TRUMPS\""
        );

        // Test deserialization
        assert_eq!(
            serde_json::from_str::<Trump>("\"CLUBS\"").unwrap(),
            Trump::Clubs
        );
        assert_eq!(
            serde_json::from_str::<Trump>("\"DIAMONDS\"").unwrap(),
            Trump::Diamonds
        );
        assert_eq!(
            serde_json::from_str::<Trump>("\"HEARTS\"").unwrap(),
            Trump::Hearts
        );
        assert_eq!(
            serde_json::from_str::<Trump>("\"SPADES\"").unwrap(),
            Trump::Spades
        );
        assert_eq!(
            serde_json::from_str::<Trump>("\"NO_TRUMPS\"").unwrap(),
            Trump::NoTrumps
        );
    }
}
