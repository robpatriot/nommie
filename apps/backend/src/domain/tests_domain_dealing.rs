//! Integration tests for card dealing logic and fairness properties.
//!
//! These tests aim to cover the full “threat model” for biased dealing:
//! - Seat bias (player position effects)
//! - Suit/rank marginal bias
//! - Short-hand rounds + undealt-card bias
//! - “Human feels” patterns (long suits, voids) skew by seat
//! - Shuffle structure / clumping (deck adjacency)
//! - Basic invariants (no duplicates, correct sizes, permutation sanity)

use std::collections::HashSet;

use crate::domain::dealing::deal_hands;
use crate::domain::{Card, Rank, Suit};

// -------------------------
// Shared helpers
// -------------------------

fn mix32(master_seed: u64, deal_index: u64) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"nommie/test/mix32/v1");
    hasher.update(&master_seed.to_le_bytes());
    hasher.update(&deal_index.to_le_bytes());
    *hasher.finalize().as_bytes()
}

fn suit_col(s: Suit) -> usize {
    match s {
        Suit::Clubs => 0,
        Suit::Diamonds => 1,
        Suit::Hearts => 2,
        Suit::Spades => 3,
    }
}

fn rank_idx(r: Rank) -> usize {
    match r {
        Rank::Two => 0,
        Rank::Three => 1,
        Rank::Four => 2,
        Rank::Five => 3,
        Rank::Six => 4,
        Rank::Seven => 5,
        Rank::Eight => 6,
        Rank::Nine => 7,
        Rank::Ten => 8,
        Rank::Jack => 9,
        Rank::Queen => 10,
        Rank::King => 11,
        Rank::Ace => 12,
    }
}

fn rank_value(r: Rank) -> u8 {
    match r {
        Rank::Two => 2,
        Rank::Three => 3,
        Rank::Four => 4,
        Rank::Five => 5,
        Rank::Six => 6,
        Rank::Seven => 7,
        Rank::Eight => 8,
        Rank::Nine => 9,
        Rank::Ten => 10,
        Rank::Jack => 11,
        Rank::Queen => 12,
        Rank::King => 13,
        Rank::Ace => 14,
    }
}

fn chi_square_gof_u32(observed: &[u32], expected: f64) -> f64 {
    observed
        .iter()
        .map(|&o| {
            let d = o as f64 - expected;
            (d * d) / expected
        })
        .sum()
}

fn chi_square_independence_4x4(table: &[[u32; 4]; 4]) -> f64 {
    let row_sum: [u32; 4] = std::array::from_fn(|r| table[r].iter().sum());
    let col_sum: [u32; 4] = std::array::from_fn(|c| table.iter().map(|row| row[c]).sum());
    let total: u32 = row_sum.iter().sum();
    let total_f = total as f64;

    table
        .iter()
        .enumerate()
        .map(|(r, row)| {
            row.iter()
                .enumerate()
                .map(|(c, &obs)| {
                    let expected = (row_sum[r] as f64) * (col_sum[c] as f64) / total_f;
                    let d = obs as f64 - expected;
                    (d * d) / expected
                })
                .sum::<f64>()
        })
        .sum()
}

// Mirror of production deck generation (only used to compute undealt + deck-structure metrics).
// This is intentionally kept in tests because production deal_hands discards undealt cards and
// sorts hands, so we need visibility into the shuffled deck for some fairness checks.
fn full_deck() -> Vec<Card> {
    let suits = [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades];
    let ranks = [
        Rank::Two,
        Rank::Three,
        Rank::Four,
        Rank::Five,
        Rank::Six,
        Rank::Seven,
        Rank::Eight,
        Rank::Nine,
        Rank::Ten,
        Rank::Jack,
        Rank::Queen,
        Rank::King,
        Rank::Ace,
    ];

    let mut deck = Vec::with_capacity(52);
    for s in suits {
        for r in ranks {
            deck.push(Card { suit: s, rank: r });
        }
    }
    deck
}

fn shuffled_deck(seed: [u8; 32]) -> Vec<Card> {
    use rand::seq::SliceRandom;
    use rand::SeedableRng;
    use rand_chacha::ChaCha20Rng;

    let mut deck = full_deck();
    let mut rng = ChaCha20Rng::from_seed(seed);
    deck.shuffle(&mut rng);
    deck
}

fn deal_and_undealt(hand_size: u8, seed: [u8; 32]) -> ([Vec<Card>; 4], Vec<Card>) {
    let hands = deal_hands(4, hand_size, seed).expect("deal_hands failed");
    let deck = shuffled_deck(seed);
    let dealt_n = 4usize * hand_size as usize;
    let undealt = deck[dealt_n..].to_vec();
    (hands, undealt)
}

fn max_suit_len_and_voids(hand: &[Card]) -> (u8, u8) {
    let mut counts = [0u8; 4];
    for c in hand {
        counts[suit_col(c.suit)] += 1;
    }
    let max_len = *counts.iter().max().unwrap();
    let voids = counts.iter().filter(|&&x| x == 0).count() as u8;
    (max_len, voids)
}

fn count_same_suit_adjacencies(deck: &[Card]) -> u32 {
    deck.windows(2).filter(|w| w[0].suit == w[1].suit).count() as u32
}

// -------------------------
// Basic API / invariants
// -------------------------

#[test]
fn test_deterministic_dealing_with_known_seed() {
    let seed = mix32(42, 0);
    let hand_size: u8 = 5;

    let hands1 = deal_hands(4, hand_size, seed).unwrap();
    let hands2 = deal_hands(4, hand_size, seed).unwrap();

    assert_eq!(hands1, hands2, "Same seed must produce identical hands");
    assert!(!hands1[0].is_empty(), "Player 0 should have cards");
    assert_eq!(
        hands1[0][0], hands2[0][0],
        "First card for player 0 must be deterministic for a given seed"
    );
}

#[test]
fn test_different_seeds_produce_different_hands() {
    let hand_size: u8 = 13;

    let hands1 = deal_hands(4, hand_size, mix32(111, 0)).unwrap();
    let hands2 = deal_hands(4, hand_size, mix32(222, 0)).unwrap();

    assert_ne!(
        hands1, hands2,
        "Different seeds should produce different hands (overwhelming probability)"
    );
}

#[test]
fn test_dealing_validates_player_count() {
    assert!(deal_hands(3, 5, [0u8; 32]).is_err());
    assert!(deal_hands(5, 5, [0u8; 32]).is_err());
}

#[test]
fn test_dealing_validates_hand_size() {
    assert!(deal_hands(4, 1, [0u8; 32]).is_err());
    assert!(deal_hands(4, 14, [0u8; 32]).is_err());
    assert!(deal_hands(4, 2, [0u8; 32]).is_ok());
    assert!(deal_hands(4, 13, [0u8; 32]).is_ok());
}

#[test]
fn test_hands_are_sorted() {
    let hands = deal_hands(4, 10, mix32(99999, 0)).unwrap();
    for (i, hand) in hands.iter().enumerate() {
        let mut sorted = hand.clone();
        sorted.sort();
        assert_eq!(hand, &sorted, "Hand {i} must be sorted");
    }
}

#[test]
fn test_no_duplicate_cards_across_hands() {
    let hands = deal_hands(4, 13, mix32(42, 0)).unwrap();

    let mut all_cards: Vec<Card> = Vec::new();
    for hand in &hands {
        all_cards.extend(hand.iter().copied());
    }

    let set: HashSet<Card> = all_cards.iter().copied().collect();
    assert_eq!(
        set.len(),
        all_cards.len(),
        "No duplicate cards should appear across hands"
    );
}

#[test]
fn test_dealing_produces_correct_hand_sizes() {
    let hands = deal_hands(4, 5, mix32(12345, 0)).unwrap();

    for hand in &hands {
        assert_eq!(hand.len(), 5, "Each hand must have 5 cards");
    }

    let total: usize = hands.iter().map(|h| h.len()).sum();
    assert_eq!(total, 20, "Total cards dealt must equal 4 * 5");
}

/// Structural invariant: for any seed/hand size, dealt + undealt should form a permutation of the deck.
/// This catches “state leakage” and duplication/missing-card bugs.
#[test]
fn test_dealt_plus_undealt_is_full_deck_permutation() {
    let hand_sizes = [13u8, 8u8, 2u8];
    for &hand_size in &hand_sizes {
        for i in 0..200u64 {
            let seed = mix32(0xD00D_F00D_BA5E_0001, i);
            let (hands, undealt) = deal_and_undealt(hand_size, seed);

            let mut all: Vec<Card> = Vec::with_capacity(52);
            for hand in &hands {
                all.extend(hand.iter().copied());
            }
            all.extend(undealt.iter().copied());

            let set: HashSet<Card> = all.iter().copied().collect();
            assert_eq!(set.len(), 52, "Expected exactly 52 unique cards");
            assert_eq!(all.len(), 52, "Expected exactly 52 total cards");
        }
    }
}

// -------------------------
// Fairness tests (threat model coverage)
// -------------------------

// Conservative chi-square critical values (alpha=0.001).
const CHI2_DF3_A001: f64 = 16.266; // df=3 (4 bins / 4 suits / 4 seats)
const CHI2_DF9_A001: f64 = 27.877; // df=9 (4x4 independence)
const CHI2_DF12_A001: f64 = 32.909; // df=12 (13 ranks)

const NUM_DEALS: u32 = 5_000;
const NUM_BATCHES: u32 = 10;
const DEALS_PER_BATCH: u32 = NUM_DEALS / NUM_BATCHES;

const HAND_SIZES: &[u8] = &[13, 8, 2];

/// Suit bias + seat dependence, across multiple hand sizes, including undealt cards for truncation rounds.
#[test]
fn test_fairness_suit_distribution_across_hand_sizes_including_undealt() {
    for &hand_size in HAND_SIZES {
        for batch in 0..NUM_BATCHES {
            let master_seed = 0x000D_1EAD_BA5E_5EED_u64 ^ (hand_size as u64) ^ (batch as u64);

            // Per-seat suit counts + contingency
            let mut suit_by_seat = [[0u32; 4]; 4];
            let mut table_4x4 = [[0u32; 4]; 4];

            // Undealt suit counts (only meaningful when hand_size < 13)
            let mut undealt_suits = [0u32; 4];

            for i in 0..DEALS_PER_BATCH {
                let deal_index = batch * DEALS_PER_BATCH + i;
                let seed = mix32(master_seed, deal_index as u64);

                let (hands, undealt) = deal_and_undealt(hand_size, seed);

                for (seat, hand) in hands.iter().enumerate() {
                    for c in hand {
                        let col = suit_col(c.suit);
                        suit_by_seat[seat][col] += 1;
                        table_4x4[seat][col] += 1;
                    }
                }
                for c in &undealt {
                    undealt_suits[suit_col(c.suit)] += 1;
                }
            }

            // Per-seat suit GOF (df=3)
            let cards_per_seat = (DEALS_PER_BATCH * hand_size as u32) as f64;
            let expected_per_suit_per_seat = cards_per_seat / 4.0;

            for (seat, counts) in suit_by_seat.iter().enumerate() {
                let chi2 = chi_square_gof_u32(counts, expected_per_suit_per_seat);
                assert!(
                    chi2 <= CHI2_DF3_A001,
                    "hand_size {} batch {} seat {}: suit GOF chi2 {:.4} too high; obs {:?}",
                    hand_size,
                    batch,
                    seat,
                    chi2,
                    counts
                );
            }

            // Seat×Suit independence (df=9)
            let chi2_ind = chi_square_independence_4x4(&table_4x4);
            assert!(
                chi2_ind <= CHI2_DF9_A001,
                "hand_size {} batch {}: seat×suit dependence chi2 {:.4} too high",
                hand_size,
                batch,
                chi2_ind
            );

            // Undealt suit GOF when truncating (df=3)
            if hand_size < 13 {
                let undealt_n = 52u32 - 4u32 * hand_size as u32;
                let expected_undealt_per_suit = (DEALS_PER_BATCH * undealt_n) as f64 / 4.0;
                let chi2_undealt = chi_square_gof_u32(&undealt_suits, expected_undealt_per_suit);

                assert!(
                    chi2_undealt <= CHI2_DF3_A001,
                    "hand_size {} batch {}: undealt suit bias chi2 {:.4} too high; obs {:?}",
                    hand_size,
                    batch,
                    chi2_undealt,
                    undealt_suits
                );
            }
        }
    }
}

/// Rank bias across seats, and undealt-card rank bias for truncation rounds.
/// Also includes “strongest seat” uniformity checks (joint structure / seat bias detector).
#[test]
fn test_fairness_rank_distribution_and_strongest_seat_across_hand_sizes_including_undealt() {
    for &hand_size in HAND_SIZES {
        for batch in 0..NUM_BATCHES {
            let master_seed = 0x52A1_8A53_EEED_BA5E_u64 ^ (hand_size as u64) ^ (batch as u64);

            let mut rank_by_seat = [[0u32; 13]; 4];
            let mut undealt_ranks = [0u32; 13];
            let mut strongest_seat_counts = [0u32; 4];

            for i in 0..DEALS_PER_BATCH {
                let deal_index = batch * DEALS_PER_BATCH + i;
                let seed = mix32(master_seed, deal_index as u64);

                let (hands, undealt) = deal_and_undealt(hand_size, seed);

                let mut sums = [0u64; 4];
                for (seat, hand) in hands.iter().enumerate() {
                    let mut sum = 0u64;
                    for c in hand {
                        rank_by_seat[seat][rank_idx(c.rank)] += 1;
                        sum += rank_value(c.rank) as u64;
                    }
                    sums[seat] = sum;
                }

                let (best_seat, _) = sums
                    .iter()
                    .enumerate()
                    .max_by_key(|(_i, &s)| s)
                    .expect("non-empty sums");
                strongest_seat_counts[best_seat] += 1;

                for c in &undealt {
                    undealt_ranks[rank_idx(c.rank)] += 1;
                }
            }

            // Rank GOF per seat (df=12)
            let cards_per_seat = (DEALS_PER_BATCH * hand_size as u32) as f64;
            let expected_per_rank_per_seat = cards_per_seat / 13.0;

            for (seat, counts) in rank_by_seat.iter().enumerate() {
                let chi2 = chi_square_gof_u32(counts, expected_per_rank_per_seat);
                assert!(
                    chi2 <= CHI2_DF12_A001,
                    "hand_size {} batch {} seat {}: rank GOF chi2 {:.4} too high",
                    hand_size,
                    batch,
                    seat,
                    chi2
                );
            }

            // Strongest seat should be ~uniform (df=3)
            let expected_strongest = DEALS_PER_BATCH as f64 / 4.0;
            let chi2_strongest = chi_square_gof_u32(&strongest_seat_counts, expected_strongest);
            assert!(
                chi2_strongest <= CHI2_DF3_A001,
                "hand_size {} batch {}: strongest-seat allocation chi2 {:.4} too high; counts {:?}",
                hand_size,
                batch,
                chi2_strongest,
                strongest_seat_counts
            );

            // Undealt rank GOF when truncating (df=12)
            if hand_size < 13 {
                let undealt_n = 52u32 - 4u32 * hand_size as u32;
                let expected_undealt_per_rank = (DEALS_PER_BATCH * undealt_n) as f64 / 13.0;
                let chi2_undealt = chi_square_gof_u32(&undealt_ranks, expected_undealt_per_rank);

                assert!(
                    chi2_undealt <= CHI2_DF12_A001,
                    "hand_size {} batch {}: undealt rank bias chi2 {:.4} too high",
                    hand_size,
                    batch,
                    chi2_undealt
                );
            }
        }
    }
}

/// “Human perception” checks (13-card hands only):
/// - max suit length >= 6 happens often enough that humans notice it; we check it is not seat-skewed
///   using a simple rate-spread tolerance.
/// - max suit length >= 7 is rarer; we check seat allocation with chi² (stronger signal, less noisy).
/// - voids are checked with a robust 2-bin (0 vs >=1) seat-skew test.
///
/// This test is intentionally focused on HAND_SIZE=13 because "long suits" perception is loudest there,
/// and because short-hand rounds are already covered by suit/rank + undealt tests.
#[test]
fn test_fairness_long_suits_and_voids_by_seat() {
    const HAND_SIZE: u8 = 13;

    // Conservative chi-square critical (df=3, alpha=0.001)
    const CHI2_DF3_A001: f64 = 16.266;

    // For 6+ max suit, we use a tolerance on per-seat rates within each batch.
    // DEALS_PER_BATCH is 500 in your config; this is intentionally lenient to avoid flakiness.
    const MAX_ALLOWED_RATE_SPREAD_6PLUS: f64 = 0.08; // max(rate) - min(rate)

    for batch in 0..NUM_BATCHES {
        let master_seed = 0xF00D_CAFE_BEEF_0001_u64 ^ (batch as u64);

        let mut count_max6plus_by_seat = [0u32; 4];
        let mut count_max7plus_by_seat = [0u32; 4];

        // Voids: 0 vs >=1
        let mut void0_by_seat = [0u32; 4];
        let mut void1plus_by_seat = [0u32; 4];

        for i in 0..DEALS_PER_BATCH {
            let deal_index = batch * DEALS_PER_BATCH + i;
            let seed = mix32(master_seed, deal_index as u64);

            let hands = deal_hands(4, HAND_SIZE, seed).expect("deal_hands failed");

            for (seat, hand) in hands.iter().enumerate() {
                let (max_len, voids) = max_suit_len_and_voids(hand);

                if max_len >= 6 {
                    count_max6plus_by_seat[seat] += 1;
                }
                if max_len >= 7 {
                    count_max7plus_by_seat[seat] += 1;
                }

                if voids == 0 {
                    void0_by_seat[seat] += 1;
                } else {
                    void1plus_by_seat[seat] += 1;
                }
            }
        }

        // ---- 6+ max suit: tolerance spread on rates (seat skew detector, low flake risk)
        let denom = DEALS_PER_BATCH as f64;
        let rates6: [f64; 4] = std::array::from_fn(|s| count_max6plus_by_seat[s] as f64 / denom);

        let min_r = rates6.iter().copied().fold(f64::INFINITY, f64::min);
        let max_r = rates6.iter().copied().fold(f64::NEG_INFINITY, f64::max);

        assert!(
            (max_r - min_r) <= MAX_ALLOWED_RATE_SPREAD_6PLUS,
            "batch {}: max-suit>=6 rate looks seat-skewed: rates {:?}, spread {:.4} > {:.4} (counts {:?})",
            batch,
            rates6,
            (max_r - min_r),
            MAX_ALLOWED_RATE_SPREAD_6PLUS,
            count_max6plus_by_seat
        );

        // ---- 7+ max suit: chi-square allocation across seats (stronger signal)
        let total7: u32 = count_max7plus_by_seat.iter().sum();
        if total7 > 0 {
            let expected = total7 as f64 / 4.0;
            let chi2 = chi_square_gof_u32(&count_max7plus_by_seat, expected);

            assert!(
                chi2 <= CHI2_DF3_A001,
                "batch {}: max-suit>=7 allocation looks seat-biased (chi2 {:.4} > {:.3}); counts {:?}, total {}",
                batch,
                chi2,
                CHI2_DF3_A001,
                count_max7plus_by_seat,
                total7
            );
        }

        // ---- Voids: chi-square allocation for "has a void" (>=1 void) across seats (df=3)
        let total_void1plus: u32 = void1plus_by_seat.iter().sum();
        if total_void1plus > 0 {
            let expected = total_void1plus as f64 / 4.0;
            let chi2 = chi_square_gof_u32(&void1plus_by_seat, expected);

            assert!(
                chi2 <= CHI2_DF3_A001,
                "batch {}: void(>=1) allocation looks seat-biased (chi2 {:.4} > {:.3}); counts {:?}, total {}",
                batch,
                chi2,
                CHI2_DF3_A001,
                void1plus_by_seat,
                total_void1plus
            );
        }

        // (Optional diagnostic: run with --nocapture)
        // println!(
        //     "batch {}: >=6 {:?} (rates {:?}), >=7 {:?}, void>=1 {:?}",
        //     batch, count_max6plus_by_seat, rates6, count_max7plus_by_seat, void1plus_by_seat
        // );
    }
}

/// Shuffle structure / clumping: same-suit adjacency count in the shuffled deck.
///
/// Fixes vs previous version:
/// - Correct expected mean for a real 52-card deck permutation: E = 12.0
///   (since P(adjacent same suit) = 12/51 and there are 51 adjacencies).
/// - Use empirical variance across deals in the batch (avoids iid/binomial under-variance).
#[test]
fn test_fairness_shuffled_deck_same_suit_adjacency_not_extreme() {
    // Very conservative. If this fails repeatedly, something is genuinely off.
    const MAX_ABS_Z: f64 = 8.0;

    // Exact expected mean for standard deck:
    // p = sum_s (k_s/n)*((k_s-1)/(n-1)) with k_s=13, n=52 => 12/51
    // E[#adj] = (n-1)*p = 51*(12/51) = 12
    const EXPECTED_MEAN: f64 = 12.0;

    for batch in 0..NUM_BATCHES {
        let master_seed = 0xABCD_1234_DEAD_BEEF_u64 ^ (batch as u64);

        // Collect adjacency counts so we can compute empirical std dev.
        let mut vals: Vec<f64> = Vec::with_capacity(DEALS_PER_BATCH as usize);

        for i in 0..DEALS_PER_BATCH {
            let deal_index = batch * DEALS_PER_BATCH + i;
            let seed = mix32(master_seed, deal_index as u64);
            let deck = shuffled_deck(seed);
            vals.push(count_same_suit_adjacencies(&deck) as f64);
        }

        let n = vals.len() as f64;
        let mean = vals.iter().sum::<f64>() / n;

        // Sample variance
        let var = {
            let mut s = 0.0;
            for &v in &vals {
                let d = v - mean;
                s += d * d;
            }
            // unbiased estimator; if n==1 (won't happen), avoid div-by-zero
            if vals.len() > 1 {
                s / (n - 1.0)
            } else {
                0.0
            }
        };

        let sd = var.sqrt();
        // Standard error of the mean
        let se = if sd > 0.0 { sd / n.sqrt() } else { 1e-9 };

        let z = (mean - EXPECTED_MEAN) / se;

        assert!(
            z.abs() <= MAX_ABS_Z,
            "batch {}: adjacency mean looks extreme (mean {:.3}, expected {:.3}, sd {:.3}, se {:.4}, z {:.3})",
            batch,
            mean,
            EXPECTED_MEAN,
            sd,
            se,
            z
        );
    }
}
