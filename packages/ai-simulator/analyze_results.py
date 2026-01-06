#!/usr/bin/env python3
"""Quick analysis script for simulation results."""

import json
import os
import sys
from collections import defaultdict
from math import sqrt
from pathlib import Path
from typing import Dict, List, Tuple, Optional, Any, Callable

# Constants
HISTOGRAM_MIN_BUCKET = -6
HISTOGRAM_MAX_BUCKET = 6
HISTOGRAM_BAR_SCALE = 2  # Scale bar to 50% max
TRUMP_DISPARITY_THRESHOLD = 0.5
NUM_PLAYERS = 4
BONUS_POINTS = 10
ALL_TRUMP_TYPES = ['Clubs', 'Diamonds', 'Hearts', 'Spades', 'No Trumps']
HAND_SIZE_BUCKETS = [(2, 5, "2-5"), (6, 9, "6-9"), (10, 13, "10-13")]


def mean(values: List[float]) -> float:
    """Calculate mean of a list of numbers."""
    if not values:
        return 0.0
    return sum(values) / len(values)


def median(values: List[float]) -> float:
    """Calculate median of a list of numbers."""
    if not values:
        return 0.0
    sorted_vals = sorted(values)
    n = len(sorted_vals)
    if n % 2 == 0:
        return (sorted_vals[n // 2 - 1] + sorted_vals[n // 2]) / 2.0
    else:
        return sorted_vals[n // 2]


def stddev(values: List[float]) -> float:
    """Calculate standard deviation of a list of numbers."""
    if not values:
        return 0.0
    m = mean(values)
    variance = sum((x - m) ** 2 for x in values) / len(values)
    return sqrt(variance)


def histogram(values: List[float], buckets: List[int]) -> Dict[str, int]:
    """Create histogram of values using bucket boundaries.
    
    Args:
        values: List of numbers (errors, which are integers)
        buckets: List of bucket boundaries (e.g., [-6, -5, -4, ..., 5, 6])
    
    Returns:
        Dict mapping bucket labels to counts
    """
    counts = defaultdict(int)
    min_bucket = buckets[0]
    max_bucket = buckets[-1]
    
    for val in values:
        val_int = int(round(val))  # Round to nearest integer for bucketing
        if val_int <= min_bucket:
            label = f"<={min_bucket}"
        elif val_int >= max_bucket:
            label = f">=+{max_bucket}"  # Format with + for consistency
        else:
            # Map to exact bucket value (exclude min and max which are handled above)
            # Format with + for positive numbers
            if val_int >= 0:
                label = f"+{val_int}"
            else:
                label = str(val_int)
        counts[label] += 1
    return counts


def groupby(items: List[Any], key_func: Callable[[Any], Any]) -> Dict[Any, List[Any]]:
    """Group items by a key function."""
    groups = defaultdict(list)
    for item in items:
        key = key_func(item)
        groups[key].append(item)
    return groups


def normalize_trump(trump_val: Optional[str]) -> str:
    """Normalize trump values: None, "NoTrumps" (Debug), or "NO_TRUMPS" (serde) -> "No Trumps"."""
    if trump_val is None or trump_val == 'NoTrumps' or trump_val == 'NO_TRUMPS':
        return 'No Trumps'
    return trump_val


def calculate_bid_stats(group: List[Dict[str, Any]]) -> Dict[str, Any]:
    """Calculate bid accuracy statistics for a group of samples.
    
    Args:
        group: List of sample dictionaries with bid/error data
    
    Returns:
        Dict with: n, errors, abs_errors, bids, actuals, exact_count, over_count,
        under_count, exact_pct, over_pct, under_pct, mean_bid, mean_actual, mean_err, mae
    """
    n = len(group)
    errors = [s['error'] for s in group]
    abs_errors = [s['abs_error'] for s in group]
    bids = [s['bid'] for s in group]
    actuals = [s['actual_tricks'] for s in group]
    
    exact_count = sum(1 for s in group if s['exact'])
    over_count = sum(1 for s in group if s['error'] < 0)
    under_count = sum(1 for s in group if s['error'] > 0)
    
    exact_pct = (exact_count / n * 100) if n > 0 else 0
    over_pct = (over_count / n * 100) if n > 0 else 0
    under_pct = (under_count / n * 100) if n > 0 else 0
    
    mean_bid = mean(bids)
    mean_actual = mean(actuals)
    mean_err = mean(errors)
    mae = mean(abs_errors)
    
    return {
        'n': n,
        'errors': errors,
        'abs_errors': abs_errors,
        'bids': bids,
        'actuals': actuals,
        'exact_count': exact_count,
        'over_count': over_count,
        'under_count': under_count,
        'exact_pct': exact_pct,
        'over_pct': over_pct,
        'under_pct': under_pct,
        'mean_bid': mean_bid,
        'mean_actual': mean_actual,
        'mean_err': mean_err,
        'mae': mae,
    }


def load_games(filepath: Path) -> List[Dict[str, Any]]:
    """Load games from JSONL file with error handling.
    
    Args:
        filepath: Path to JSONL file
        
    Returns:
        List of game dictionaries
        
    Raises:
        FileNotFoundError: If file doesn't exist
        json.JSONDecodeError: If JSON parsing fails
    """
    if not filepath.exists():
        raise FileNotFoundError(f"File not found: {filepath}")
    
    games = []
    with open(filepath, 'r') as f:
        for line_num, line in enumerate(f, 1):
            line = line.strip()
            if not line:
                continue
            try:
                games.append(json.loads(line))
            except json.JSONDecodeError as e:
                print(f"Warning: Failed to parse JSON on line {line_num}: {e}", file=sys.stderr)
                continue
    
    if not games:
        raise ValueError(f"No valid games found in {filepath}")
    
    return games


def validate_game_structure(game: Dict[str, Any]) -> bool:
    """Validate that game has expected structure.
    
    Args:
        game: Game dictionary to validate
        
    Returns:
        True if valid, False otherwise
    """
    required_keys = ['game_id', 'config', 'result', 'rounds']
    if not all(key in game for key in required_keys):
        return False
    
    if 'ai_types' not in game.get('config', {}):
        return False
    
    if 'winner' not in game.get('result', {}):
        return False
    
    return True


def collect_samples(games: List[Dict[str, Any]]) -> Tuple[List[Dict[str, Any]], List[Dict[str, Any]], Dict[str, Dict[str, int]], Dict[str, Dict[str, Any]]]:
    """Collect bid and game samples from games.
    
    Args:
        games: List of game dictionaries
        
    Returns:
        Tuple of (bid_samples, game_samples, win_stats, scores)
    """
    bid_samples = []
    game_samples = []
    win_stats = defaultdict(lambda: {'wins': 0, 'total': 0})
    bid_accuracy = defaultdict(lambda: {'exact': 0, 'underbid': 0, 'overbid': 0, 'total': 0})
    scores = defaultdict(lambda: {'total': 0, 'count': 0, 'min': float('inf'), 'max': float('-inf')})
    
    for game in games:
        if not validate_game_structure(game):
            print(f"Warning: Skipping invalid game {game.get('game_id', 'unknown')}", file=sys.stderr)
            continue
        
        game_id = game['game_id']
        winner = game['result']['winner']
        ai_types = game['config']['ai_types']
        final_scores = game['result']['final_scores']
        
        # Track wins and game-level samples
        for seat, ai_type in enumerate(ai_types):
            win_stats[ai_type]['total'] += 1
            won = (seat == winner)
            if seat == winner:
                win_stats[ai_type]['wins'] += 1
            
            game_samples.append({
                'ai_type': ai_type,
                'game_id': game_id,
                'seat': seat,
                'won': won,
                'final_score': final_scores[seat],
            })
        
        # Track bid accuracy and collect bid samples
        for round_idx, round_data in enumerate(game.get('rounds', [])):
            hand_size = round_data.get('hand_size', 0)
            dealer = round_data.get('dealer')
            trump = round_data.get('trump')
            trump_selector = round_data.get('trump_selector')
            bids = round_data.get('bids', [])
            tricks_won = round_data.get('tricks_won', [])
            
            # Determine highest bidder: use trump_selector if available (most accurate),
            # otherwise fall back to calculating from bids
            highest_bidder_seat = trump_selector
            if highest_bidder_seat is None:
                # Fallback: find highest bid (handles cases where trump wasn't selected)
                highest_bid = -1
                for seat, bid in enumerate(bids):
                    if bid is not None and bid > highest_bid:
                        highest_bid = bid
                        highest_bidder_seat = seat
            
            for ba in round_data.get('bid_accuracy', []):
                seat = ba.get('seat', 0)
                ai_type = ai_types[seat] if seat < len(ai_types) else 'unknown'
                bid = ba.get('bid', 0)
                actual_tricks = ba.get('tricks', 0)
                error = actual_tricks - bid  # actual - bid
                abs_error = abs(error)
                
                # Track aggregated bid accuracy
                acc = bid_accuracy[ai_type]
                if ba.get('exact', False):
                    acc['exact'] += 1
                elif ba.get('underbid'):
                    acc['underbid'] += 1
                elif ba.get('overbid'):
                    acc['overbid'] += 1
                acc['total'] += 1
                
                # Collect sample
                sample = {
                    'ai_type': ai_type,
                    'game_id': game_id,
                    'round_index': round_idx,
                    'hand_size': hand_size,
                    'seat': seat,
                    'bid': bid,
                    'actual_tricks': actual_tricks,
                    'error': error,
                    'abs_error': abs_error,
                    'trump': trump,
                    'dealer': dealer,
                    'highest_bidder': (seat == highest_bidder_seat) if highest_bidder_seat is not None else None,
                    'chose_trump': (seat == trump_selector) if trump_selector is not None else None,
                    'exact': ba.get('exact', False),
                    'round_score': round_data.get('scores', [0] * NUM_PLAYERS)[seat] if seat < len(round_data.get('scores', [])) else 0,
                }
                bid_samples.append(sample)
        
        # Track scores
        for seat, (ai_type, score) in enumerate(zip(ai_types, final_scores)):
            scores[ai_type]['total'] += score
            scores[ai_type]['count'] += 1
            scores[ai_type]['min'] = min(scores[ai_type]['min'], score)
            scores[ai_type]['max'] = max(scores[ai_type]['max'], score)
    
    return bid_samples, game_samples, win_stats, scores


def print_round_insights(games: List[Dict[str, Any]]) -> None:
    """Print round-level insights including trump selection frequency."""
    total_rounds = sum(len(game.get('rounds', [])) for game in games)
    print("\n=== Round-Level Insights ===")
    print(f"Total rounds analyzed: {total_rounds}")
    
    # Trump selection patterns
    trump_counts = defaultdict(int)
    # Initialize all possible trump types to ensure they're shown even with 0 count
    for trump in ALL_TRUMP_TYPES:
        trump_counts[trump] = 0
    
    for game in games:
        for round_data in game.get('rounds', []):
            trump_raw = round_data.get('trump')
            trump = normalize_trump(trump_raw)
            trump_counts[trump] += 1
    
    print("\nTrump selection frequency:")
    for trump in ALL_TRUMP_TYPES:
        count = trump_counts[trump]
        pct = (count / total_rounds * 100) if total_rounds > 0 else 0
        print(f"  {trump:15} {count:4} ({pct:5.1f}%)")


def print_terminology() -> None:
    """Print terminology explanation."""
    print("\n" + "=" * 70)
    print("=== Terminology ===")
    print("error = actual_tricks - bid")
    print("  • error > 0  => underbid (bid too low, took more tricks than bid)")
    print("  • error < 0  => overbid (bid too high, took fewer tricks than bid)")
    print("  • error = 0  => exact match")


def print_bid_accuracy_overall(bid_samples: List[Dict[str, Any]], samples_by_ai_type: Dict[str, List[Dict[str, Any]]]) -> None:
    """Print overall bid accuracy and error distribution."""
    print("\n" + "=" * 70)
    print("=== Bid Accuracy & Error Distribution (Overall) ===")
    
    for ai_type in sorted(samples_by_ai_type.keys()):
        samples = samples_by_ai_type[ai_type]
        if not samples:
            continue
        
        stats = calculate_bid_stats(samples)
        
        print(f"\n{ai_type}:")
        print(f"  N: {stats['n']}")
        print(f"  Exact: {stats['exact_pct']:5.1f}%  Overbid: {stats['over_pct']:5.1f}%  Underbid: {stats['under_pct']:5.1f}%")
        print(f"  Mean error: {stats['mean_err']:6.2f}")
        print(f"  Median error: {median(stats['errors']):6.2f}")
        print(f"  StdDev error: {stddev(stats['errors']):6.2f}")
        print(f"  MAE: {stats['mae']:6.2f}")
        rmse = sqrt(mean([e ** 2 for e in stats['errors']])) if stats['errors'] else 0.0
        print(f"  RMSE: {rmse:6.2f}")
        
        # Histogram
        buckets = list(range(HISTOGRAM_MIN_BUCKET, HISTOGRAM_MAX_BUCKET + 1))
        hist = histogram(stats['errors'], buckets)
        print(f"  Error distribution:")
        bucket_labels = [f"<={HISTOGRAM_MIN_BUCKET}"] + [str(i) if i < 0 else f"+{i}" for i in range(HISTOGRAM_MIN_BUCKET + 1, HISTOGRAM_MAX_BUCKET)] + [f">=+{HISTOGRAM_MAX_BUCKET}"]
        for bucket in bucket_labels:
            count = hist.get(bucket, 0)
            pct = (count / stats['n'] * 100) if stats['n'] > 0 else 0
            bar = '#' * int(pct / HISTOGRAM_BAR_SCALE)
            print(f"    {bucket:>4}: {count:4} ({pct:5.1f}%) {bar}")


def print_breakdown_by_hand_size(samples_by_ai_type: Dict[str, List[Dict[str, Any]]]) -> None:
    """Print breakdown by hand size."""
    print("\n" + "=" * 70)
    print("=== Breakdown by Hand Size ===")
    
    for ai_type in sorted(samples_by_ai_type.keys()):
        samples = samples_by_ai_type[ai_type]
        if not samples:
            continue
        
        by_hand_size = groupby(samples, lambda s: s['hand_size'])
        print(f"\n{ai_type}:")
        for hand_size in sorted(by_hand_size.keys()):
            group = by_hand_size[hand_size]
            stats = calculate_bid_stats(group)
            
            print(f"  Hand size {hand_size:2}: N={stats['n']:4}  "
                  f"Exact:{stats['exact_pct']:5.1f}% Overbid:{stats['over_pct']:5.1f}% Underbid:{stats['under_pct']:5.1f}%  "
                  f"Bid:{stats['mean_bid']:4.1f} Actual:{stats['mean_actual']:4.1f}  "
                  f"Error:{stats['mean_err']:5.2f} MAE:{stats['mae']:5.2f}")


def check_trump_disparity(samples_by_ai_type: Dict[str, List[Dict[str, Any]]]) -> Dict[str, Dict[str, Any]]:
    """Check for significant disparity in MAE across trump types.
    
    Returns:
        Dict mapping ai_type to trump breakdown data if disparity found
    """
    trump_disparity_data = {}
    
    for ai_type, samples in samples_by_ai_type.items():
        if not samples:
            continue
        
        by_trump = groupby(samples, lambda s: normalize_trump(s.get('trump')))
        mae_by_trump = {}
        
        for trump, group in by_trump.items():
            abs_errors = [s['abs_error'] for s in group]
            mae_by_trump[trump] = mean(abs_errors) if abs_errors else 0.0
        
        if len(mae_by_trump) > 1:
            mae_values = list(mae_by_trump.values())
            mae_range = max(mae_values) - min(mae_values)
            # Show if MAE range > threshold (significant disparity)
            if mae_range > TRUMP_DISPARITY_THRESHOLD:
                trump_disparity_data[ai_type] = {
                    'by_trump': by_trump,
                    'mae_range': mae_range
                }
    
    return trump_disparity_data


def print_breakdown_by_trump(trump_disparity_data: Dict[str, Dict[str, Any]]) -> None:
    """Print breakdown by trump type if significant disparity detected."""
    if not trump_disparity_data:
        return
    
    print("\n" + "=" * 70)
    print("=== Breakdown by Trump Type ===")
    print("Note: Shown because significant performance disparity detected across trump types.")
    print("      This may indicate AI issues with specific trump types.")
    
    for ai_type in sorted(trump_disparity_data.keys()):
        by_trump = trump_disparity_data[ai_type]['by_trump']
        mae_range = trump_disparity_data[ai_type]['mae_range']
        
        print(f"\n{ai_type} (MAE range: {mae_range:.2f}):")
        for trump in sorted(by_trump.keys()):
            group = by_trump[trump]
            stats = calculate_bid_stats(group)
            
            print(f"  {trump:15}: N={stats['n']:4}  "
                  f"Exact:{stats['exact_pct']:5.1f}% Overbid:{stats['over_pct']:5.1f}% Underbid:{stats['under_pct']:5.1f}%  "
                  f"Bid:{stats['mean_bid']:4.1f} Actual:{stats['mean_actual']:4.1f}  "
                  f"Error:{stats['mean_err']:5.2f} MAE:{stats['mae']:5.2f}")


def print_breakdown_by_seat(samples_by_ai_type: Dict[str, List[Dict[str, Any]]], game_samples: List[Dict[str, Any]]) -> None:
    """Print breakdown by seat."""
    print("\n" + "=" * 70)
    print("=== Breakdown by Seat ===")
    
    for ai_type in sorted(samples_by_ai_type.keys()):
        samples = samples_by_ai_type[ai_type]
        if not samples:
            continue
        
        by_seat = groupby(samples, lambda s: s['seat'])
        print(f"\n{ai_type}:")
        for seat in sorted(by_seat.keys()):
            group = by_seat[seat]
            n = len(group)
            abs_errors = [s['abs_error'] for s in group]
            mae = mean(abs_errors) if abs_errors else 0.0
            
            # Win rate for this seat (from game samples)
            seat_games = [g for g in game_samples if g['ai_type'] == ai_type and g['seat'] == seat]
            wins = sum(1 for g in seat_games if g['won'])
            total_games = len(seat_games)
            win_rate = (wins / total_games * 100) if total_games > 0 else 0
            
            # Mean score per game
            scores_list = [g['final_score'] for g in seat_games]
            mean_score = mean(scores_list) if scores_list else 0.0
            
            print(f"  Seat {seat}: N={n:4}  Win rate: {win_rate:5.1f}%  "
                  f"Mean score/game: {mean_score:6.1f}  MAE: {mae:5.2f}")
        
        # First leader analysis (if dealer info available)
        if any(s.get('dealer') is not None for s in samples):
            print(f"\n  {ai_type} - First Leader Analysis:")
            # First leader is (dealer + 1) % NUM_PLAYERS
            first_leader_samples = []
            non_first_leader_samples = []
            for s in samples:
                if s.get('dealer') is not None:
                    first_leader_seat = (s['dealer'] + 1) % NUM_PLAYERS
                    if s['seat'] == first_leader_seat:
                        first_leader_samples.append(s)
                    else:
                        non_first_leader_samples.append(s)
            
            if first_leader_samples:
                n_fl = len(first_leader_samples)
                mae_fl = mean([s['abs_error'] for s in first_leader_samples])
                print(f"    First leader: N={n_fl:4}  MAE: {mae_fl:5.2f}")
            
            if non_first_leader_samples:
                n_nfl = len(non_first_leader_samples)
                mae_nfl = mean([s['abs_error'] for s in non_first_leader_samples])
                print(f"    Not first leader: N={n_nfl:4}  MAE: {mae_nfl:5.2f}")


def print_auction_dynamics(samples_by_ai_type: Dict[str, List[Dict[str, Any]]]) -> None:
    """Print auction dynamics (highest bidder effect)."""
    print("\n" + "=" * 70)
    print("=== Auction Dynamics ===")
    
    for ai_type in sorted(samples_by_ai_type.keys()):
        samples = samples_by_ai_type[ai_type]
        if not samples:
            continue
        
        # Use highest_bidder if available, otherwise fall back to chose_trump (they're the same)
        # Prefer highest_bidder as it's more explicit
        if any(s.get('highest_bidder') is not None for s in samples):
            highest_bidder_samples = [s for s in samples if s.get('highest_bidder')]
            not_highest_bidder_samples = [s for s in samples if not s.get('highest_bidder')]
        elif any(s.get('chose_trump') is not None for s in samples):
            # Fallback: use chose_trump if highest_bidder not available
            highest_bidder_samples = [s for s in samples if s.get('chose_trump')]
            not_highest_bidder_samples = [s for s in samples if not s.get('chose_trump')]
        else:
            continue
        
        print(f"\n{ai_type}:")
        for label, group in [("Highest bidder", highest_bidder_samples),
                            ("Not highest bidder", not_highest_bidder_samples)]:
            if not group:
                continue
            
            stats = calculate_bid_stats(group)
            
            print(f"  {label:20}: N={stats['n']:4}  "
                  f"Exact:{stats['exact_pct']:5.1f}% Overbid:{stats['over_pct']:5.1f}% Underbid:{stats['under_pct']:5.1f}%  "
                  f"Bid:{stats['mean_bid']:4.1f} Actual:{stats['mean_actual']:4.1f}  "
                  f"Error:{stats['mean_err']:5.2f} MAE:{stats['mae']:5.2f}")


def print_calibration_table(samples: List[Dict[str, Any]], label: str) -> None:
    """Print calibration table for a group of samples.
    
    Args:
        samples: List of sample dictionaries
        label: Label for the table
    """
    by_bid = groupby(samples, lambda s: s['bid'])
    print(f"    {label}:")
    print(f"      {'Bid':>4} {'Count':>6} {'Avg Actual':>11} {'Mean Error':>11}")
    for bid in sorted(by_bid.keys()):
        group = by_bid[bid]
        n = len(group)
        actuals = [s['actual_tricks'] for s in group]
        errors = [s['error'] for s in group]
        avg_actual = mean(actuals) if actuals else 0.0
        mean_err = mean(errors) if errors else 0.0
        print(f"      {bid:4} {n:6} {avg_actual:11.2f} {mean_err:11.2f}")


def print_calibration_tables(samples_by_ai_type: Dict[str, List[Dict[str, Any]]]) -> None:
    """Print calibration tables (bid → outcome mapping)."""
    print("\n" + "=" * 70)
    print("=== Calibration Tables (Bid → Outcome Mapping) ===")
    
    for ai_type in sorted(samples_by_ai_type.keys()):
        samples = samples_by_ai_type[ai_type]
        if not samples:
            continue
        
        # By bid value
        print(f"\n{ai_type} - By Bid Value:")
        print_calibration_table(samples, "All hand sizes")
        
        # By hand-size buckets
        print(f"\n  {ai_type} - By Hand-Size Buckets:")
        for min_size, max_size, label in HAND_SIZE_BUCKETS:
            bucket_samples = [s for s in samples if min_size <= s['hand_size'] <= max_size]
            if not bucket_samples:
                continue
            print_calibration_table(bucket_samples, f"Hand size {label}")


def print_score_metrics(samples_by_ai_type: Dict[str, List[Dict[str, Any]]], game_samples: List[Dict[str, Any]]) -> None:
    """Print score metrics (objective performance)."""
    print("\n" + "=" * 70)
    print("=== Score Metrics (Objective Performance) ===")
    
    unique_ai_types_games = sorted(set(g['ai_type'] for g in game_samples))
    
    for ai_type in unique_ai_types_games:
        games = [g for g in game_samples if g['ai_type'] == ai_type]
        if not games:
            continue
        
        wins = sum(1 for g in games if g['won'])
        total = len(games)
        win_rate = (wins / total * 100) if total > 0 else 0
        
        scores_list = [g['final_score'] for g in games]
        avg_score = mean(scores_list) if scores_list else 0.0
        stddev_score = stddev(scores_list) if scores_list else 0.0
        
        # Per-hand metrics from bid samples
        bid_samples_ai = samples_by_ai_type.get(ai_type, [])
        if bid_samples_ai:
            # Points per hand = actual_tricks + (BONUS_POINTS if exact else 0)
            points_per_hand = [s['actual_tricks'] + (BONUS_POINTS if s['exact'] else 0) for s in bid_samples_ai]
            avg_points_per_hand = mean(points_per_hand) if points_per_hand else 0.0
            
            exact_count = sum(1 for s in bid_samples_ai if s['exact'])
            bonus_hit_rate = (exact_count / len(bid_samples_ai) * 100) if bid_samples_ai else 0
        else:
            avg_points_per_hand = 0.0
            bonus_hit_rate = 0.0
        
        print(f"\n{ai_type}:")
        print(f"  Win rate: {win_rate:5.1f}%")
        print(f"  Avg score/game: {avg_score:6.1f}")
        print(f"  StdDev score/game: {stddev_score:6.1f}")
        print(f"  Avg points/hand: {avg_points_per_hand:6.2f}")
        print(f"  Bonus hit rate (+10 rate): {bonus_hit_rate:5.1f}%")


def print_contract_conversion_stats(samples_by_ai_type: Dict[str, List[Dict[str, Any]]]) -> None:
    """Print round-level contract conversion stats."""
    print("\n" + "=" * 70)
    print("=== Round-Level Contract Conversion Stats ===")
    
    for ai_type in sorted(samples_by_ai_type.keys()):
        samples = samples_by_ai_type[ai_type]
        if not samples:
            continue
        
        bids = [s['bid'] for s in samples]
        actuals = [s['actual_tricks'] for s in samples]
        exact_count = sum(1 for s in samples if s['exact'])
        exact_rate = (exact_count / len(samples)) if samples else 0
        
        avg_bid = mean(bids) if bids else 0.0
        avg_actual = mean(actuals) if actuals else 0.0
        avg_bonus_points = BONUS_POINTS * exact_rate
        avg_tricks_points = avg_actual
        
        print(f"\n{ai_type}:")
        print(f"  Avg bid: {avg_bid:5.2f}")
        print(f"  Avg actual tricks: {avg_actual:5.2f}")
        print(f"  Avg bonus points/hand: {avg_bonus_points:5.2f}")
        print(f"  Avg tricks points/hand: {avg_tricks_points:5.2f}")
        print(f"  Total avg points/hand: {avg_bonus_points + avg_tricks_points:5.2f}")


def export_to_csv(bid_samples: List[Dict[str, Any]], filepath: Path) -> None:
    """Export bid samples to CSV file.
    
    Args:
        bid_samples: List of sample dictionaries
        filepath: Path to output CSV file
    """
    try:
        with open(filepath, 'w') as f:
            # Write header
            f.write("ai_name,game_id,round_index,hand_size,seat,bid,actual_tricks,error,abs_error,")
            f.write("trump,highest_bidder,chose_trump,score_delta,bonus_awarded\n")
            
            # Write rows
            for s in sorted(bid_samples, key=lambda x: (x['game_id'], x['round_index'], x['seat'])):
                trump_str = s.get('trump') or ''
                highest_bidder_str = '1' if s.get('highest_bidder') else '0'
                chose_trump_str = '1' if s.get('chose_trump') else '0'
                bonus_awarded_str = '1' if s.get('exact') else '0'
                score_delta = s.get('round_score', 0)
                
                f.write(f"{s['ai_type']},{s['game_id']},{s['round_index']},{s['hand_size']},")
                f.write(f"{s['seat']},{s['bid']},{s['actual_tricks']},{s['error']},{s['abs_error']},")
                f.write(f"{trump_str},{highest_bidder_str},{chose_trump_str},{score_delta},{bonus_awarded_str}\n")
        
        print("\n" + "=" * 70)
        print(f"=== Export ===")
        print(f"Exported {len(bid_samples)} records to: {filepath}")
        print(f"Export enabled via SIM_EXPORT=1 environment variable")
    except IOError as e:
        print(f"Error: Failed to write export file: {e}", file=sys.stderr)


def analyze_jsonl(filepath: Path) -> None:
    """Analyze JSONL results file.
    
    Args:
        filepath: Path to JSONL file
    """
    try:
        games = load_games(filepath)
    except (FileNotFoundError, ValueError) as e:
        print(f"Error: {e}", file=sys.stderr)
        sys.exit(1)
    
    print(f"Loaded {len(games)} games from {filepath}\n")
    
    # Detect if all seats use the same AI type (for context in analysis)
    unique_ai_types_in_games = set()
    for game in games:
        if 'config' in game and 'ai_types' in game['config']:
            unique_ai_types_in_games.update(game['config']['ai_types'])
    all_same_ai = len(unique_ai_types_in_games) == 1
    
    if all_same_ai:
        ai_type_name = list(unique_ai_types_in_games)[0]
        print(f"NOTE: All 4 seats use the same AI type: {ai_type_name}")
        print("  Positional/auction dynamics stats compare the same AI in different roles.\n")
    
    # Collect samples
    bid_samples, game_samples, win_stats, scores = collect_samples(games)
    
    # Pre-filter samples by AI type for efficiency
    samples_by_ai_type = defaultdict(list)
    for sample in bid_samples:
        samples_by_ai_type[sample['ai_type']].append(sample)
    
    # Print all analysis sections
    print_round_insights(games)
    print_terminology()
    print_bid_accuracy_overall(bid_samples, samples_by_ai_type)
    print_breakdown_by_hand_size(samples_by_ai_type)
    
    trump_disparity_data = check_trump_disparity(samples_by_ai_type)
    print_breakdown_by_trump(trump_disparity_data)
    
    print_breakdown_by_seat(samples_by_ai_type, game_samples)
    print_auction_dynamics(samples_by_ai_type)
    print_calibration_tables(samples_by_ai_type)
    print_score_metrics(samples_by_ai_type, game_samples)
    print_contract_conversion_stats(samples_by_ai_type)
    
    # Optional export
    export_enabled = os.getenv('SIM_EXPORT', '0') == '1'
    if export_enabled:
        export_path = filepath.parent / f"{filepath.stem}_export.csv"
        export_to_csv(bid_samples, export_path)


if __name__ == '__main__':
    if len(sys.argv) < 2:
        # Find latest JSONL file in default directory
        results_dir = Path('simulation-results')
        if results_dir.exists():
            jsonl_files = list(results_dir.glob('*.jsonl'))
            if jsonl_files:
                latest = max(jsonl_files, key=lambda p: p.stat().st_mtime)
                analyze_jsonl(latest)
            else:
                print("No JSONL files found in simulation-results/", file=sys.stderr)
                sys.exit(1)
        else:
            print("Usage: python3 analyze_results.py [<path-to-jsonl-file>|<directory>]", file=sys.stderr)
            sys.exit(1)
    else:
        arg_path = Path(sys.argv[1])
        if arg_path.is_dir():
            # Directory provided - find latest JSONL file
            jsonl_files = list(arg_path.glob('*.jsonl'))
            if jsonl_files:
                latest = max(jsonl_files, key=lambda p: p.stat().st_mtime)
                analyze_jsonl(latest)
            else:
                print(f"No JSONL files found in {arg_path}/", file=sys.stderr)
                sys.exit(1)
        else:
            # File path provided
            analyze_jsonl(arg_path)
