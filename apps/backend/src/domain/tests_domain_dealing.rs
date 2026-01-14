//! Integration tests for card dealing logic - simple deterministic tests
//! that verify the public API works correctly for basic cases.

use std::collections::HashSet;

use crate::domain::dealing::deal_hands;
use crate::domain::Card;

/// Deterministic dealing test with known seed
#[test]
fn test_deterministic_dealing_with_known_seed() {
    let seed: u64 = 42;
    let hand_size: u8 = 5;

    let hands1 = deal_hands(4, hand_size, seed).unwrap();
    let hands2 = deal_hands(4, hand_size, seed).unwrap();

    // Same seed produces identical results
    assert_eq!(hands1, hands2, "Same seed must produce identical hands");

    // Verify first few cards are deterministic (regression guard)
    // For seed=42, hand_size=5, player 0's first card should always be the same
    assert!(!hands1[0].is_empty(), "Player 0 should have cards");
    let first_card = hands1[0][0];

    // On subsequent runs with same seed, first card should match
    let hands3 = deal_hands(4, hand_size, seed).unwrap();
    assert_eq!(
        hands3[0][0], first_card,
        "First card for player 0 must be deterministic for seed={seed}"
    );
}

/// Test that different seeds produce different results
#[test]
fn test_different_seeds_produce_different_hands() {
    let hand_size: u8 = 13;

    let hands1 = deal_hands(4, hand_size, 111).unwrap();
    let hands2 = deal_hands(4, hand_size, 222).unwrap();

    // Different seeds should produce different hands (with extremely high probability)
    assert_ne!(
        hands1, hands2,
        "Different seeds should produce different hands"
    );
}

/// Test validation errors for invalid inputs
#[test]
fn test_dealing_validates_player_count() {
    let result = deal_hands(3, 5, 12345);
    assert!(result.is_err(), "Should reject player_count != 4");

    let result = deal_hands(5, 5, 12345);
    assert!(result.is_err(), "Should reject player_count != 4");
}

#[test]
fn test_dealing_validates_hand_size() {
    // Too small
    assert!(
        deal_hands(4, 1, 12345).is_err(),
        "Should reject hand_size < 2"
    );

    // Too large
    assert!(
        deal_hands(4, 14, 12345).is_err(),
        "Should reject hand_size > 13"
    );

    // Valid boundary cases
    assert!(
        deal_hands(4, 2, 12345).is_ok(),
        "Should accept hand_size = 2"
    );
    assert!(
        deal_hands(4, 13, 12345).is_ok(),
        "Should accept hand_size = 13"
    );
}

#[test]
fn test_hands_are_sorted() {
    let hands = deal_hands(4, 10, 99999).unwrap();

    for (i, hand) in hands.iter().enumerate() {
        let mut sorted = hand.clone();
        sorted.sort();
        assert_eq!(hand, &sorted, "Hand {i} must be sorted");
    }
}

#[test]
fn test_no_duplicate_cards_across_hands() {
    let hands = deal_hands(4, 13, 42).unwrap();

    let mut all_cards: Vec<Card> = Vec::new();
    for hand in &hands {
        all_cards.extend(hand.iter().copied());
    }

    // Check uniqueness
    for i in 0..all_cards.len() {
        for j in (i + 1)..all_cards.len() {
            assert_ne!(
                all_cards[i], all_cards[j],
                "Duplicate card found: {:?}",
                all_cards[i]
            );
        }
    }
}

#[test]
fn test_dealing_produces_correct_hand_sizes() {
    let hands = deal_hands(4, 5, 12345).unwrap();

    // Verify each hand has the correct size
    for hand in &hands {
        assert_eq!(hand.len(), 5, "Each hand must have 5 cards");
    }

    // Verify total cards dealt equals player_count * hand_size
    let total: usize = hands.iter().map(|h| h.len()).sum();
    assert_eq!(total, 20, "Total cards dealt must equal 4 * 5");
}

#[test]
fn test_dealing_hand_uniqueness() {
    let hands = deal_hands(4, 3, 999).unwrap();

    // Verify no card appears in multiple hands
    let mut all_cards: Vec<Card> = Vec::new();
    for hand in &hands {
        all_cards.extend(hand.iter());
    }

    let card_set: HashSet<Card> = all_cards.iter().copied().collect();
    assert_eq!(
        card_set.len(),
        all_cards.len(),
        "No card should appear in multiple hands"
    );
}

/// Test suit distribution across many deals to detect bias.
///
/// This test performs a large number of deals and checks if suits
/// are evenly distributed. If the shuffle has bias, certain suits
/// will appear more frequently than expected.
#[test]
fn test_suit_distribution_bias() {
    use std::collections::HashMap;

    use crate::domain::Suit;

    const NUM_DEALS: u32 = 5_000;
    const HAND_SIZE: u8 = 13;

    const NUM_BATCHES: u32 = 10;
    const DEALS_PER_BATCH: u32 = NUM_DEALS / NUM_BATCHES;

    // df=3 (4 suits), alpha=0.001
    const CHI2_CRIT_DF3_ALPHA_0_001: f64 = 16.266;
    // df=9 (4x4), alpha=0.001
    const CHI2_CRIT_DF9_ALPHA_0_001: f64 = 27.877;

    fn mix64(mut x: u64) -> u64 {
        x = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = x;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn chi_square_gof(observed: &[u32], expected: f64) -> f64 {
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

    for batch in 0..NUM_BATCHES {
        // Clippy-friendly grouping (4-hex-digit groups)
        let master_seed = mix64(0x000D_1EAD_BA5E_5EED_u64 ^ (batch as u64));

        // Overall suit counts in this batch
        let mut suit_counts: HashMap<Suit, u32> = HashMap::new();
        suit_counts.insert(Suit::Clubs, 0);
        suit_counts.insert(Suit::Diamonds, 0);
        suit_counts.insert(Suit::Hearts, 0);
        suit_counts.insert(Suit::Spades, 0);

        // Suit distribution by player position in this batch
        let mut suit_by_player: [HashMap<Suit, u32>; 4] = [
            HashMap::new(),
            HashMap::new(),
            HashMap::new(),
            HashMap::new(),
        ];
        for player_suits in &mut suit_by_player {
            player_suits.insert(Suit::Clubs, 0);
            player_suits.insert(Suit::Diamonds, 0);
            player_suits.insert(Suit::Hearts, 0);
            player_suits.insert(Suit::Spades, 0);
        }

        // 4x4 contingency table: rows=player (0..3), cols=suit (C,D,H,S).
        let mut table_4x4 = [[0u32; 4]; 4];

        for i in 0..DEALS_PER_BATCH {
            let deal_index = batch * DEALS_PER_BATCH + i;
            let seed = mix64(master_seed ^ (deal_index as u64));
            let hands = deal_hands(4, HAND_SIZE, seed).unwrap();

            for (player_idx, hand) in hands.iter().enumerate() {
                for card in hand {
                    *suit_counts.get_mut(&card.suit).unwrap() += 1;
                    *suit_by_player[player_idx].get_mut(&card.suit).unwrap() += 1;

                    let suit_col = match card.suit {
                        Suit::Clubs => 0,
                        Suit::Diamonds => 1,
                        Suit::Hearts => 2,
                        Suit::Spades => 3,
                    };
                    table_4x4[player_idx][suit_col] += 1;
                }
            }
        }

        let total_cards = DEALS_PER_BATCH * 4 * HAND_SIZE as u32;
        let expected_per_suit = total_cards as f64 / 4.0;
        let expected_per_player_suit = (DEALS_PER_BATCH * HAND_SIZE as u32) as f64 / 4.0;

        println!(
            "\n=== Suit Distribution Test — batch {}/{} ===",
            batch + 1,
            NUM_BATCHES
        );
        println!("Deals in batch: {}", DEALS_PER_BATCH);
        println!("Total cards dealt: {}", total_cards);
        println!("Expected per suit (overall): {:.2}", expected_per_suit);

        // Overall GOF is usually forced-equal when dealing full decks; only print/assert if not exact.
        let overall_obs = [
            suit_counts[&Suit::Clubs],
            suit_counts[&Suit::Diamonds],
            suit_counts[&Suit::Hearts],
            suit_counts[&Suit::Spades],
        ];
        let overall_chi2 = chi_square_gof(&overall_obs, expected_per_suit);

        let expected_per_suit_u32 = expected_per_suit.round() as u32;
        let overall_is_exact = overall_obs.iter().all(|&x| x == expected_per_suit_u32);
        if !overall_is_exact {
            println!("Overall suit chi² (df=3): {:.4}", overall_chi2);
            assert!(
                overall_chi2 <= CHI2_CRIT_DF3_ALPHA_0_001,
                "Batch {}: overall suit distribution looks biased (chi²={:.4} > {:.3})",
                batch,
                overall_chi2,
                CHI2_CRIT_DF3_ALPHA_0_001
            );
        }

        for (player_idx, player_suits) in suit_by_player.iter().enumerate() {
            let obs = [
                player_suits[&Suit::Clubs],
                player_suits[&Suit::Diamonds],
                player_suits[&Suit::Hearts],
                player_suits[&Suit::Spades],
            ];
            let chi2 = chi_square_gof(&obs, expected_per_player_suit);
            println!("Player {} suit chi² (df=3): {:.4}", player_idx, chi2);

            assert!(
                chi2 <= CHI2_CRIT_DF3_ALPHA_0_001,
                "Batch {}: player {} suit distribution looks biased (chi²={:.4} > {:.3})",
                batch,
                player_idx,
                chi2,
                CHI2_CRIT_DF3_ALPHA_0_001
            );
        }

        let chi2_ind = chi_square_independence_4x4(&table_4x4);
        println!("Player×Suit independence chi² (df=9): {:.4}", chi2_ind);

        assert!(
            chi2_ind <= CHI2_CRIT_DF9_ALPHA_0_001,
            "Batch {}: suit appears dependent on player position (chi²={:.4} > {:.3})",
            batch,
            chi2_ind,
            CHI2_CRIT_DF9_ALPHA_0_001
        );
    }
}

#[test]
fn test_rank_distribution_bias() {
    use crate::domain::Rank;

    const NUM_DEALS: u32 = 5_000;
    const HAND_SIZE: u8 = 13;

    const NUM_BATCHES: u32 = 10;
    const DEALS_PER_BATCH: u32 = NUM_DEALS / NUM_BATCHES;

    // df=12 (13 ranks), alpha=0.001
    const CHI2_CRIT_DF12_ALPHA_0_001: f64 = 32.909;

    // df=3 (4 seats), alpha=0.001 — used for tail allocation chi²
    const CHI2_CRIT_DF3_ALPHA_0_001: f64 = 16.266;

    // Conservative, non-flaky tolerances per batch (500 hands/seat).
    const MAX_ALLOWED_MEAN_SUM_DIFF: f64 = 3.0;
    const MAX_ALLOWED_MEAN_HIGHCARDS_DIFF: f64 = 0.4;

    // Fat-tail checks
    // Variance estimate is noisy; keep this lenient.
    const MAX_ALLOWED_VARIANCE_REL_SPREAD: f64 = 0.30; // (max-min)/min

    fn mix64(mut x: u64) -> u64 {
        x = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = x;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn chi_square_gof(observed: &[u32], expected: f64) -> f64 {
        observed
            .iter()
            .map(|&o| {
                let d = o as f64 - expected;
                (d * d) / expected
            })
            .sum()
    }

    fn rank_index(rank: Rank) -> usize {
        match rank {
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

    fn rank_value(rank: Rank) -> u8 {
        match rank {
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

    fn is_high_card(rank: Rank) -> bool {
        matches!(
            rank,
            Rank::Ten | Rank::Jack | Rank::Queen | Rank::King | Rank::Ace
        )
    }

    fn variance(values: &[u64], mean: f64) -> f64 {
        let n = values.len() as f64;
        values
            .iter()
            .map(|&v| {
                let d = v as f64 - mean;
                d * d
            })
            .sum::<f64>()
            / n
    }

    for batch in 0..NUM_BATCHES {
        let master_seed = mix64(0x52A1_8A53_EEED_BA5E_u64 ^ (batch as u64));

        let mut rank_by_player = [[0u32; 13]; 4];
        let mut total_hand_sum_by_player = [0u64; 4];
        let mut total_high_cards_by_player = [0u32; 4];

        // For exact-k tail selection (no cutoff ties)
        let mut all_hand_sums_with_seat: Vec<(u64, usize)> =
            Vec::with_capacity((DEALS_PER_BATCH as usize) * 4);

        // For variance per seat
        let mut hand_sums_by_player: [Vec<u64>; 4] = std::array::from_fn(|_| Vec::new());

        for i in 0..DEALS_PER_BATCH {
            let deal_index = batch * DEALS_PER_BATCH + i;
            let seed = mix64(master_seed ^ (deal_index as u64));
            let hands = deal_hands(4, HAND_SIZE, seed).unwrap();

            for (player_idx, hand) in hands.iter().enumerate() {
                let mut hand_sum: u64 = 0;
                let mut hand_high: u32 = 0;

                for card in hand {
                    let r = card.rank;
                    rank_by_player[player_idx][rank_index(r)] += 1;

                    hand_sum += rank_value(r) as u64;
                    if is_high_card(r) {
                        hand_high += 1;
                    }
                }

                total_hand_sum_by_player[player_idx] += hand_sum;
                total_high_cards_by_player[player_idx] += hand_high;

                all_hand_sums_with_seat.push((hand_sum, player_idx));
                hand_sums_by_player[player_idx].push(hand_sum);
            }
        }

        println!(
            "\n=== Rank Bias Test — batch {}/{} ===",
            batch + 1,
            NUM_BATCHES
        );
        println!("Deals in batch: {}", DEALS_PER_BATCH);

        // 1) Rank frequency per seat (GOF, df=12)
        let cards_per_player = (DEALS_PER_BATCH * HAND_SIZE as u32) as f64;
        let expected_per_rank_per_player = cards_per_player / 13.0;

        for (player_idx, obs_arr) in rank_by_player.iter().enumerate() {
            let chi2 = chi_square_gof(obs_arr, expected_per_rank_per_player);
            println!("Player {} rank chi² (df=12): {:.4}", player_idx, chi2);

            assert!(
                chi2 <= CHI2_CRIT_DF12_ALPHA_0_001,
                "Batch {}: player {} rank distribution looks biased (chi²={:.4} > {:.3})",
                batch,
                player_idx,
                chi2,
                CHI2_CRIT_DF12_ALPHA_0_001
            );
        }

        // 2) Mean hand sum per seat
        let hands_per_player = DEALS_PER_BATCH as f64;

        let mean_sum_by_player: [f64; 4] =
            std::array::from_fn(|i| total_hand_sum_by_player[i] as f64 / hands_per_player);
        let global_mean_sum = mean_sum_by_player.iter().sum::<f64>() / 4.0;

        println!(
            "Mean hand-sum by player: [104ish] [{:.3}, {:.3}, {:.3}, {:.3}] (global {:.3})",
            mean_sum_by_player[0],
            mean_sum_by_player[1],
            mean_sum_by_player[2],
            mean_sum_by_player[3],
            global_mean_sum
        );

        let max_mean_sum_diff: f64 = mean_sum_by_player
            .iter()
            .map(|&m| (m - global_mean_sum).abs())
            .fold(0.0_f64, f64::max);

        assert!(
            max_mean_sum_diff <= MAX_ALLOWED_MEAN_SUM_DIFF,
            "Batch {}: mean hand strength differs too much by seat (max diff {:.3} > {:.3}). Means: [{:.3}, {:.3}, {:.3}, {:.3}], global {:.3}",
            batch,
            max_mean_sum_diff,
            MAX_ALLOWED_MEAN_SUM_DIFF,
            mean_sum_by_player[0],
            mean_sum_by_player[1],
            mean_sum_by_player[2],
            mean_sum_by_player[3],
            global_mean_sum
        );

        // 3) Mean high-cards per hand per seat (10+)
        let mean_high_by_player: [f64; 4] =
            std::array::from_fn(|i| total_high_cards_by_player[i] as f64 / hands_per_player);
        let global_mean_high = mean_high_by_player.iter().sum::<f64>() / 4.0;

        println!(
            "Mean high-cards/hand (10+): [{:.3}, {:.3}, {:.3}, {:.3}] (global {:.3})",
            mean_high_by_player[0],
            mean_high_by_player[1],
            mean_high_by_player[2],
            mean_high_by_player[3],
            global_mean_high
        );

        let max_mean_high_diff: f64 = mean_high_by_player
            .iter()
            .map(|&m| (m - global_mean_high).abs())
            .fold(0.0_f64, f64::max);

        assert!(
            max_mean_high_diff <= MAX_ALLOWED_MEAN_HIGHCARDS_DIFF,
            "Batch {}: high-card rate differs too much by seat (max diff {:.3} > {:.3}). Means: [{:.3}, {:.3}, {:.3}, {:.3}], global {:.3}",
            batch,
            max_mean_high_diff,
            MAX_ALLOWED_MEAN_HIGHCARDS_DIFF,
            mean_high_by_player[0],
            mean_high_by_player[1],
            mean_high_by_player[2],
            mean_high_by_player[3],
            global_mean_high
        );

        // 4) Fat-tail check A: variance spread by seat
        let variances: [f64; 4] =
            std::array::from_fn(|i| variance(&hand_sums_by_player[i], mean_sum_by_player[i]));

        let min_var = variances.iter().copied().fold(f64::INFINITY, f64::min);
        let max_var = variances.iter().copied().fold(0.0_f64, f64::max);

        println!(
            "Hand-sum variance by player: [{:.2}, {:.2}, {:.2}, {:.2}]",
            variances[0], variances[1], variances[2], variances[3]
        );

        assert!(
            (max_var - min_var) / min_var <= MAX_ALLOWED_VARIANCE_REL_SPREAD,
            "Batch {}: hand-strength variance differs too much by seat (min {:.2}, max {:.2}, rel spread {:.3} > {:.3})",
            batch,
            min_var,
            max_var,
            (max_var - min_var) / min_var,
            MAX_ALLOWED_VARIANCE_REL_SPREAD
        );

        // 5) Fat-tail check B: exact-k tails, and test allocation across seats with chi² (df=3)
        all_hand_sums_with_seat.sort_unstable_by_key(|(sum, _seat)| *sum);

        let n = all_hand_sums_with_seat.len();
        assert!(n > 0, "expected non-empty hand list");

        let k = (n * 5) / 100; // exact bottom/top 5%
        assert!(k > 0, "batch too small to form tails");

        let low_threshold_value = all_hand_sums_with_seat[k - 1].0;
        let high_threshold_value = all_hand_sums_with_seat[n - k].0;

        println!(
            "Exact-k tail thresholds (diagnostic only): weak <= {}, strong >= {} (k={}, n={})",
            low_threshold_value, high_threshold_value, k, n
        );

        let mut weak_counts = [0u32; 4];
        let mut strong_counts = [0u32; 4];

        for &(_sum, seat) in &all_hand_sums_with_seat[..k] {
            weak_counts[seat] += 1;
        }
        for &(_sum, seat) in &all_hand_sums_with_seat[n - k..] {
            strong_counts[seat] += 1;
        }

        let weak_strong_pairs: [(u32, u32); 4] =
            std::array::from_fn(|i| (weak_counts[i], strong_counts[i]));
        println!(
            "Exact-k extreme hands by seat (weak, strong): {:?}",
            weak_strong_pairs
        );

        let expected_per_seat = k as f64 / 4.0;

        let weak_chi2 = chi_square_gof(&weak_counts, expected_per_seat);
        let strong_chi2 = chi_square_gof(&strong_counts, expected_per_seat);

        println!(
            "Tail allocation chi² (df=3): weak {:.4}, strong {:.4}",
            weak_chi2, strong_chi2
        );

        assert!(
            weak_chi2 <= CHI2_CRIT_DF3_ALPHA_0_001,
            "Batch {}: weak-tail allocation looks seat-biased (chi²={:.4} > {:.3}); counts {:?}",
            batch,
            weak_chi2,
            CHI2_CRIT_DF3_ALPHA_0_001,
            weak_counts
        );

        assert!(
            strong_chi2 <= CHI2_CRIT_DF3_ALPHA_0_001,
            "Batch {}: strong-tail allocation looks seat-biased (chi²={:.4} > {:.3}); counts {:?}",
            batch,
            strong_chi2,
            CHI2_CRIT_DF3_ALPHA_0_001,
            strong_counts
        );
    }
}
