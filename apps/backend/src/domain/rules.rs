use std::ops::RangeInclusive;

pub const PLAYERS: usize = 4;
pub const MAX_ROUNDS: u8 = 26;

// Hand-size schedule: 13 → 12 → ... → 2 (FOUR rounds at 2) → 3 → ... → 13
// Total 26 rounds.
pub fn hand_size_for_round(round_no: u8) -> Option<u8> {
    if round_no == 0 || round_no > MAX_ROUNDS {
        return None;
    }
    // Rounds 1..=11: 13 down to 3 (11 steps)
    if round_no <= 11 {
        return Some(13 - (round_no - 1));
    }
    // Rounds 12..=15: four rounds at 2
    if (12..=15).contains(&round_no) {
        return Some(2);
    }
    // Rounds 16..=26: 3 up to 13 (11 steps)
    let step = round_no - 15; // 1..=11
    Some(2 + step)
}

pub fn valid_bid_range(hand_size: u8) -> RangeInclusive<u8> {
    0..=hand_size
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schedule_is_correct() {
        let expected: [u8; 26] = [
            13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, // down to 3
            2, 2, 2, 2, // four rounds at 2
            3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13,
        ];
        for (i, &hs) in expected.iter().enumerate() {
            assert_eq!(hand_size_for_round((i as u8) + 1), Some(hs));
        }
        assert_eq!(hand_size_for_round(0), None);
        assert_eq!(hand_size_for_round(27), None);
    }

    #[test]
    fn bid_range_matches_hand_size() {
        for hs in 0..=13u8 {
            let r = valid_bid_range(hs);
            assert_eq!(*r.start(), 0);
            assert_eq!(*r.end(), hs);
        }
    }
}
