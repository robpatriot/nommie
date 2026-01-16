//! Metrics collection and output for AI simulation results.

use backend::domain::player_view::{GameHistory, RoundHistory};
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;

/// Complete game metrics for output.
#[derive(Debug, Clone, Serialize)]
pub struct GameMetrics {
    pub game_id: u32,
    pub seed: i64,
    pub timestamp: String,
    pub config: GameConfig,
    pub result: GameResultMetrics,
    pub rounds: Vec<RoundMetrics>,
    pub player_metrics: Vec<PlayerMetrics>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GameConfig {
    pub ai_types: [String; 4],
    pub total_games: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct GameResultMetrics {
    pub final_scores: [i16; 4],
    pub winner: u8,
    pub duration_ms: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct RoundMetrics {
    pub round_no: u8,
    pub hand_size: u8,
    pub dealer: u8,
    pub bids: [Option<u8>; 4],
    pub trump_selector: Option<u8>,
    pub trump: Option<String>,
    pub tricks_won: [u8; 4],
    pub scores: [i16; 4],
    pub bid_accuracy: Vec<BidAccuracy>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BidAccuracy {
    pub seat: u8,
    pub bid: u8,
    pub tricks: u8,
    pub exact: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub underbid: Option<u8>, // Amount by which tricks exceeded bid (tricks > bid)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub overbid: Option<u8>, // Amount by which bid exceeded tricks (tricks < bid)
}

#[derive(Debug, Clone, Serialize)]
pub struct PlayerMetrics {
    pub seat: u8,
    pub ai_type: String,
    pub total_score: i16,
    pub rounds_won: u32,
    pub bid_accuracy: BidAccuracyStats,
    pub avg_tricks_per_round: f64,
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub custom_metrics: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BidAccuracyStats {
    pub exact: u32,
    pub underbid: u32, // Count of underbids (tricks > bid)
    pub overbid: u32,  // Count of overbids (tricks < bid)
    pub exact_pct: f64,
}

/// Build metrics from game result and history.
pub fn build_game_metrics(
    game_id: u32,
    seed: i64,
    ai_types: [String; 4],
    total_games: u32,
    result: &super::simulator::GameResult,
    duration_ms: f64,
) -> GameMetrics {
    let timestamp = time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| String::from("unknown"));

    // Build round metrics
    let rounds: Vec<RoundMetrics> = result
        .history
        .rounds
        .iter()
        .map(|round| build_round_metrics(round))
        .collect();

    // Build player metrics
    let player_metrics: Vec<PlayerMetrics> = (0..4)
        .map(|seat| build_player_metrics(seat as u8, &ai_types[seat], &result.history, &rounds))
        .collect();

    // Determine winner
    let winner = result
        .final_scores
        .iter()
        .enumerate()
        .max_by_key(|&(_, &score)| score)
        .map(|(idx, _)| idx as u8)
        .unwrap_or(0);

    GameMetrics {
        game_id,
        seed,
        timestamp,
        config: GameConfig {
            ai_types,
            total_games,
        },
        result: GameResultMetrics {
            final_scores: result.final_scores,
            winner,
            duration_ms,
        },
        rounds,
        player_metrics,
    }
}

fn build_round_metrics(round: &RoundHistory) -> RoundMetrics {
    let mut bid_accuracy = Vec::new();

    for seat in 0..4 {
        if let Some(bid) = round.bids[seat] {
            // Derive tricks from round_score and bid.
            // round_score = tricks + 10 bonus when tricks == bid, else tricks.
            let round_score = round.scores[seat].round_score;
            let tricks = if round_score >= 10 && round_score - 10 == bid {
                bid
            } else {
                round_score
            };
            let exact = tricks == bid;
            // Terminology: underbid = bid too low (tricks > bid), overbid = bid too high (tricks < bid)
            let underbid = if tricks > bid {
                Some(tricks - bid)
            } else {
                None
            };
            let overbid = if tricks < bid {
                Some(bid - tricks)
            } else {
                None
            };

            bid_accuracy.push(BidAccuracy {
                seat: seat as u8,
                bid,
                tricks,
                exact,
                underbid,
                overbid,
            });
        }
    }

    RoundMetrics {
        round_no: round.round_no,
        hand_size: round.hand_size,
        dealer: round.dealer_seat,
        bids: round.bids,
        trump_selector: round.trump_selector_seat,
        trump: round.trump.as_ref().map(|t| format!("{:?}", t)),
        tricks_won: extract_tricks_won(round),
        scores: round.scores.map(|s| s.round_score as i16),
        bid_accuracy,
    }
}

fn extract_tricks_won(round: &RoundHistory) -> [u8; 4] {
    // Derive tricks for each player using the same logic as in build_round_metrics.
    let mut tricks_won = [0u8; 4];
    for seat in 0..4 {
        let round_score = round.scores[seat].round_score;
        let bid_opt = round.bids[seat];
        let tricks = if let Some(bid) = bid_opt {
            if round_score >= 10 && round_score - 10 == bid {
                bid
            } else {
                round_score
            }
        } else {
            round_score
        };
        tricks_won[seat] = tricks;
    }
    tricks_won
}

fn build_player_metrics(
    seat: u8,
    ai_type: &str,
    history: &GameHistory,
    rounds: &[RoundMetrics],
) -> PlayerMetrics {
    let total_score = history
        .rounds
        .last()
        .map(|r| r.scores[seat as usize].cumulative_score)
        .unwrap_or(0);

    // Count rounds won (highest score in round)
    let rounds_won = rounds
        .iter()
        .filter(|r| {
            let max_score = r.scores.iter().max().copied().unwrap_or(0);
            r.scores[seat as usize] == max_score
        })
        .count() as u32;

    // Calculate bid accuracy
    // Terminology: underbid = bid too low (tricks > bid), overbid = bid too high (tricks < bid)
    let mut exact = 0;
    let mut underbid = 0; // tricks > bid
    let mut overbid = 0; // tricks < bid

    for round in rounds {
        if let Some(ba) = round.bid_accuracy.iter().find(|ba| ba.seat == seat) {
            if ba.exact {
                exact += 1;
            } else if ba.underbid.is_some() {
                // underbid = tricks > bid
                underbid += 1;
            } else if ba.overbid.is_some() {
                // overbid = tricks < bid
                overbid += 1;
            }
        }
    }

    let total_bids = exact + underbid + overbid;
    let exact_pct = if total_bids > 0 {
        (exact as f64 / total_bids as f64) * 100.0
    } else {
        0.0
    };

    // Calculate average tricks per round
    let total_tricks: u32 = rounds
        .iter()
        .map(|r| r.tricks_won[seat as usize] as u32)
        .sum();
    let avg_tricks = if !rounds.is_empty() {
        total_tricks as f64 / rounds.len() as f64
    } else {
        0.0
    };

    PlayerMetrics {
        seat,
        ai_type: ai_type.to_string(),
        total_score,
        rounds_won,
        bid_accuracy: BidAccuracyStats {
            exact,
            underbid,
            overbid,
            exact_pct,
        },
        avg_tricks_per_round: avg_tricks,
        custom_metrics: HashMap::new(), // Will be populated by AI-specific metrics
    }
}

/// CSV summary row for quick analysis.
#[derive(Debug, Serialize)]
pub struct CsvSummaryRow {
    pub game_id: u32,
    pub seed: i64,
    pub winner: u8,
    pub seat0_score: i16,
    pub seat1_score: i16,
    pub seat2_score: i16,
    pub seat3_score: i16,
    pub seat0_ai: String,
    pub seat1_ai: String,
    pub seat2_ai: String,
    pub seat3_ai: String,
}

impl From<&GameMetrics> for CsvSummaryRow {
    fn from(metrics: &GameMetrics) -> Self {
        CsvSummaryRow {
            game_id: metrics.game_id,
            seed: metrics.seed,
            winner: metrics.result.winner,
            seat0_score: metrics.result.final_scores[0],
            seat1_score: metrics.result.final_scores[1],
            seat2_score: metrics.result.final_scores[2],
            seat3_score: metrics.result.final_scores[3],
            seat0_ai: metrics.config.ai_types[0].clone(),
            seat1_ai: metrics.config.ai_types[1].clone(),
            seat2_ai: metrics.config.ai_types[2].clone(),
            seat3_ai: metrics.config.ai_types[3].clone(),
        }
    }
}
