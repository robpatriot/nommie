//! Shared types for the simulator.

use clap::ValueEnum;

#[derive(Debug, Clone, ValueEnum)]
pub enum OutputFormat {
    Jsonl,
    Json,
}

#[derive(Debug, Clone, ValueEnum)]
pub enum MetricsLevel {
    Basic,
    Detailed,
}
