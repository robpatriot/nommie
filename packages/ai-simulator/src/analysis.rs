//! Analysis helpers for simulation results (Phase 3).

use serde_json::Value;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

/// Load and analyze JSONL results file.
pub fn analyze_jsonl<P: AsRef<Path>>(
    path: P,
) -> Result<AnalysisResults, Box<dyn std::error::Error>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut games = Vec::new();

    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let game: Value = serde_json::from_str(&line)?;
        games.push(game);
    }

    Ok(AnalysisResults { games })
}

/// Analysis results container.
pub struct AnalysisResults {
    games: Vec<Value>,
}

impl AnalysisResults {
    /// Get total number of games.
    pub fn game_count(&self) -> usize {
        self.games.len()
    }

    /// Calculate win rates by AI type and seat.
    pub fn win_rates_by_ai(&self) -> HashMap<String, WinStats> {
        let mut stats: HashMap<String, WinStats> = HashMap::new();

        for game in &self.games {
            if let (Some(winner), Some(config)) = (
                game["result"]["winner"].as_u64(),
                game["config"]["ai_types"].as_array(),
            ) {
                let winner_seat = winner as usize;
                if let Some(ai_type) = config.get(winner_seat).and_then(|v| v.as_str()) {
                    let entry = stats.entry(ai_type.to_string()).or_insert_with(WinStats::default);
                    entry.wins += 1;
                }

                // Count total games per AI type
                for (seat, ai_val) in config.iter().enumerate() {
                    if let Some(ai_type) = ai_val.as_str() {
                        let entry = stats.entry(ai_type.to_string()).or_insert_with(WinStats::default);
                        entry.total_games += 1;
                    }
                }
            }
        }

        // Calculate win rates
        for stat in stats.values_mut() {
            if stat.total_games > 0 {
                stat.win_rate = (stat.wins as f64 / stat.total_games as f64) * 100.0;
            }
        }

        stats
    }

    /// Calculate average bid accuracy by AI type.
    pub fn bid_accuracy_by_ai(&self) -> HashMap<String, BidAccuracyStats> {
        let mut stats: HashMap<String, BidAccuracyStats> = HashMap::new();

        for game in &self.games {
            if let Some(players) = game["player_metrics"].as_array() {
                for player in players {
                    if let (Some(ai_type), Some(accuracy)) = (
                        player["ai_type"].as_str(),
                        player["bid_accuracy"].as_object(),
                    ) {
                        let entry = stats
                            .entry(ai_type.to_string())
                            .or_insert_with(BidAccuracyStats::default);

                        if let Some(exact) = accuracy["exact"].as_u64() {
                            entry.exact += exact as u32;
                        }
                        if let Some(over) = accuracy["over"].as_u64() {
                            entry.over += over as u32;
                        }
                        if let Some(under) = accuracy["under"].as_u64() {
                            entry.under += under as u32;
                        }
                        entry.total_rounds += 1;
                    }
                }
            }
        }

        // Calculate percentages
        for stat in stats.values_mut() {
            let total = stat.exact + stat.over + stat.under;
            if total > 0 {
                stat.exact_pct = (stat.exact as f64 / total as f64) * 100.0;
                stat.over_pct = (stat.over as f64 / total as f64) * 100.0;
                stat.under_pct = (stat.under as f64 / total as f64) * 100.0;
            }
        }

        stats
    }

    /// Get average scores by AI type.
    pub fn avg_scores_by_ai(&self) -> HashMap<String, ScoreStats> {
        let mut stats: HashMap<String, ScoreStats> = HashMap::new();

        for game in &self.games {
            if let (Some(players), Some(final_scores)) = (
                game["player_metrics"].as_array(),
                game["result"]["final_scores"].as_array(),
            ) {
                for (player, score_val) in players.iter().zip(final_scores.iter()) {
                    if let (Some(ai_type), Some(score)) = (
                        player["ai_type"].as_str(),
                        score_val.as_i64(),
                    ) {
                        let entry = stats
                            .entry(ai_type.to_string())
                            .or_insert_with(ScoreStats::default);
                        entry.total_score += score;
                        entry.game_count += 1;
                    }
                }
            }
        }

        // Calculate averages
        for stat in stats.values_mut() {
            if stat.game_count > 0 {
                stat.avg_score = stat.total_score as f64 / stat.game_count as f64;
            }
        }

        stats
    }
}

#[derive(Default, Debug)]
pub struct WinStats {
    pub wins: u32,
    pub total_games: u32,
    pub win_rate: f64,
}

#[derive(Default, Debug)]
pub struct BidAccuracyStats {
    pub exact: u32,
    pub over: u32,
    pub under: u32,
    pub total_rounds: u32,
    pub exact_pct: f64,
    pub over_pct: f64,
    pub under_pct: f64,
}

#[derive(Default, Debug)]
pub struct ScoreStats {
    pub total_score: i64,
    pub game_count: u32,
    pub avg_score: f64,
}

