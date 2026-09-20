#!/usr/bin/env python3
"""
Comparative visualization script for RQ1 vs RQ6 unsafe code execution percentages.
Cumulative Distribution Functions comparison.
"""

import argparse
import json
import sys

import numpy as np
import matplotlib
matplotlib.use('Agg')  # headless: savefig only, no display required
import matplotlib.pyplot as plt
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent))
import datasets  # noqa: E402


def load_data(json_file):
    """Load and parse the JSON data file."""
    with open(json_file, 'r') as f:
        data = json.load(f)
    return data

def extract_unsafe_percentages(data):
    """Extract unsafe_percentage values from all crates."""
    percentages = []
    crate_names = []
    for crate_name, crate_data in data['crates'].items():
        percentages.append(crate_data['unsafe_percentage'])
        crate_names.append(crate_name)
    return np.array(percentages), crate_names

def generate_cumulative_frequency(percentages):
    """Generate cumulative frequency distribution."""
    sorted_percentages = np.sort(percentages)
    n = len(sorted_percentages)
    cumulative_freq = np.arange(1, n + 1) / n
    # Ensure cumulative frequency doesn't exceed 1.0 due to floating point precision
    cumulative_freq = np.clip(cumulative_freq, 0, 1.0)
    return sorted_percentages, cumulative_freq

def create_comparative_cdf_plot(rq1_data, rq6_data, output_file=None):
    """Create single CDF plot showing full distribution with focused region marked."""

    # Create figure with single plot
    _, ax = plt.subplots(1, 1, figsize=(8, 6))

    # Generate CDFs
    rq1_sorted, rq1_cdf = generate_cumulative_frequency(rq1_data)
    rq6_sorted, rq6_cdf = generate_cumulative_frequency(rq6_data)

    # Color-blind safe colors
    color_rq1 = '#1f77b4'  # Blue
    color_rq6 = '#ff7f0e'  # Orange

    # Plot CDF lines
    ax.plot(rq1_sorted, rq1_cdf, linewidth=2, color=color_rq1, alpha=0.8,
            label='without standard libraries', linestyle='-')
    ax.plot(rq6_sorted, rq6_cdf, linewidth=2, color=color_rq6, alpha=0.8,
            label='with standard libraries', linestyle='--')

    # Add key statistical points as dotted lines
    # Medians
    rq1_median = np.median(rq1_data)
    rq6_median = np.median(rq6_data)
    ax.axvline(x=rq1_median, color=color_rq1, linestyle=':', alpha=0.6, linewidth=1)
    ax.axvline(x=rq6_median, color=color_rq6, linestyle=':', alpha=0.6, linewidth=1)

    # Add text annotations for medians
    ax.text(rq1_median + 0.5, 0.55, f'Median\nw/o: {rq1_median:.1f}%',
            fontsize=15, color=color_rq1, ha='left', va='center')
    ax.text(rq6_median + 0.5, 0.65, f'Median\nw/: {rq6_median:.1f}%',
            fontsize=15, color=color_rq6, ha='left', va='center')

    # 75th percentiles
    rq1_q75 = np.percentile(rq1_data, 75)
    rq6_q75 = np.percentile(rq6_data, 75)
    ax.axvline(x=rq1_q75, color=color_rq1, linestyle=':', alpha=0.4, linewidth=1)
    ax.axvline(x=rq6_q75, color=color_rq6, linestyle=':', alpha=0.4, linewidth=1)

    # Add horizontal line at 75% cumulative frequency
    ax.axhline(y=0.75, color='gray', linestyle=':', alpha=0.3, linewidth=1)
    ax.text(1, 0.76, '75%', fontsize=8, color='gray', ha='left', va='bottom')

    # Customize plot
    ax.set_xlabel('Percentage of Unsafe Code Execution (%)', fontsize=20)
    ax.set_ylabel('Cumulative Fraction of Crates', fontsize=20)
    #ax.set_title('Standard Libraries Impact on Unsafe Code Execution', fontsize=12, fontweight='bold')
    ax.grid(True, alpha=0.3)
    ax.legend(fontsize=18, loc='lower right')
    ax.set_xlim(0, 100)
    ax.set_ylim(0, 1)

    plt.tight_layout()

    if output_file:
        plt.savefig(output_file, dpi=300, bbox_inches='tight')
        print(f"CDF plot saved to: {output_file}")

    plt.show()


def print_comparative_statistics(rq1_data, rq6_data):
    """Print comparative statistics for RQ1 vs RQ6."""
    print("\n=== Comparative Statistics ===")
    print(f"Total number of programs: {len(rq1_data)}")
    
    print(f"\nRQ1 (User Code Only):")
    print(f"  Mean: {np.mean(rq1_data):.2f}%")
    print(f"  Median: {np.median(rq1_data):.2f}%")
    print(f"  Std Dev: {np.std(rq1_data):.2f}%")
    
    print(f"\nRQ6 (Including native lib):")
    print(f"  Mean: {np.mean(rq6_data):.2f}%")
    print(f"  Median: {np.median(rq6_data):.2f}%")
    print(f"  Std Dev: {np.std(rq6_data):.2f}%")
    
    delta = rq6_data - rq1_data
    print(f"\nnative Library Impact (Δ):")
    print(f"  Mean increase: {np.mean(delta):.2f}%")
    print(f"  Median increase: {np.median(delta):.2f}%")
    print(f"  Max increase: {np.max(delta):.2f}%")
    print(f"  Programs with increase: {np.sum(delta > 0)}/{len(delta)} ({100*np.sum(delta > 0)/len(delta):.1f}%)")

def main():
    """Main function to run the comparative analysis."""
    # Read the two committed JSON inputs (produced from rebench_v3 by
    # port_v3_to_json.py). RQ1 = without_native (user code only),
    # RQ6 = with_native (including std/native libs). Decoupled from
    # aggregate_rebench so the figure tracks exactly the same numbers the
    # table reads.
    ap = argparse.ArgumentParser(description=__doc__)
    datasets.add_argument(ap)
    ds = datasets.get(ap.parse_args().dataset)
    wo_name = f"{ds.stem('cpucycle_withoutnative')}.json"
    w_name = f"{ds.stem('cpucycle_withnative')}.json"
    print(f"dataset {ds.name}: {ds.scope}")
    print(f"Loading RQ1 (without_native) from {wo_name}...")
    rq1_data_full = load_data(HERE / wo_name)
    print(f"Loading RQ6 (with_native) from {w_name}...")
    rq6_data_full = load_data(HERE / w_name)

    rq1_percentages, rq1_crates = extract_unsafe_percentages(rq1_data_full)
    rq6_percentages, rq6_crates = extract_unsafe_percentages(rq6_data_full)

    # Verify same crates
    if not np.array_equal(rq1_crates, rq6_crates):
        print("Warning: Different crates in datasets")
    
    # Print comparative statistics
    print_comparative_statistics(rq1_percentages, rq6_percentages)
    
    # Create comparative CDF plot with inset
    print("\nCreating comparative CDF plot with inset...")
    figdir = HERE.parent / "Latex" / "figures"
    stem = ds.stem("cumulative_frequency_unsafe_execution_rq1_rq6")
    for ext in ("png", "pdf"):
        create_comparative_cdf_plot(rq1_percentages, rq6_percentages,
                                    str(figdir / f"{stem}.{ext}"))

if __name__ == "__main__":
    main()