#!/usr/bin/env python3
"""Quick analysis script for simulation results."""

import json
import sys
from collections import defaultdict
from pathlib import Path

def analyze_jsonl(filepath):
    """Analyze JSONL results file."""
    games = []
    with open(filepath, 'r') as f:
        for line in f:
            if line.strip():
                games.append(json.loads(line))
    
    print(f"Loaded {len(games)} games from {filepath}\n")
    
    # Win rates by AI type
    win_stats = defaultdict(lambda: {'wins': 0, 'total': 0})
    bid_accuracy = defaultdict(lambda: {'exact': 0, 'over': 0, 'under': 0, 'total': 0})
    scores = defaultdict(lambda: {'total': 0, 'count': 0, 'min': float('inf'), 'max': float('-inf')})
    
    for game in games:
        winner = game['result']['winner']
        ai_types = game['config']['ai_types']
        final_scores = game['result']['final_scores']
        
        # Track wins
        for seat, ai_type in enumerate(ai_types):
            win_stats[ai_type]['total'] += 1
            if seat == winner:
                win_stats[ai_type]['wins'] += 1
        
        # Track bid accuracy
        for player in game['player_metrics']:
            ai_type = player['ai_type']
            acc = player['bid_accuracy']
            bid_accuracy[ai_type]['exact'] += acc['exact']
            bid_accuracy[ai_type]['over'] += acc['over']
            bid_accuracy[ai_type]['under'] += acc['under']
            bid_accuracy[ai_type]['total'] += acc['exact'] + acc['over'] + acc['under']
        
        # Track scores
        for seat, (ai_type, score) in enumerate(zip(ai_types, final_scores)):
            scores[ai_type]['total'] += score
            scores[ai_type]['count'] += 1
            scores[ai_type]['min'] = min(scores[ai_type]['min'], score)
            scores[ai_type]['max'] = max(scores[ai_type]['max'], score)
    
    # Print win rates
    print("=== Win Rates by AI Type ===")
    for ai_type in sorted(win_stats.keys()):
        stats = win_stats[ai_type]
        win_rate = (stats['wins'] / stats['total'] * 100) if stats['total'] > 0 else 0
        print(f"{ai_type:15} {stats['wins']:3}/{stats['total']:3} = {win_rate:5.1f}%")
    
    # Print bid accuracy
    print("\n=== Bid Accuracy by AI Type ===")
    for ai_type in sorted(bid_accuracy.keys()):
        acc = bid_accuracy[ai_type]
        total = acc['total']
        if total > 0:
            exact_pct = (acc['exact'] / total * 100)
            over_pct = (acc['over'] / total * 100)
            under_pct = (acc['under'] / total * 100)
            print(f"{ai_type:15} Exact: {exact_pct:5.1f}%  Over: {over_pct:5.1f}%  Under: {under_pct:5.1f}%")
    
    # Print average scores
    print("\n=== Average Scores by AI Type ===")
    for ai_type in sorted(scores.keys()):
        s = scores[ai_type]
        avg = s['total'] / s['count'] if s['count'] > 0 else 0
        print(f"{ai_type:15} Avg: {avg:6.1f}  Min: {s['min']:4.0f}  Max: {s['max']:4.0f}")
    
    # Round-level insights
    print("\n=== Round-Level Insights ===")
    total_rounds = sum(len(game['rounds']) for game in games)
    print(f"Total rounds analyzed: {total_rounds}")
    
    # Trump selection patterns
    trump_counts = defaultdict(int)
    for game in games:
        for round in game['rounds']:
            if round['trump']:
                trump_counts[round['trump']] += 1
    
    print("\nTrump selection frequency:")
    for trump, count in sorted(trump_counts.items(), key=lambda x: -x[1]):
        pct = (count / total_rounds * 100) if total_rounds > 0 else 0
        print(f"  {trump:15} {count:4} ({pct:5.1f}%)")

if __name__ == '__main__':
    if len(sys.argv) < 2:
        # Find latest JSONL file
        results_dir = Path('simulation-results')
        if results_dir.exists():
            jsonl_files = list(results_dir.glob('*.jsonl'))
            if jsonl_files:
                latest = max(jsonl_files, key=lambda p: p.stat().st_mtime)
                analyze_jsonl(latest)
            else:
                print("No JSONL files found in simulation-results/")
        else:
            print("Usage: python3 analyze_results.py <path-to-jsonl-file>")
    else:
        analyze_jsonl(sys.argv[1])

