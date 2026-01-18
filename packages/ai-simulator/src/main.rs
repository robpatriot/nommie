//! AI Simulator CLI - Fast in-memory game simulation for AI training.
//!
//! This tool runs games entirely in memory without validation or database overhead,
//! allowing rapid iteration on AI strategies.

mod metrics;
mod output;
mod simulator;
mod types;

use backend::ai::{create_ai, AiConfig, AiPlayer};
use clap::{Parser, ValueEnum};
use metrics::build_game_metrics;
use output::OutputWriter;
use rand::Rng;
use simulator::{GameResult, Simulator};
use std::time::Instant;
use tracing::{info, warn};

#[derive(Parser)]
#[command(name = "ai-simulator")]
#[command(about = "Fast in-memory game simulator for AI training")]
struct Args {
    /// Number of games to simulate
    #[arg(short, long, default_value = "1")]
    games: u32,

    /// AI type for all seats (shortcut to set all 4 seats to the same AI)
    #[arg(long, conflicts_with_all = ["seat0", "seat1", "seat2", "seat3"])]
    seats: Option<AiType>,

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

    /// Game seed (for deterministic games) - if provided, fills first 8 bytes of 32-byte seed
    #[arg(long)]
    seed: Option<u64>,

    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,

    /// Show output summary and file paths
    #[arg(long)]
    show_output: bool,

    /// Output directory for results
    #[arg(long, default_value = "./simulation-results")]
    output_dir: String,

    /// Output format
    #[arg(long, default_value = "jsonl")]
    output_format: OutputFormat,

    /// Compress output files
    #[arg(long)]
    compress: bool,

    /// Metrics detail level
    #[arg(long, default_value = "detailed")]
    metrics_level: MetricsLevel,
}

#[derive(Debug, Clone, ValueEnum)]
enum AiType {
    Strategic,
    Heuristic,
    Reckoner,
    Tactician,
    Random,
}

use types::{MetricsLevel, OutputFormat};

impl AiType {
    fn name(&self) -> &'static str {
        match self {
            AiType::Strategic => "Strategic",
            AiType::Heuristic => "Heuristic",
            AiType::Reckoner => "Reckoner",
            AiType::Tactician => "Tactician",
            AiType::Random => "RandomPlayer", // Actual name in registry
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Initialize logging - silent by default, only show warnings/errors
    let filter = if args.verbose {
        "debug"
    } else if args.show_output {
        "info"
    } else {
        "warn" // Only show warnings and errors by default
    };
    tracing_subscriber::fmt().with_env_filter(filter).init();

    if args.show_output {
        info!("Starting AI simulator");
        info!("Configuration: {} games", args.games);
    }

    // Determine AI types: use --seats if provided, otherwise use individual seat parameters
    let seat_types = if let Some(seats_ai) = args.seats {
        [
            seats_ai.clone(),
            seats_ai.clone(),
            seats_ai.clone(),
            seats_ai,
        ]
    } else {
        [args.seat0, args.seat1, args.seat2, args.seat3]
    };

    if args.show_output {
        info!(
            "AI types: seat0={:?}, seat1={:?}, seat2={:?}, seat3={:?}",
            seat_types[0], seat_types[1], seat_types[2], seat_types[3]
        );
    }

    // Create output writer
    let mut output_writer =
        OutputWriter::new(&args.output_dir, &args.output_format, args.compress)?;
    if args.show_output {
        info!("Output directory: {}", args.output_dir);
    }

    // Create AI players
    let ai_types = [
        seat_types[0].name().to_string(),
        seat_types[1].name().to_string(),
        seat_types[2].name().to_string(),
        seat_types[3].name().to_string(),
    ];
    let ais = [
        create_ai_player(seat_types[0].name())?,
        create_ai_player(seat_types[1].name())?,
        create_ai_player(seat_types[2].name())?,
        create_ai_player(seat_types[3].name())?,
    ];

    // Run simulations
    let start = Instant::now();
    let mut results = Vec::new();
    let mut errors = 0;

    for game_num in 1..=args.games {
        let game_start = Instant::now();
        let game_seed: [u8; 32] = if let Some(s) = args.seed {
            // If seed is provided, use it to fill first 8 bytes, rest zeros
            let mut seed = [0u8; 32];
            seed[..8].copy_from_slice(&s.to_le_bytes());
            seed
        } else {
            // Generate random 32-byte seed
            rand::random()
        };

        let game_res = run_game(game_num, game_seed, &ais);

        match game_res {
            Ok(result) => {
                let duration_ms = game_start.elapsed().as_secs_f64() * 1000.0;
                let scores = result.final_scores;

                // Build and write metrics
                let metrics = build_game_metrics(
                    game_num,
                    &game_seed,
                    ai_types.clone(),
                    args.games,
                    &result,
                    duration_ms,
                );

                if let Err(e) = output_writer.write_game(&metrics) {
                    warn!("Failed to write metrics for game {}: {}", game_num, e);
                }

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

    // Get output file paths before finishing
    let (jsonl_path, csv_path) = output_writer.output_paths();
    let jsonl_path_clone = jsonl_path.cloned();
    let csv_path_clone = csv_path.cloned();

    // Finish writing
    output_writer.finish()?;

    // Print output file paths and summary only if requested
    if args.show_output {
        if let Some(path) = jsonl_path_clone {
            info!("Detailed results written to: {}", path.display());
        }
        if let Some(path) = csv_path_clone {
            info!("Summary CSV written to: {}", path.display());
        }

        // Print summary
        print_summary(&results, errors, elapsed, args.games);
    }

    Ok(())
}

fn create_ai_player(ai_type: &str) -> Result<Box<dyn AiPlayer>, Box<dyn std::error::Error>> {
    // Use random seed for each AI to get varied behavior
    let mut rng = rand::rng();
    let config = AiConfig::from_json(Some(&serde_json::json!({
        "seed": rng.random::<u64>()
    })));

    create_ai(ai_type, config).ok_or_else(|| format!("Unknown AI type: {}", ai_type).into())
}

fn run_game(
    game_num: u32,
    game_seed: [u8; 32],
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
        println!(
            "Average time per game: {:?}",
            elapsed / results.len() as u32
        );
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
