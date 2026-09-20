#!/usr/bin/env python3
import json
import statistics
from typing import List, Dict

def geometric_mean(values: List[float]) -> float:
    """Calculate geometric mean of a list of values."""
    if not values or any(v <= 0 for v in values):
        # Filter out zeros for geometric mean
        values = [v for v in values if v > 0]
        if not values:
            return 0.0
    product = 1.0
    for v in values:
        product *= v
    return product ** (1.0 / len(values))

def analyze_file(filepath: str, label: str):
    """Analyze unsafe memory instruction patterns in a JSON file."""
    print(f"\n{'='*80}")
    print(f"Analysis for: {label}")
    print(f"{'='*80}")

    with open(filepath, 'r') as f:
        data = json.load(f)

    crates = data['crates']

    # Calculate percentages for each crate
    percentages = []
    heap_alloc_ratios = []

    for crate in crates:
        name = crate['crate_name']
        stats = crate['aggregated_stats']

        unsafe_mem_inst = stats['unsafe_memory_instructions']
        unsafe_load = stats['unsafe_load']
        unsafe_store = stats['unsafe_store']
        heap_allocs = stats['total_heap_allocations']

        # Calculate (load + store) / instructions percentage
        if unsafe_mem_inst > 0:
            percentage = ((unsafe_load + unsafe_store) / unsafe_mem_inst) * 100
            percentages.append(percentage)

            # Also track relationship between memory instructions and heap allocations
            if heap_allocs > 0:
                ratio = unsafe_mem_inst / heap_allocs
                heap_alloc_ratios.append(ratio)

    # Print statistics for (load + store) / instructions
    print(f"\n(Unsafe Load + Unsafe Store) / Unsafe Memory Instructions (%)")
    print(f"-" * 80)
    if percentages:
        print(f"Min:     {min(percentages):>10.2f}%")
        print(f"Max:     {max(percentages):>10.2f}%")
        print(f"Median:  {statistics.median(percentages):>10.2f}%")
        print(f"Mean:    {statistics.mean(percentages):>10.2f}%")
        print(f"Geomean: {geometric_mean(percentages):>10.2f}%")
        print(f"Samples: {len(percentages)} crates")
    else:
        print("No data available")

    # Print statistics for memory instructions vs heap allocations
    print(f"\nUnsafe Memory Instructions / Total Heap Allocations (ratio)")
    print(f"-" * 80)
    if heap_alloc_ratios:
        print(f"Min:     {min(heap_alloc_ratios):>10.2f}x")
        print(f"Max:     {max(heap_alloc_ratios):>10.2f}x")
        print(f"Median:  {statistics.median(heap_alloc_ratios):>10.2f}x")
        print(f"Mean:    {statistics.mean(heap_alloc_ratios):>10.2f}x")
        print(f"Geomean: {geometric_mean(heap_alloc_ratios):>10.2f}x")
        print(f"Samples: {len(heap_alloc_ratios)} crates")
    else:
        print("No data available")

    # Show some examples of crates with extreme values
    print(f"\nTop 5 crates by (Load + Store) / Instructions ratio:")
    print(f"-" * 80)
    crate_percentages = []
    for crate in crates:
        stats = crate['aggregated_stats']
        unsafe_mem_inst = stats['unsafe_memory_instructions']
        if unsafe_mem_inst > 0:
            percentage = ((stats['unsafe_load'] + stats['unsafe_store']) / unsafe_mem_inst) * 100
            crate_percentages.append((crate['crate_name'], percentage, stats['unsafe_load'], stats['unsafe_store'], unsafe_mem_inst))

    crate_percentages.sort(key=lambda x: x[1], reverse=True)
    for name, pct, load, store, total in crate_percentages[:5]:
        print(f"{name:20s}: {pct:6.2f}% (load={load:>12,}, store={store:>12,}, total={total:>12,})")

    print(f"\nBottom 5 crates by (Load + Store) / Instructions ratio:")
    print(f"-" * 80)
    for name, pct, load, store, total in crate_percentages[-5:]:
        print(f"{name:20s}: {pct:6.2f}% (load={load:>12,}, store={store:>12,}, total={total:>12,})")

def main():
    print("\n" + "="*80)
    print("Unsafe Memory Instruction Analysis")
    print("="*80)

    # Analyze with native (native=true)
    analyze_file(
        '/Users/oscar/projects/rust-bench-paper/heaptracker_rq2/heap_with_native.json',
        'WITH NATIVE (native=true)'
    )

    # Analyze without native (native=false)
    analyze_file(
        '/Users/oscar/projects/rust-bench-paper/heaptracker_rq2/heap_without_native.json',
        'WITHOUT NATIVE (native=false)'
    )

    print(f"\n{'='*80}")
    print("Analysis Complete")
    print(f"{'='*80}\n")

if __name__ == '__main__':
    main()
