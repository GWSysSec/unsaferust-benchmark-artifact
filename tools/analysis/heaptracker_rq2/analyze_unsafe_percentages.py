#!/usr/bin/env python3
"""
Script to analyze unsafe allocation percentages in heap tracking data.
Classifies allocations by size: small (≤1K) and large (≥128K).
"""

import json
import sys
from typing import Dict, List, Tuple

def get_bin_size_ranges() -> List[Tuple[int, int]]:
    """
    Define the size ranges for each histogram bin.
    Based on power-of-2 allocation size bins starting from 1 byte.
    """
    # Allocation size bins (in bytes) - power of 2 progression
    # Bin 0: 1-1 bytes
    # Bin 1: 2-3 bytes
    # Bin 2: 4-7 bytes
    # Bin 3: 8-15 bytes
    # Bin 4: 16-31 bytes
    # Bin 5: 32-63 bytes
    # Bin 6: 64-127 bytes
    # Bin 7: 128-255 bytes
    # Bin 8: 256-511 bytes
    # Bin 9: 512-1023 bytes (1K boundary)
    # Bin 10: 1024-2047 bytes
    # Bin 11: 2048-4095 bytes
    # Bin 12: 4096-8191 bytes
    # Bin 13: 8192+ bytes (could include 128K+)

    ranges = []
    for i in range(14):  # 14 bins total
        if i == 0:
            start = 1
            end = 1
        elif i == 1:
            start = 2
            end = 3
        else:
            start = 2 ** (i)
            end = (2 ** (i + 1)) - 1
            if i == 13:  # Last bin is open-ended
                ranges.append((start, float('inf')))
                continue

        ranges.append((start, end))

    return ranges

def classify_bins() -> Tuple[List[int], List[int]]:
    """
    Classify bins into small (≤1K) and large (≥128K) categories.
    Based on generate_heap_table.py categorization:
    Small: bin 0 only (≤1K)
    Large: bins 8+ (≥128K)
    Returns: (small_bins, large_bins)
    """
    small_bins = [0]  # Only bin 0 is small (≤1K)
    large_bins = list(range(8, 14))  # Bins 8+ are large (≥128K)

    return small_bins, large_bins

def calculate_unsafe_percentage(total_bins: List[int], unsafe_bins: List[int],
                              target_bins: List[int]) -> float:
    """
    Calculate unsafe percentage for specific bins.
    unsafe% = (sum of unsafe counts in target bins) / (sum of total counts in target bins) * 100
    """
    total_count = sum(total_bins[i] for i in target_bins if i < len(total_bins))
    unsafe_count = sum(unsafe_bins[i] for i in target_bins if i < len(unsafe_bins))

    if total_count == 0:
        return 0.0

    return (unsafe_count / total_count) * 100

def analyze_crates(data: Dict) -> None:
    """
    Analyze all crates and find top unsafe percentages for small and large allocations.
    """
    small_bins, large_bins = classify_bins()

    ranges = get_bin_size_ranges()
    print("Bin size ranges:")
    for i, (min_size, max_size) in enumerate(ranges):
        if max_size == float('inf'):
            print(f"  Bin {i}: {min_size}+ bytes")
        else:
            print(f"  Bin {i}: {min_size}-{max_size} bytes")

    print(f"\nSmall allocation bins (≤1K): {small_bins}")
    print(f"Large allocation bins (≥128K): {large_bins}")
    print()

    small_results = []
    large_results = []

    for crate in data['crates']:
        crate_name = crate['crate_name']
        stats = crate['aggregated_stats']

        total_histogram = stats['size_histogram']
        unsafe_histogram = stats['unsafe_size_histogram']

        # Calculate unsafe percentages
        small_unsafe_pct = calculate_unsafe_percentage(total_histogram, unsafe_histogram, small_bins)
        large_unsafe_pct = calculate_unsafe_percentage(total_histogram, unsafe_histogram, large_bins)

        # Calculate total counts for context
        small_total = sum(total_histogram[i] for i in small_bins if i < len(total_histogram))
        small_unsafe = sum(unsafe_histogram[i] for i in small_bins if i < len(unsafe_histogram))

        large_total = sum(total_histogram[i] for i in large_bins if i < len(total_histogram))
        large_unsafe = sum(unsafe_histogram[i] for i in large_bins if i < len(unsafe_histogram))

        if small_total > 0:
            small_results.append({
                'crate': crate_name,
                'unsafe_pct': small_unsafe_pct,
                'total_allocs': small_total,
                'unsafe_allocs': small_unsafe
            })

        if large_total > 0:
            large_results.append({
                'crate': crate_name,
                'unsafe_pct': large_unsafe_pct,
                'total_allocs': large_total,
                'unsafe_allocs': large_unsafe
            })

    # Sort by unsafe percentage (descending)
    small_results.sort(key=lambda x: x['unsafe_pct'], reverse=True)
    large_results.sort(key=lambda x: x['unsafe_pct'], reverse=True)

    # Print top results
    print("="*80)
    print("TOP CRATES BY UNSAFE % - SMALL ALLOCATIONS (≤1K)")
    print("="*80)
    print(f"{'Rank':<4} {'Crate':<20} {'Unsafe%':<10} {'Unsafe/Total':<15} {'Total Allocs':<12}")
    print("-" * 80)

    for i, result in enumerate(small_results[:15], 1):
        if result['unsafe_pct'] > 0:  # Only show crates with unsafe allocations
            print(f"{i:<4} {result['crate']:<20} {result['unsafe_pct']:<10.2f} "
                  f"{result['unsafe_allocs']}/{result['total_allocs']:<10} {result['total_allocs']:<12}")

    print("\n" + "="*80)
    print("TOP CRATES BY UNSAFE % - LARGE ALLOCATIONS (≥128K)")
    print("="*80)
    print(f"{'Rank':<4} {'Crate':<20} {'Unsafe%':<10} {'Unsafe/Total':<15} {'Total Allocs':<12}")
    print("-" * 80)

    for i, result in enumerate(large_results[:15], 1):
        if result['unsafe_pct'] > 0:  # Only show crates with unsafe allocations
            print(f"{i:<4} {result['crate']:<20} {result['unsafe_pct']:<10.2f} "
                  f"{result['unsafe_allocs']}/{result['total_allocs']:<10} {result['total_allocs']:<12}")

    # Summary statistics
    small_with_unsafe = [r for r in small_results if r['unsafe_pct'] > 0]
    large_with_unsafe = [r for r in large_results if r['unsafe_pct'] > 0]

    print("\n" + "="*80)
    print("SUMMARY STATISTICS")
    print("="*80)
    print(f"Small allocations (≤1K):")
    print(f"  Total crates with small allocations: {len(small_results)}")
    print(f"  Crates with unsafe small allocations: {len(small_with_unsafe)}")
    if small_with_unsafe:
        print(f"  Highest unsafe %: {small_with_unsafe[0]['unsafe_pct']:.2f}% ({small_with_unsafe[0]['crate']})")
        avg_unsafe_pct = sum(r['unsafe_pct'] for r in small_with_unsafe) / len(small_with_unsafe)
        print(f"  Average unsafe % (among crates with unsafe): {avg_unsafe_pct:.2f}%")

    print(f"\nLarge allocations (≥128K):")
    print(f"  Total crates with large allocations: {len(large_results)}")
    print(f"  Crates with unsafe large allocations: {len(large_with_unsafe)}")
    if large_with_unsafe:
        print(f"  Highest unsafe %: {large_with_unsafe[0]['unsafe_pct']:.2f}% ({large_with_unsafe[0]['crate']})")
        avg_unsafe_pct = sum(r['unsafe_pct'] for r in large_with_unsafe) / len(large_with_unsafe)
        print(f"  Average unsafe % (among crates with unsafe): {avg_unsafe_pct:.2f}%")

def main():
    if len(sys.argv) != 2:
        print("Usage: python analyze_unsafe_percentages.py <heap_data.json>")
        sys.exit(1)

    input_file = sys.argv[1]

    try:
        with open(input_file, 'r') as f:
            data = json.load(f)

        print(f"Analyzing heap tracking data from: {input_file}")
        print(f"Processing date: {data['metadata']['processing_date']}")
        print(f"Total crates: {data['metadata']['total_crates']}")
        print(f"Filter applied: {data['metadata']['filter_applied']}")
        print()

        analyze_crates(data)

    except FileNotFoundError:
        print(f"Error: File {input_file} not found")
        sys.exit(1)
    except json.JSONDecodeError:
        print(f"Error: Invalid JSON in {input_file}")
        sys.exit(1)
    except Exception as e:
        print(f"Error: {e}")
        sys.exit(1)

if __name__ == "__main__":
    main()