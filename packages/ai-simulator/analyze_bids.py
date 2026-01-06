#!/usr/bin/env python3
"""Analyze bid totals vs hand size to see if underbidding is causing forced tricks."""

import json
import sys
from collections import defaultdict
from pathlib import Path

def analyze_bids(filepath):
    """Analyze bid totals and hand sizes."""
    games = []
    with open(filepath, 'r') as f:
        for line in f:
            if line.strip():
                games.append(json.loads(line))
    
    print(f"Loaded {len(games)} games from {filepath}\n")
    
    # Track bid totals vs hand size
    total_bids_by_hand_size = defaultdict(lambda: {'total_bids': 0, 'rounds': 0, 'below_hand_size': 0, 'equal_hand_size': 0, 'above_hand_size': 0})
    bid_distribution = defaultdict(int)
    
    for game in games:
        for round in game['rounds']:
            hand_size = round['hand_size']
            bids = [b for b in round['bids'] if b is not None]
            
            if len(bids) == 4:
                total_bid = sum(bids)
                total_bids_by_hand_size[hand_size]['total_bids'] += total_bid
                total_bids_by_hand_size[hand_size]['rounds'] += 1
                
                if total_bid < hand_size:
                    total_bids_by_hand_size[hand_size]['below_hand_size'] += 1
                elif total_bid == hand_size:
                    total_bids_by_hand_size[hand_size]['equal_hand_size'] += 1
                else:
                    total_bids_by_hand_size[hand_size]['above_hand_size'] += 1
                
                # Track individual bid distribution
                for bid in bids:
                    bid_distribution[bid] += 1
    
    print("=== Bid Totals vs Hand Size ===")
    for hand_size in sorted(total_bids_by_hand_size.keys()):
        stats = total_bids_by_hand_size[hand_size]
        if stats['rounds'] > 0:
            avg_total_bid = stats['total_bids'] / stats['rounds']
            expected_total = hand_size
            deviation = avg_total_bid - expected_total
            
            below_pct = (stats['below_hand_size'] / stats['rounds'] * 100) if stats['rounds'] > 0 else 0
            equal_pct = (stats['equal_hand_size'] / stats['rounds'] * 100) if stats['rounds'] > 0 else 0
            above_pct = (stats['above_hand_size'] / stats['rounds'] * 100) if stats['rounds'] > 0 else 0
            
            print(f"\nHand Size {hand_size:2}:")
            print(f"  Rounds: {stats['rounds']}")
            print(f"  Avg total bid: {avg_total_bid:.2f} (expected: {expected_total}, deviation: {deviation:+.2f})")
            print(f"  Below hand_size: {stats['below_hand_size']:4} ({below_pct:5.1f}%)")
            print(f"  Equal hand_size: {stats['equal_hand_size']:4} ({equal_pct:5.1f}%)")
            print(f"  Above hand_size: {stats['above_hand_size']:4} ({above_pct:5.1f}%)")
    
    print("\n=== Individual Bid Distribution ===")
    total_bids = sum(bid_distribution.values())
    for bid in sorted(bid_distribution.keys()):
        count = bid_distribution[bid]
        pct = (count / total_bids * 100) if total_bids > 0 else 0
        print(f"  Bid {bid:2}: {count:5} ({pct:5.1f}%)")

if __name__ == '__main__':
    if len(sys.argv) < 2:
        # Find latest JSONL file
        results_dir = Path('simulation-results')
        if results_dir.exists():
            jsonl_files = list(results_dir.glob('*.jsonl'))
            if jsonl_files:
                latest = max(jsonl_files, key=lambda p: p.stat().st_mtime)
                analyze_bids(latest)
            else:
                print("No JSONL files found in simulation-results/")
        else:
            print("Usage: python3 analyze_bids.py <path-to-jsonl-file>")
    else:
        analyze_bids(sys.argv[1])

