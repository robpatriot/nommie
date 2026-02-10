# AI Simulator

Fast in-memory game simulator for AI training and evaluation. Runs Nomination Whist games entirely in memory without database or validation overhead, enabling rapid iteration on AI strategies.

## Prerequisites

- Rust toolchain (workspace builds from repo root)
- Python 3 for `analyze_results.py` (stdlib only, no pip deps)

## Quick Start

From the **repository root**:

```bash
# Run 10 games with default AI (Strategic on all seats)
cargo run --manifest-path packages/ai-simulator/Cargo.toml -- -g 10 --show-output

# Analyze the latest results
python3 packages/ai-simulator/analyze_results.py
```

## ai-simulator CLI

### Running Simulations

```bash
cargo run --manifest-path packages/ai-simulator/Cargo.toml -- [OPTIONS]
```

### Options

| Option | Default | Description |
|--------|---------|-------------|
| `-g`, `--games <N>` | 1 | Number of games to simulate |
| `--seats <AI>` | — | AI type for all 4 seats (shorthand) |
| `--seat0`, `--seat1`, `--seat2`, `--seat3` | strategic | Per-seat AI type |
| `--seed <u64>` | random | Game seed for deterministic replay |
| `--output-dir <PATH>` | ./simulation-results | Output directory |
| `--output-format` | jsonl | `jsonl` or `json` |
| `--compress` | false | Gzip output files |
| `--show-output` | false | Print summary and file paths |
| `-v`, `--verbose` | false | Enable debug logging |

### AI Types

- **strategic** — Default; balanced bidding and play
- **heuristic** — Rule-based approach
- **reckoner** — Counting-focused strategy
- **tactician** — Tactical play emphasis
- **random** — Random bids and plays (baseline)

### Examples

```bash
# 100 games, all Strategic (default)
cargo run --manifest-path packages/ai-simulator/Cargo.toml -- -g 100 --show-output

# Pit Strategic vs Random
cargo run --manifest-path packages/ai-simulator/Cargo.toml -- -g 50 \
  --seat0 strategic --seat1 strategic --seat2 random --seat3 random --show-output

# All seats same AI (shortcut)
cargo run --manifest-path packages/ai-simulator/Cargo.toml -- -g 20 --seats tactician --show-output

# Deterministic replay
cargo run --manifest-path packages/ai-simulator/Cargo.toml -- -g 1 --seed 42 --show-output

# Compressed output for large runs
cargo run --manifest-path packages/ai-simulator/Cargo.toml -- -g 1000 --compress --output-dir ./batch-results
```

## Output

The simulator writes two files per run in the output directory:

| File | Description |
|------|-------------|
| `simulation_<timestamp>.jsonl` | One JSON object per line: full game metrics, rounds, bid accuracy |
| `simulation_<timestamp>_summary.csv` | Per-game summary: winner, scores, AI types |

With `--compress`, the JSONL file is gzipped (`.jsonl.gz`).

## analyze_results.py

Analyzes JSONL output to produce bid accuracy, error distributions, and performance metrics.

### Usage

```bash
# Analyze latest file in simulation-results/ (default)
python3 packages/ai-simulator/analyze_results.py

# Analyze specific file
python3 packages/ai-simulator/analyze_results.py simulation-results/simulation_2026-02-10T18-25-41.jsonl

# Analyze latest in a directory
python3 packages/ai-simulator/analyze_results.py ./batch-results
```

### Output Sections

- **Round-Level Insights** — Trump selection frequency
- **Terminology** — Error convention: `error = actual_tricks - bid`
- **Bid Accuracy** — Exact / overbid / underbid rates, MAE, RMSE, error histogram
- **Breakdown by Hand Size** — Performance at 2–5, 6–9, 10–13 cards
- **Breakdown by Trump** — Shown when MAE disparity across trump types exceeds threshold
- **Breakdown by Seat** — Win rate, mean score, MAE per seat
- **Auction Dynamics** — Highest bidder vs non-highest bidder performance
- **Calibration Tables** — Bid → outcome mapping (by bid value and hand-size buckets)
- **Score Metrics** — Win rate, avg score, bonus hit rate
- **Contract Conversion** — Avg bid, tricks, bonus points per hand

### Optional Export

Set `SIM_EXPORT=1` to export bid samples to CSV:

```bash
SIM_EXPORT=1 python3 packages/ai-simulator/analyze_results.py simulation-results/latest.jsonl
```

Creates `latest_export.csv` alongside the input file.

## Typical Workflow

1. Run simulations from repo root:
   ```bash
   cargo run --manifest-path packages/ai-simulator/Cargo.toml -- -g 200 --show-output
   ```

2. Analyze results:
   ```bash
   python3 packages/ai-simulator/analyze_results.py
   ```

3. Compare AI types by running separate batches with different `--seat*` / `--seats` and analyzing each output.
