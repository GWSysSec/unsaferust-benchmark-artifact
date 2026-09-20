#!/usr/bin/env python3
"""
Filter JSON files to keep only the top 100 crates by unsafe allocation percentage.
Uses the with_native data as the base case to identify top crates.
"""

import json
import sys
from pathlib import Path


def calculate_unsafe_percentage(crate_data):
    """Calculate overall unsafe allocation percentage for a crate"""
    stats = crate_data['aggregated_stats']
    total_allocs = sum(stats['size_histogram'])
    total_unsafe = sum(stats['unsafe_size_histogram'])

    return (total_unsafe / total_allocs * 100) if total_allocs > 0 else 0


def find_top_crates_from_with_native(json_file_path, top_n=100):
    """Find top N crates by unsafe allocation percentage from with_native data"""

    with open(json_file_path, 'r') as f:
        data = json.load(f)

    # Calculate unsafe percentage for each crate
    crate_stats = []
    for crate in data['crates']:
        crate_name = crate['crate_name']
        unsafe_percentage = calculate_unsafe_percentage(crate)

        crate_stats.append({
            'name': crate_name,
            'unsafe_percentage': unsafe_percentage,
            'data': crate
        })

    # Sort by unsafe percentage (descending) and get top N
    crate_stats.sort(key=lambda x: x['unsafe_percentage'], reverse=True)

    return crate_stats[:top_n]


def filter_json_file(input_file, output_file, target_crate_names):
    """Filter JSON file to keep only crates in target_crate_names list"""

    with open(input_file, 'r') as f:
        data = json.load(f)

    # Filter crates to keep only those in target list
    filtered_crates = []
    target_set = set(target_crate_names)

    for crate in data['crates']:
        if crate['crate_name'] in target_set:
            filtered_crates.append(crate)

    # Update the data structure
    data['crates'] = filtered_crates

    # Update metadata
    if 'metadata' in data:
        data['metadata']['total_crates'] = len(filtered_crates)
        data['metadata']['filter_applied'] = f"Top 100 crates by unsafe allocation % from with_native"

    # Write filtered data
    with open(output_file, 'w') as f:
        json.dump(data, f, indent=2)

    return len(filtered_crates)


def main():
    # Define file paths
    base_dir = Path("heaptracker_rq2")
    with_native_file = base_dir / "heap_with_native.json"
    without_native_file = base_dir / "heap_without_native.json"

    # Check if files exist
    if not with_native_file.exists():
        print(f"Error: {with_native_file} not found")
        return 1

    if not without_native_file.exists():
        print(f"Error: {without_native_file} not found")
        return 1

    print("Step 1: Finding top 100 crates by unsafe allocation % from with_native data...")

    # Find top 100 crates from with_native data
    top_crates = find_top_crates_from_with_native(with_native_file, top_n=100)
    target_crate_names = [crate['name'] for crate in top_crates]

    print(f"Found top 100 crates. Top 10:")
    for i, crate in enumerate(top_crates[:10], 1):
        print(f"  {i:2d}. {crate['name']}: {crate['unsafe_percentage']:.1f}%")

    print(f"\nStep 2: Filtering with_native JSON to keep only top 100 crates...")

    # Filter with_native file
    filtered_count_with = filter_json_file(
        with_native_file,
        with_native_file,
        target_crate_names
    )

    print(f"Filtered {with_native_file}: kept {filtered_count_with} crates")

    print(f"\nStep 3: Filtering without_native JSON to keep only those same 100 crates...")

    # Filter without_native file
    filtered_count_without = filter_json_file(
        without_native_file,
        without_native_file,
        target_crate_names
    )

    print(f"Filtered {without_native_file}: kept {filtered_count_without} crates")

    print(f"\nFiltering complete!")
    print(f"Both JSON files now contain only the top 100 crates by unsafe allocation percentage.")

    if filtered_count_with != filtered_count_without:
        print(f"\nWarning: Different number of crates found in each file:")
        print(f"  with_native: {filtered_count_with}")
        print(f"  without_native: {filtered_count_without}")
        print("Some crates from the top 100 list may not exist in the without_native data.")

    return 0


if __name__ == "__main__":
    sys.exit(main())