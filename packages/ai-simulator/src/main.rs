//! AI Simulator CLI - Fast in-memory game simulation for AI training.
//!
//! This tool runs games entirely in memory without validation or database overhead,
//! allowing rapid iteration on AI strategies.

mod simulator;

use simulator::{GameResult, Simulator};
use backend::ai::{create_ai, AiConfig, AiPlayer};
use clap::{Parser, ValueEnum};
use rand::Rng;
use std::time::Instant;
use tracing::{info, warn};

#[derive(Parser)]
#[command(name = "ai-simulator")]
#[command(about = "Fast in-memory game simulator for AI training")]
struct Args {
    /// Number of games to simulate
    #[arg(short, long, default_value = "1")]
    games: u32,

    /// AI type for seat 0
    #[arg(long, default_value = "strategic")]
    seat0: AiType,

    /// AI type for seat 1
    #[arg(long, default_value = "strategic")]
    seat1: AiType,

    /// AI type for seat 2
    #[arg(long, default_value = "strategic")]
    seat2: AiType,

    /// AI type for seat 3
    #[arg(long, default_value = "strategic")]
    seat3: AiType,

    /// Game seed (for deterministic games)
    #[arg(long)]
    seed: Option<u64>,

    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,
}

#[derive(Debug, Clone, ValueEnum)]
enum AiType {
    Strategic,
    Heuristic,
    Reckoner,
    Random,
}

impl AiType {
    fn name(&self) -> &'static str {
        match self {
            AiType::Strategic => "Strategic",
            AiType::Heuristic => "Heuristic",
            AiType::Reckoner => "Reckoner",
            AiType::Random => "Random",
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Initialize logging
    let filter = if args.verbose {
        "debug"
    } else {
        "info"
    };
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .init();

    info!("Starting AI simulator");
    info!("Configuration: {} games", args.games);
    info!(
        "AI types: seat0={:?}, seat1={:?}, seat2={:?}, seat3={:?}",
        args.seat0, args.seat1, args.seat2, args.seat3
    );

    // Create AI players
    let ais = [
        create_ai_player(args.seat0.name())?,
        create_ai_player(args.seat1.name())?,
        create_ai_player(args.seat2.name())?,
        create_ai_player(args.seat3.name())?,
    ];

    // Run simulations
    let start = Instant::now();
    let mut results = Vec::new();
    let mut errors = 0;

    // Initialize RNG for random seeds if not provided
    let mut rng = rand::rng();
    
    for game_num in 1..=args.games {
        let game_seed = args.seed.map(|s| s as i64).unwrap_or_else(|| {
            // Generate random seed if no seed provided
            rng.random::<i64>()
        });

        match run_game(game_num, game_seed, &ais) {
            Ok(result) => {
                let scores = result.final_scores;
                results.push(result);
                if args.verbose {
                    info!("Game {} completed: scores={:?}", game_num, scores);
                }
            }
            Err(e) => {
                errors += 1;
                warn!("Game {} failed: {}", game_num, e);
            }
        }
    }

    let elapsed = start.elapsed();

    // Print summary
    print_summary(&results, errors, elapsed, args.games);

    Ok(())
}

fn create_ai_player(
    ai_type: &str,
) -> Result<Box<dyn AiPlayer>, Box<dyn std::error::Error>> {
    // Use random seed for each AI to get varied behavior
    let mut rng = rand::rng();
    let config = AiConfig::from_json(Some(&serde_json::json!({
        "seed": rng.random::<u64>()
    })));

    create_ai(ai_type, config)
        .ok_or_else(|| format!("Unknown AI type: {}", ai_type).into())
}

fn run_game(
    game_num: u32,
    game_seed: i64,
    ais: &[Box<dyn AiPlayer>; 4],
) -> Result<GameResult, Box<dyn std::error::Error>> {
    let simulator = Simulator::new(game_seed, game_num as i64);
    simulator.simulate_game(ais).map_err(|e| e.into())
}

fn print_summary(results: &[GameResult], errors: u32, elapsed: std::time::Duration, total: u32) {
    println!("\n=== Simulation Summary ===");
    println!("Games completed: {}/{}", results.len(), total);
    if errors > 0 {
        println!("Errors: {}", errors);
    }
    println!("Total time: {:?}", elapsed);
    if !results.is_empty() {
        println!("Average time per game: {:?}", elapsed / results.len() as u32);
    }

    if results.is_empty() {
        return;
    }

    // Calculate statistics
    let mut wins = [0u32; 4];
    let mut total_scores = [0i64; 4];
    let mut max_scores = [i16::MIN; 4];
    let mut min_scores = [i16::MAX; 4];

    for result in results {
        // Find winner(s) - highest score
        let max_score = result.final_scores.iter().max().copied().unwrap_or(0);
        for (seat, &score) in result.final_scores.iter().enumerate() {
            total_scores[seat] += score as i64;
            max_scores[seat] = max_scores[seat].max(score);
            min_scores[seat] = min_scores[seat].min(score);
        }
        // Count wins (only players with max score win)
        for (seat, &score) in result.final_scores.iter().enumerate() {
            if score == max_score {
                wins[seat] += 1;
            }
        }
    }

    println!("\n=== Results by Seat ===");
    for seat in 0..4 {
        let avg_score = total_scores[seat] as f64 / results.len() as f64;
        let win_rate = (wins[seat] as f64 / results.len() as f64) * 100.0;
        println!(
            "Seat {}: avg={:.1}, min={}, max={}, wins={} ({:.1}%)",
            seat, avg_score, min_scores[seat], max_scores[seat], wins[seat], win_rate
        );
    }
}

