#!/usr/bin/env python3

def analyze_unsafe_coverage(filename="unsafe_coverage.stat"):
    """
    Analyze unsafe coverage statistics from the given file.
    Tracks coverage for lines starting with 'src/' across all test runs.
    """

    all_registered = set()
    all_executed = set()

    try:
        with open(filename, 'r') as f:
            lines = f.readlines()
    except FileNotFoundError:
        print(f"Error: File '{filename}' not found")
        return

    current_section = None

    for line in lines:
        line = line.strip()

        if line == "=== REGISTERED_LINES ===":
            current_section = "registered"
            continue
        elif line == "=== EXECUTED_LINES ===":
            current_section = "executed"
            continue
        elif line.startswith("=== ") or line == "":
            current_section = None
            continue

        # Process lines that start with 'src/'
        if current_section and line.startswith("src/"):
            if current_section == "registered":
                all_registered.add(line)
            elif current_section == "executed":
                all_executed.add(line)

    # Calculate coverage
    total_registered = len(all_registered)
    total_executed = len(all_executed)

    if total_registered == 0:
        coverage_percentage = 0.0
    else:
        coverage_percentage = (total_executed / total_registered) * 100

    # Print results
    print(f"=== OVERALL UNSAFE COVERAGE ANALYSIS ===")
    print(f"Total registered src/ lines: {total_registered}")
    print(f"Total executed src/ lines: {total_executed}")
    print(f"Coverage percentage: {coverage_percentage:.2f}%")
    print()

    # Show unexecuted lines
    unexecuted = all_registered - all_executed
    if unexecuted:
        print(f"Unexecuted src/ lines ({len(unexecuted)}):")
        for line in sorted(unexecuted):
            print(f"  {line}")
    else:
        print("All registered src/ lines were executed!")

if __name__ == "__main__":
    analyze_unsafe_coverage()