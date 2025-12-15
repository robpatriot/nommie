use crate::domain::{Suit, Trump};
use crate::errors::domain::{DomainError, ValidationKind};

#[test]
fn trump_conversions() {
    // From<Suit> for Trump
    assert_eq!(Trump::from(Suit::Clubs), Trump::Clubs);
    assert_eq!(Trump::from(Suit::Diamonds), Trump::Diamonds);
    assert_eq!(Trump::from(Suit::Hearts), Trump::Hearts);
    assert_eq!(Trump::from(Suit::Spades), Trump::Spades);

    // TryFrom<Trump> for Suit - success cases
    use std::convert::TryInto;
    assert_eq!(Trump::Clubs.try_into(), Ok(Suit::Clubs));
    assert_eq!(Trump::Diamonds.try_into(), Ok(Suit::Diamonds));
    assert_eq!(Trump::Hearts.try_into(), Ok(Suit::Hearts));
    assert_eq!(Trump::Spades.try_into(), Ok(Suit::Spades));

    // TryFrom<Trump> for Suit - NoTrump fails
    let result: Result<Suit, _> = Trump::NoTrumps.try_into();
    assert_eq!(
        result,
        Err(DomainError::validation(
            ValidationKind::InvalidTrumpConversion,
            "Cannot convert NoTrumps to Suit"
        ))
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
        "\"NO_TRUMP\""
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
        serde_json::from_str::<Trump>("\"NO_TRUMP\"").unwrap(),
        Trump::NoTrumps
    );
}
