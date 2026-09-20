import argparse
import json
import math
import sys
import numpy as np
import matplotlib
matplotlib.use('Agg')  # headless: savefig only, no display required
import matplotlib.pyplot as plt
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent))
import datasets  # noqa: E402


def load_data(ds):
    """Load the two native-library variants of one measurement run."""
    with open(HERE / f"{ds.stem('heap_with_native')}.json") as f:
        data_with = json.load(f)

    with open(HERE / f"{ds.stem('heap_without_native')}.json") as f:
        data_without = json.load(f)

    return data_with, data_without

def categorize_memory_sizes(size_histogram):
    """
    Categorize memory consumption (in bytes) into three size ranges based on histogram bins.
    Returns the total memory consumed in each category.

    Bin size ranges and their midpoints (used for estimation):
    Bin 0: 0-1K (midpoint: 512B)
    Bin 1: 1K-2K (midpoint: 1.5KB = 1536B)
    Bin 2: 2K-4K (midpoint: 3KB = 3072B)
    Bin 3: 4K-8K (midpoint: 6KB = 6144B)
    Bin 4: 8K-16K (midpoint: 12KB = 12288B)
    Bin 5: 16K-32K (midpoint: 24KB = 24576B)
    Bin 6: 32K-64K (midpoint: 48KB = 49152B)
    Bin 7: 64K-128K (midpoint: 96KB = 98304B)
    Bin 8+: ≥128K (midpoint: 192KB = 196608B for bin 8, doubles for each subsequent bin)

    Small: ≤1K (bin 0)
    Medium: 1K-128K (bins 1-7)
    Large: ≥128K (bins 8+)
    """
    if len(size_histogram) < 9:
        return 0, 0, 0

    # Bin midpoints in bytes
    bin_midpoints = [
        512,      # Bin 0: 0-1K
        1536,     # Bin 1: 1K-2K
        3072,     # Bin 2: 2K-4K
        6144,     # Bin 3: 4K-8K
        12288,    # Bin 4: 8K-16K
        24576,    # Bin 5: 16K-32K
        49152,    # Bin 6: 32K-64K
        98304,    # Bin 7: 64K-128K
    ]

    # Calculate memory for small allocations
    small_memory = size_histogram[0] * bin_midpoints[0]

    # Calculate memory for medium allocations
    medium_memory = sum(size_histogram[i] * bin_midpoints[i] for i in range(1, 8))

    # Calculate memory for large allocations
    # For bins 8+, use 128K * 2^(bin-8) as midpoint
    large_memory = 0
    for i in range(8, len(size_histogram)):
        midpoint = 196608 * (2 ** (i - 8))  # 192KB for bin 8, doubles each bin
        large_memory += size_histogram[i] * midpoint

    return small_memory, medium_memory, large_memory

def is_valid_crate(crate):
    """Check if crate has valid data (unsafe <= total for all bins)"""
    stats = crate['aggregated_stats']
    for total, unsafe in zip(stats['size_histogram'], stats['unsafe_size_histogram']):
        if unsafe > total:
            return False
    return True

def extract_heap_data(data_with, data_without):
    """Extract heap memory data and calculate unsafe heap percentages using memory sizes"""
    # Filter out problematic crates
    valid_with = [crate for crate in data_with['crates'] if is_valid_crate(crate)]
    valid_without = [crate for crate in data_without['crates'] if is_valid_crate(crate)]

    # Create dictionaries for quick lookup
    with_data = {}
    without_data = {}

    for crate in valid_with:
        stats = crate['aggregated_stats']
        small_mem, medium_mem, large_mem = categorize_memory_sizes(stats['size_histogram'])
        unsafe_small_mem, unsafe_medium_mem, unsafe_large_mem = categorize_memory_sizes(stats['unsafe_size_histogram'])

        total_memory = small_mem + medium_mem + large_mem
        total_unsafe_mem = unsafe_small_mem + unsafe_medium_mem + unsafe_large_mem

        with_data[crate['crate_name']] = {
            'total': total_memory,
            'unsafe': total_unsafe_mem
        }

    for crate in valid_without:
        stats = crate['aggregated_stats']
        small_mem, medium_mem, large_mem = categorize_memory_sizes(stats['size_histogram'])
        unsafe_small_mem, unsafe_medium_mem, unsafe_large_mem = categorize_memory_sizes(stats['unsafe_size_histogram'])

        total_memory = small_mem + medium_mem + large_mem
        total_unsafe_mem = unsafe_small_mem + unsafe_medium_mem + unsafe_large_mem

        without_data[crate['crate_name']] = {
            'total': total_memory,
            'unsafe': total_unsafe_mem
        }

    # Find common crates
    common_crates = set(with_data.keys()).intersection(set(without_data.keys()))

    # Extract data
    with_percentages = []
    without_percentages = []
    crate_names = []

    for crate_name in sorted(common_crates):
        # Calculate unsafe heap memory percentages
        with_total = with_data[crate_name]['total']
        with_unsafe = with_data[crate_name]['unsafe']
        without_total = without_data[crate_name]['total']
        without_unsafe = without_data[crate_name]['unsafe']

        with_pct = (with_unsafe / with_total * 100) if with_total > 0 else 0
        without_pct = (without_unsafe / without_total * 100) if without_total > 0 else 0

        with_percentages.append(with_pct)
        without_percentages.append(without_pct)
        crate_names.append(crate_name)

    return {
        'with_percentages': with_percentages,
        'without_percentages': without_percentages,
        'crate_names': crate_names
    }

def create_cdf_plot(data):
    """Create CDF comparison plot"""
    # High contrast colorblind-friendly colors
    colors = {'with': '#D55E00', 'without': '#0173B2'}  # Orange and Blue

    # Set up figure with 4:3 aspect ratio
    fig, ax = plt.subplots(1, 1, figsize=(8, 6))
    
    # CDF comparison
    sorted_with = np.sort(data['with_percentages'])
    sorted_without = np.sort(data['without_percentages'])
    y_vals = np.arange(1, len(sorted_with) + 1) / len(sorted_with)
    y_vals_without = np.arange(1, len(sorted_without) + 1) / len(sorted_without)
    
    # Calculate medians for legend
    with_median = np.median(data['with_percentages'])
    without_median = np.median(data['without_percentages'])

    ax.plot(sorted_without, y_vals_without, color=colors['without'], linewidth=2,
            label=f'without standard libraries')
    ax.plot(sorted_with, y_vals, color=colors['with'], linewidth=2,
            label=f'with standard libraries')
    ax.set_xlabel('Percentage of Unsafe Heap Usage (%)', fontsize=20)
    ax.set_ylabel('Cumulative Fraction of Crates', fontsize=20)
    #ax.set_title('Standard Libraries Impact on Unsafe Heap Memory Usage')
    ax.grid(True, alpha=0.3)
    ax.legend(loc='lower center', bbox_to_anchor=(0.5, 0), fontsize=20)
    ax.set_xlim(0, 100)
    ax.set_ylim(0, 1)
    
    
    # Adjust layout
    plt.tight_layout()
    
    return fig

def main():
    """Generate the heap CDF and comparison plots"""
    # Load data
    ap = argparse.ArgumentParser(description=__doc__)
    datasets.add_argument(ap)
    ds = datasets.get(ap.parse_args().dataset)
    print(f"dataset {ds.name}: {ds.scope}")

    data_with, data_without = load_data(ds)
    
    # Extract and process data
    heap_data = extract_heap_data(data_with, data_without)
    
    # Create the plot
    fig = create_cdf_plot(heap_data)
    
    # Save the plot
    output_dir = HERE.parent / "Latex" / "figures"
    output_dir.mkdir(parents=True, exist_ok=True)

    stem = ds.stem("heap_native_cdf")
    output_file = output_dir / f"{stem}.pdf"
    fig.savefig(output_file, dpi=300, bbox_inches='tight', format='pdf')

    # Also save as PNG for preview
    output_file_png = output_dir / f"{stem}.png"
    fig.savefig(output_file_png, dpi=300, bbox_inches='tight', format='png')
    
    print(f"Plot saved to: {output_file}")
    print(f"Preview saved to: {output_file_png}")
    
    # Print statistics
    print(f"\nStatistics:")
    print(f"Number of crates analyzed: {len(heap_data['crate_names'])}")
    print(f"With native libraries - Median unsafe heap %: {np.median(heap_data['with_percentages']):.2f}%")
    print(f"Without native libraries - Median unsafe heap %: {np.median(heap_data['without_percentages']):.2f}%")
    print(f"With native libraries - Mean unsafe heap %: {np.mean(heap_data['with_percentages']):.2f}%")
    print(f"Without native libraries - Mean unsafe heap %: {np.mean(heap_data['without_percentages']):.2f}%")

if __name__ == "__main__":
    main()