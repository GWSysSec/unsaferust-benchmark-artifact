import json
import statistics

def calculate_geomean(values):
    """Calculate geometric mean of a list of values"""
    if not values or any(v <= 0 for v in values):
        return 0.0
    product = 1.0
    for v in values:
        product *= v
    return product ** (1.0 / len(values))

def analyze_cpu_cycle_distribution():
    # Load both JSON files
    with open('externalfalse_nativefalse.json', 'r') as f:
        data_nativefalse = json.load(f)

    with open('externalfalse_nativetrue.json', 'r') as f:
        data_nativetrue = json.load(f)

    # Extract unsafe percentages for both datasets
    percentages_nativefalse = []
    percentages_nativetrue = []

    for crate_data in data_nativefalse['crates'].values():
        percentages_nativefalse.append(crate_data['unsafe_percentage'])

    for crate_data in data_nativetrue['crates'].values():
        percentages_nativetrue.append(crate_data['unsafe_percentage'])

    # Calculate statistics for both datasets
    def print_stats(percentages, label):
        zero_count = sum(1 for p in percentages if p == 0.0)
        non_zero_percentages = [p for p in percentages if p > 0.0]

        print(f"\n{label}:")
        print(f"  All crates ({len(percentages)} total, {zero_count} with 0%):")
        print(f"    Min: {min(percentages):.6f}%")
        print(f"    Max: {max(percentages):.6f}%")
        print(f"    Median: {statistics.median(percentages):.6f}%")
        print(f"    Geometric Mean: {calculate_geomean(percentages):.6f}%")

        if non_zero_percentages:
            print(f"  Excluding zeros ({len(non_zero_percentages)} crates):")
            print(f"    Min: {min(non_zero_percentages):.6f}%")
            print(f"    Max: {max(non_zero_percentages):.6f}%")
            print(f"    Median: {statistics.median(non_zero_percentages):.6f}%")
            print(f"    Geometric Mean: {calculate_geomean(non_zero_percentages):.6f}%")

    print_stats(percentages_nativefalse, "External=false, Native=false")
    print_stats(percentages_nativetrue, "External=false, Native=true")

if __name__ == "__main__":
    analyze_cpu_cycle_distribution()