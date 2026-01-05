//! Reckoner — deterministic, bid-targeted AI with tempo-aware trick taking.
//!
//! Design goals:
//! - Always legal (use engine `legal_*()`).
//! - Deterministic (no RNG).
//! - Self-score focused: optimize landing *own* bid exactly (+10 bonus).
//! - Uses RoundMemory only as fuzzy evidence (void detection + coarse high-card tracking).
//! - Stronger play: tempo-aware “win security” and accidental-win avoidance.
//!
//! This file is intended to run side-by-side with Strategic.

use std::collections::HashMap;

use crate::ai::{AiError, AiPlayer};
use crate::domain::player_view::{CurrentRoundInfo, GameHistory};
use crate::domain::round_memory::{PlayMemory, RoundMemory};
use crate::domain::{card_beats, Card, GameContext, Rank, Suit, Trump};

#[derive(Debug, Clone, Copy)]
struct Policy {
    need: u8,
    avoid: bool,
    pressure: f32,
    tricks_remaining: u8,
    endgame: bool,
    must_win_out: bool,
}

#[derive(Debug, Clone, Copy)]
struct Expect {
    trump: f32,
    cash: f32,
    length: f32,
    ruff: f32,
}

impl Expect {
    fn total(&self) -> f32 {
        self.trump + self.cash + self.length + self.ruff
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WinCertainty {
    No,
    Fragile,
    Likely,
    Sure,
}

#[derive(Clone)]
pub struct Reckoner {
    _seed: Option<u64>, // reserved, unused: keep determinism
}

impl Reckoner {
    pub const NAME: &'static str = "Reckoner";
    pub const VERSION: &'static str = "0.1.0";

    pub fn new(seed: Option<u64>) -> Self {
        Self { _seed: seed }
    }

    // ----------------------------
    // Core policy
    // ----------------------------

    fn compute_policy(state: &CurrentRoundInfo) -> Policy {
        let my_seat = state.player_seat as usize;
        let my_bid = state.bids[my_seat].unwrap_or(0);
        let won = state.tricks_won[my_seat];

        let tricks_played = state.trick_no.saturating_sub(1);
        let tricks_remaining = state.hand_size.saturating_sub(tricks_played);

        let need = my_bid.saturating_sub(won);
        let avoid = won >= my_bid;
        let must_win_out = tricks_remaining > 0 && need >= tricks_remaining;

        let endgame = if state.hand_size >= 10 {
            tricks_remaining <= 4
        } else {
            tricks_remaining <= 3
        };

        let pressure = if need == 0 {
            -2.0
        } else if must_win_out {
            2.0
        } else if tricks_remaining > 0 {
            ((need as f32) / (tricks_remaining as f32) * 1.5).clamp(-0.2, 1.5)
        } else {
            0.0
        };

        Policy {
            need,
            avoid,
            pressure,
            tricks_remaining,
            endgame,
            must_win_out,
        }
    }

    // ----------------------------
    // Minimal memory interpretation
    // ----------------------------

    fn detect_opponent_voids(memory: Option<&RoundMemory>) -> [Vec<Suit>; 4] {
        let mut voids: [Vec<Suit>; 4] = [vec![], vec![], vec![], vec![]];
        let Some(memory) = memory else { return voids };

        for trick in &memory.tricks {
            if trick.plays.is_empty() {
                continue;
            }

            // Best-effort lead suit
            let lead = match &trick.plays[0].1 {
                PlayMemory::Exact(c) => Some(c.suit),
                PlayMemory::Suit(s) => Some(*s),
                _ => None,
            };
            let Some(lead) = lead else { continue };

            for (seat, play) in &trick.plays {
                match play {
                    PlayMemory::Exact(c) if c.suit != lead => {
                        let v = &mut voids[*seat as usize];
                        if !v.contains(&lead) {
                            v.push(lead);
                        }
                    }
                    PlayMemory::Suit(s) if *s != lead => {
                        let v = &mut voids[*seat as usize];
                        if !v.contains(&lead) {
                            v.push(lead);
                        }
                    }
                    _ => {}
                }
            }
        }

        voids
    }

    fn remaining_tracked_highs(memory: Option<&RoundMemory>, suit: Suit, my_hand: &[Card]) -> i32 {
        let tracked = |r: Rank| {
            matches!(
                r,
                Rank::Ten | Rank::Jack | Rank::Queen | Rank::King | Rank::Ace
            )
        };

        let mut rem = 5i32;
        for c in my_hand.iter().filter(|c| c.suit == suit) {
            if tracked(c.rank) {
                rem -= 1;
            }
        }

        let Some(memory) = memory else {
            return rem.max(0);
        };

        for trick in &memory.tricks {
            for (_seat, play) in &trick.plays {
                if let PlayMemory::Exact(c) = play {
                    if c.suit == suit && tracked(c.rank) {
                        rem -= 1;
                    }
                }
            }
        }

        rem.max(0)
    }

    // ----------------------------
    // Estimator: expected remaining tricks by bucket
    // ----------------------------

    fn is_trump(card: &Card, trump: Trump) -> bool {
        matches!(
            (trump, card.suit),
            (Trump::Clubs, Suit::Clubs)
                | (Trump::Diamonds, Suit::Diamonds)
                | (Trump::Hearts, Suit::Hearts)
                | (Trump::Spades, Suit::Spades)
        )
    }

    fn rank_cash_weight(rank: Rank) -> f32 {
        match rank {
            Rank::Ace => 1.0,
            Rank::King => 0.85,
            Rank::Queen => 0.55,
            Rank::Jack => 0.35,
            Rank::Ten => 0.25,
            _ => 0.0,
        }
    }

    fn rank_trump_weight(rank: Rank) -> f32 {
        match rank {
            Rank::Ace => 1.0,
            Rank::King => 0.9,
            Rank::Queen => 0.7,
            Rank::Jack => 0.55,
            Rank::Ten => 0.45,
            Rank::Nine => 0.3,
            _ => 0.15,
        }
    }

    fn suit_counts(hand: &[Card]) -> HashMap<Suit, usize> {
        let mut m = HashMap::new();
        for c in hand {
            *m.entry(c.suit).or_insert(0) += 1;
        }
        m
    }

    fn safe_to_run_suit_factor(
        suit: Suit,
        opponent_voids: &[Vec<Suit>; 4],
        trump_len: usize,
        tricks_remaining: u8,
    ) -> f32 {
        for voids in opponent_voids.iter().take(4) {
            if voids.contains(&suit) {
                return 0.0;
            }
        }

        let mut f: f32 = 0.2;

        if trump_len >= 5 {
            f = 0.55;
        } else if trump_len >= 4 {
            f = 0.45;
        } else if trump_len >= 3 {
            f = 0.35;
        }

        if tricks_remaining <= 4 {
            f += 0.2;
        } else if tricks_remaining <= 6 {
            f += 0.1;
        }

        f.clamp(0.0, 0.8)
    }

    fn estimate_from_here(
        state: &CurrentRoundInfo,
        memory: Option<&RoundMemory>,
        opponent_voids: &[Vec<Suit>; 4],
        policy: Policy,
        trump: Trump,
    ) -> Expect {
        let hand = &state.hand;

        // CASH: likely winners discounted by “tracked highs still out”
        let mut cash = 0.0;
        for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
            let mut suit_cards: Vec<Card> =
                hand.iter().copied().filter(|c| c.suit == suit).collect();
            if suit_cards.is_empty() {
                continue;
            }
            suit_cards.sort(); // low..high
            let rem_tracked = Self::remaining_tracked_highs(memory, suit, hand) as f32;
            let uncertainty = (rem_tracked / 5.0).clamp(0.0, 1.0);

            for c in suit_cards.iter().rev().take(3) {
                let base = Self::rank_cash_weight(c.rank);
                if base <= 0.0 {
                    continue;
                }
                let disc = match c.rank {
                    Rank::Ace => 1.0,
                    Rank::King => 1.0 - 0.35 * uncertainty,
                    Rank::Queen => 1.0 - 0.55 * uncertainty,
                    _ => 1.0 - 0.65 * uncertainty,
                };
                cash += base * disc;
            }
        }

        // TRUMP: trump control more valuable in smaller hands
        let mut tr = 0.0;
        let mut ruff = 0.0;
        let mut length = 0.0;

        if trump == Trump::NoTrumps {
            // In NT: length matters more
            let counts = Self::suit_counts(hand);
            for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
                let n = *counts.get(&suit).unwrap_or(&0) as i32;
                let raw = (n - 3).max(0) as f32;
                length += raw * 0.55;
            }
        } else {
            let trump_suit = match trump {
                Trump::Clubs => Suit::Clubs,
                Trump::Diamonds => Suit::Diamonds,
                Trump::Hearts => Suit::Hearts,
                Trump::Spades => Suit::Spades,
                Trump::NoTrumps => Suit::Clubs,
            };

            let counts = Self::suit_counts(hand);
            let trump_len = *counts.get(&trump_suit).unwrap_or(&0);
            let trumps: Vec<Card> = hand
                .iter()
                .copied()
                .filter(|c| c.suit == trump_suit)
                .collect();

            if trump_len > 0 {
                let mut quality = 0.0;
                for c in &trumps {
                    quality += Self::rank_trump_weight(c.rank);
                }
                let mut base = (trump_len as f32) * 0.55 + quality * 0.45;

                let ratio = (state.hand_size as f32 / 13.0).clamp(0.15, 1.0);
                let scale = (1.0 / ratio).min(2.0);
                base *= scale;

                tr += base * 0.35;
            }

            // RUFF potential
            if trump_len >= 2 {
                for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
                    if suit == trump_suit {
                        continue;
                    }
                    let n = *counts.get(&suit).unwrap_or(&0);
                    if n == 0 {
                        ruff += 0.55;
                    } else if n == 1 {
                        ruff += 0.25;
                    }
                }
                ruff *= ((trump_len as f32) / (state.hand_size as f32)).clamp(0.2, 1.0);
            }

            // LENGTH (discounted in trump games)
            for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
                if suit == trump_suit {
                    continue;
                }
                let n = *counts.get(&suit).unwrap_or(&0) as i32;
                let raw = (n - 3).max(0) as f32;
                if raw <= 0.0 {
                    continue;
                }
                let disc = Self::safe_to_run_suit_factor(
                    suit,
                    opponent_voids,
                    trump_len,
                    policy.tricks_remaining,
                );
                length += raw * disc;
            }
        }

        Expect {
            trump: tr,
            cash,
            length,
            ruff,
        }
    }

    // ----------------------------
    // Trick evaluation: tempo-aware win certainty
    // ----------------------------

    fn players_left_to_act(current_plays: &[(u8, Card)]) -> usize {
        4usize.saturating_sub(current_plays.len()).saturating_sub(1)
    }

    fn current_winner(current_plays: &[(u8, Card)], lead: Suit, trump: Trump) -> Option<Card> {
        let mut w = None;
        for (_seat, c) in current_plays {
            w = match w {
                None => Some(*c),
                Some(cur) => {
                    if card_beats(*c, cur, lead, trump) {
                        Some(*c)
                    } else {
                        Some(cur)
                    }
                }
            }
        }
        w
    }

    fn would_win_now(card: Card, current_plays: &[(u8, Card)], lead: Suit, trump: Trump) -> bool {
        match Self::current_winner(current_plays, lead, trump) {
            None => true,
            Some(w) => card_beats(card, w, lead, trump),
        }
    }

    fn win_certainty(
        card: Card,
        lead: Suit,
        trump: Trump,
        current_plays: &[(u8, Card)],
        memory: Option<&RoundMemory>,
        my_hand: &[Card],
    ) -> WinCertainty {
        if !Self::would_win_now(card, current_plays, lead, trump) {
            return WinCertainty::No;
        }

        let left = Self::players_left_to_act(current_plays);
        if left == 0 {
            return WinCertainty::Sure;
        }

        // Overtake risk proxy: remaining tracked highs in led suit.
        let rem = Self::remaining_tracked_highs(memory, lead, my_hand) as f32;
        let risk = (rem / 5.0).clamp(0.0, 1.0);

        let base = match card.rank {
            Rank::Ace => 1.0,
            Rank::King => 0.85,
            Rank::Queen => 0.65,
            Rank::Jack => 0.5,
            Rank::Ten => 0.4,
            _ => 0.25,
        };

        let position_penalty = 0.12 * left as f32;
        let score = base - (risk * 0.55) - position_penalty;

        if score >= 0.75 {
            WinCertainty::Likely
        } else {
            WinCertainty::Fragile
        }
    }

    fn accidental_win_risk(card: Card, trump: Trump, lead_suit: Option<Suit>, avoid: bool) -> f32 {
        if !avoid {
            return 0.0;
        }

        if trump != Trump::NoTrumps && Self::is_trump(&card, trump) {
            return 0.8;
        }

        match lead_suit {
            None => match card.rank {
                Rank::Ace => 0.9,
                Rank::King => 0.75,
                Rank::Queen => 0.55,
                Rank::Jack => 0.4,
                Rank::Ten => 0.3,
                _ => 0.15,
            },
            Some(lead) => {
                if card.suit != lead {
                    0.05
                } else {
                    match card.rank {
                        Rank::Ace => 0.85,
                        Rank::King => 0.7,
                        Rank::Queen => 0.5,
                        Rank::Jack => 0.35,
                        Rank::Ten => 0.25,
                        _ => 0.1,
                    }
                }
            }
        }
    }

    // ----------------------------
    // Play selection
    // ----------------------------

    fn choose_play_impl(state: &CurrentRoundInfo, cx: &GameContext) -> Card {
        let legal = state.legal_plays();
        if legal.is_empty() {
            return state.hand[0];
        }

        let trump = state.trump.unwrap_or(Trump::NoTrumps);
        let memory = cx.round_memory();

        let policy = Self::compute_policy(state);
        let opponent_voids = Self::detect_opponent_voids(memory);
        let expect = Self::estimate_from_here(state, memory, &opponent_voids, policy, trump);

        let lead_suit = state.current_trick_plays.first().map(|(_, c)| c.suit);

        // --- Endgame hard rules (small but powerful) ---
        if policy.endgame {
            // If avoiding, take the lowest legal that does NOT win if possible.
            if policy.avoid {
                let mut sorted = legal.clone();
                sorted.sort();
                if let Some(lead) = lead_suit {
                    for &c in &sorted {
                        if !Self::would_win_now(c, &state.current_trick_plays, lead, trump) {
                            return c;
                        }
                    }
                }
                return sorted[0];
            }

            // If must win out, take a win if available; else lowest legal.
            if policy.must_win_out {
                if let Some(lead) = lead_suit {
                    let mut winners: Vec<Card> = legal
                        .iter()
                        .copied()
                        .filter(|&c| {
                            Self::would_win_now(c, &state.current_trick_plays, lead, trump)
                        })
                        .collect();
                    if !winners.is_empty() {
                        winners.sort(); // cheapest win
                        return winners[0];
                    }
                } else {
                    // Leading: prefer high trump, else highest.
                    let mut sorted = legal.clone();
                    sorted.sort();
                    sorted.reverse();
                    for &c in &sorted {
                        if trump != Trump::NoTrumps && Self::is_trump(&c, trump) {
                            return c;
                        }
                    }
                    return sorted[0];
                }
            }
        }

        // --- Scoring (deterministic) ---
        let e_total = expect.total();
        let need_f = policy.need as f32;

        let mut best = legal[0];
        let mut best_score = f32::MIN;

        for &card in &legal {
            let mut s = 0.0;

            // 1) Target alignment via win certainty
            match lead_suit {
                Some(lead) => {
                    let cert = Self::win_certainty(
                        card,
                        lead,
                        trump,
                        &state.current_trick_plays,
                        memory,
                        &state.hand,
                    );
                    if policy.avoid {
                        s -= match cert {
                            WinCertainty::Sure => 5.0,
                            WinCertainty::Likely => 3.5,
                            WinCertainty::Fragile => 1.5,
                            WinCertainty::No => 0.0,
                        };
                    } else {
                        let left = Self::players_left_to_act(&state.current_trick_plays);
                        let base = match cert {
                            WinCertainty::Sure => 3.0,
                            WinCertainty::Likely => 2.4,
                            WinCertainty::Fragile => 1.2,
                            WinCertainty::No => 0.0,
                        };
                        // penalize fragile wins early in trick
                        let tempo = if left > 0 && cert == WinCertainty::Fragile {
                            0.6
                        } else {
                            1.0
                        };
                        s += base * tempo;
                        s += policy.pressure * (base * 0.35);
                    }
                }
                None => {
                    // Leading: avoid leading “forced winners” if avoiding; otherwise allow.
                    if policy.avoid {
                        s -= Self::rank_cash_weight(card.rank) * 0.8;
                        if trump != Trump::NoTrumps && Self::is_trump(&card, trump) {
                            s -= 0.6;
                        }
                    } else {
                        // Mild preference: lead cash winners when behind expectation.
                        s += Self::rank_cash_weight(card.rank) * 0.25;
                        if trump != Trump::NoTrumps && Self::is_trump(&card, trump) {
                            s += 0.15;
                        }
                    }

                    // Avoid leading into known voids unless we need wins badly
                    let mut void_risk = 0.0;
                    for (seat, voids) in opponent_voids.iter().enumerate() {
                        if seat == state.player_seat as usize {
                            continue;
                        }
                        if voids.contains(&card.suit) {
                            void_risk += 1.0;
                        }
                    }
                    if void_risk > 0.0 && !policy.must_win_out {
                        s -= void_risk * 0.35;
                    }
                }
            }

            // 2) Accidental win risk when avoiding
            s -= Self::accidental_win_risk(card, trump, lead_suit, policy.avoid) * 2.0;

            // 3) Expected-tricks guidance (moderate bias)
            if !policy.avoid && e_total + 0.25 < need_f {
                if trump != Trump::NoTrumps && Self::is_trump(&card, trump) {
                    s += 0.45;
                }
                s += Self::rank_cash_weight(card.rank) * 0.25;
            }
            if policy.avoid && e_total > need_f + 0.75 {
                // dump high cards to reduce future forced wins
                s += Self::rank_cash_weight(card.rank) * 0.15;
            }

            // 4) Tie-break: prefer lower card (conserve) unless must win out
            if !policy.must_win_out {
                s -= Self::rank_cash_weight(card.rank) * 0.05;
            }

            if s > best_score || (s == best_score && card < best) {
                best_score = s;
                best = card;
            }
        }

        best
    }

    // ----------------------------
    // Bidding and trump selection
    // ----------------------------

    fn estimate_tricks_basic(state: &CurrentRoundInfo) -> f32 {
        let hand = &state.hand;
        let hand_size = state.hand_size as f32;

        let counts = Self::suit_counts(hand);
        let mut lens: Vec<usize> = counts.values().copied().collect();
        lens.sort_by(|a, b| b.cmp(a));
        let longest = lens.first().copied().unwrap_or(0) as f32;
        let avg = hand_size / 4.0;

        let mut high = 0.0;
        for c in hand {
            high += Self::rank_cash_weight(c.rank);
        }

        let shape = ((longest - avg).max(0.0)) * 0.35;

        let mut short = 0.0;
        for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
            let n = *counts.get(&suit).unwrap_or(&0) as f32;
            if n == 0.0 {
                short += 0.35;
            } else if n == 1.0 {
                short += 0.2;
            }
        }

        (high * 0.85 + shape + short).clamp(0.0, hand_size)
    }

    fn exactness_difficulty(state: &CurrentRoundInfo) -> f32 {
        let hand = &state.hand;
        if hand.is_empty() || state.hand_size == 0 {
            return 1.0;
        }

        let counts = Self::suit_counts(hand);
        let avg = (state.hand_size as f32) / 4.0;

        let mut voids = 0.0;
        let mut singles = 0.0;
        let mut extreme = 0.0;

        for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
            let n = *counts.get(&suit).unwrap_or(&0) as f32;
            if n == 0.0 {
                voids += 1.0;
            } else if n == 1.0 {
                singles += 1.0;
            }
            if n > avg * 1.7 || (avg > 0.0 && n < avg * 0.5) {
                extreme += 1.0;
            }
        }

        let mut forced = 0.0;
        for c in hand {
            if matches!(c.rank, Rank::Ace | Rank::King) {
                forced += 1.0;
            }
        }

        let size_scale = ((state.hand_size as f32) / 13.0).sqrt().max(0.35);
        let mut diff = 1.0 + (voids * 0.25 + singles * 0.12 + extreme * 0.08 + forced * 0.02);
        diff *= size_scale;
        diff.clamp(0.6, 1.6)
    }

    fn choose_bid_from_estimate(legal: &[u8], est: f32) -> u8 {
        let mut best = legal[0];
        let mut best_d = (best as f32 - est).abs();
        for &b in legal.iter().skip(1) {
            let d = (b as f32 - est).abs();
            if d < best_d || (d == best_d && b < best) {
                best = b;
                best_d = d;
            }
        }
        best
    }

    fn adjust_for_history(est: f32, history: &GameHistory, hand_size: u8, my_seat: u8) -> f32 {
        let expected_avg = (hand_size as f32) / 4.0;
        let mut bias = 0.0;
        let mut n = 0.0;
        for seat in 0..4u8 {
            if seat == my_seat {
                continue;
            }
            // Extract recent bids for this seat from history
            let recent: Vec<u8> = history
                .rounds
                .iter()
                .rev()
                .take(3)
                .filter_map(|round| round.bids[seat as usize])
                .collect();
            if recent.is_empty() {
                continue;
            }
            let avg = recent.iter().map(|b| *b as f32).sum::<f32>() / (recent.len() as f32);
            bias += avg - expected_avg;
            n += 1.0;
        }
        if n == 0.0 {
            return est;
        }
        let table_bias = (bias / n).clamp(-1.0, 1.0);
        est - (table_bias * 0.15)
    }

    fn compute_bid(state: &CurrentRoundInfo, cx: &GameContext) -> u8 {
        let legal = state.legal_bids();
        let mut est = Self::estimate_tricks_basic(state);

        // Mild score-position adjustment (avoid over-aggression; exactness matters)
        let my = state.scores[state.player_seat as usize] as i32;
        let best = state.scores.iter().copied().max().unwrap_or(0) as i32;
        let delta = best - my;
        if delta >= 20 {
            est *= 1.08;
        } else if delta >= 10 {
            est *= 1.04;
        } else if delta <= 0 {
            est *= 0.98;
        }

        if let Some(history) = cx.game_history() {
            est = Self::adjust_for_history(est, history, state.hand_size, state.player_seat);
        }

        let diff = Self::exactness_difficulty(state);
        if diff > 1.0 {
            est *= 1.0 / diff;
        } else {
            est *= 1.0 + (1.0 - diff) * 0.03;
        }

        est = est.clamp(0.0, state.hand_size as f32);
        Self::choose_bid_from_estimate(&legal, est)
    }

    fn trump_score(hand: &[Card], suit: Suit) -> i32 {
        let count = hand.iter().filter(|c| c.suit == suit).count() as i32;
        let high: i32 = hand
            .iter()
            .filter(|c| c.suit == suit)
            .map(|c| match c.rank {
                Rank::Ace => 6,
                Rank::King => 5,
                Rank::Queen => 2,
                Rank::Jack => 1,
                Rank::Ten => 1,
                _ => 0,
            })
            .sum();
        (count * 10) + high
    }

    fn stopper_strength(hand: &[Card], suit: Suit) -> i32 {
        let mut ranks: Vec<Rank> = hand
            .iter()
            .filter(|c| c.suit == suit)
            .map(|c| c.rank)
            .collect();
        ranks.sort();
        let has = |r: Rank| ranks.contains(&r);

        let mut score = 0;
        if has(Rank::Ace) {
            score += 3;
        }
        if has(Rank::King) && ranks.len() >= 2 {
            score += 2;
        }
        if has(Rank::Queen) && has(Rank::Jack) && ranks.len() >= 3 {
            score += 1;
        }
        score
    }

    fn prefer_no_trump(state: &CurrentRoundInfo) -> bool {
        let hand = &state.hand;
        let counts = Self::suit_counts(hand);

        let mut min_len = usize::MAX;
        let mut max_len = 0usize;
        for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
            let n = *counts.get(&suit).unwrap_or(&0);
            min_len = min_len.min(n);
            max_len = max_len.max(n);
        }
        if min_len == 0 {
            return false;
        }
        if max_len >= 7 {
            return false;
        }

        let mut stoppers = 0;
        for suit in [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades] {
            if Self::stopper_strength(hand, suit) >= 2 {
                stoppers += 1;
            }
        }

        let need = if state.hand_size <= 5 { 2 } else { 3 };
        stoppers >= need
    }

    fn choose_trump_impl(state: &CurrentRoundInfo) -> Trump {
        let legal = state.legal_trumps();
        let hand = &state.hand;

        if legal.contains(&Trump::NoTrumps) && Self::prefer_no_trump(state) {
            let best_suit = [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades]
                .into_iter()
                .map(|s| Self::trump_score(hand, s))
                .max()
                .unwrap_or(0);
            if best_suit < 55 {
                return Trump::NoTrumps;
            }
        }

        let mut best = legal[0];
        let mut best_score = i32::MIN;
        for &t in &legal {
            let suit = match t {
                Trump::Clubs => Some(Suit::Clubs),
                Trump::Diamonds => Some(Suit::Diamonds),
                Trump::Hearts => Some(Suit::Hearts),
                Trump::Spades => Some(Suit::Spades),
                Trump::NoTrumps => None,
            };
            let score = suit.map(|s| Self::trump_score(hand, s)).unwrap_or(-999);
            if score > best_score {
                best_score = score;
                best = t;
            }
        }
        best
    }
}

impl AiPlayer for Reckoner {
    fn choose_bid(&self, state: &CurrentRoundInfo, cx: &GameContext) -> Result<u8, AiError> {
        let legal = state.legal_bids();
        if legal.is_empty() {
            return Err(AiError::InvalidMove("No legal bids".into()));
        }
        Ok(Self::compute_bid(state, cx))
    }

    fn choose_trump(&self, state: &CurrentRoundInfo, _cx: &GameContext) -> Result<Trump, AiError> {
        let legal = state.legal_trumps();
        if legal.is_empty() {
            return Err(AiError::InvalidMove("No legal trumps".into()));
        }
        Ok(Self::choose_trump_impl(state))
    }

    fn choose_play(&self, state: &CurrentRoundInfo, cx: &GameContext) -> Result<Card, AiError> {
        let legal = state.legal_plays();
        if legal.is_empty() {
            return Err(AiError::InvalidMove("No legal plays".into()));
        }
        Ok(Self::choose_play_impl(state, cx))
    }
}
