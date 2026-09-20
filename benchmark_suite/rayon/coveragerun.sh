#!/usr/bin/env bash
set -u  # no -e, so it won't exit on failure
set -o pipefail

failed=()

crate_dir="rayon-demo"

if [[ -f "$crate_dir/Cargo.toml" ]]; then
    echo "==> Running benchmarks for $crate_dir"

    (
        cd "$crate_dir"
        
        # Add debug profile sections if they don't exist
        if ! grep -q "^\[profile\.release\]" Cargo.toml; then
            echo "" >> Cargo.toml
            echo "[profile.release]" >> Cargo.toml
            echo "debug = true" >> Cargo.toml
        elif ! grep -A10 "^\[profile\.release\]" Cargo.toml | grep -q "^debug\s*="; then
            # profile.release exists but doesn't have any debug setting
            sed -i '/^\[profile\.release\]/a debug = true' Cargo.toml
        fi
        
        if ! grep -q "^\[profile\.bench\]" Cargo.toml; then
            echo "" >> Cargo.toml
            echo "[profile.bench]" >> Cargo.toml
            echo "debug = true" >> Cargo.toml
        elif ! grep -A10 "^\[profile\.bench\]" Cargo.toml | grep -q "^debug\s*="; then
            # profile.bench exists but doesn't have any debug setting
            sed -i '/^\[profile\.bench\]/a debug = true' Cargo.toml
        fi
        
        cargo clean
        log_file="../${crate_dir}-bench.log"

        if ! /usr/bin/time -p cargo bench > "$log_file" 2>&1; then
            echo "!! Benchmark failed for $crate_dir"
            failed+=("$crate_dir")
        fi

        # Check for global /tmp/unsafe_coverage.stat
        if [[ -f /tmp/unsafe_coverage.stat ]]; then
            echo "" >> "$log_file"
            echo "=== /tmp/unsafe_coverage.stat ===" >> "$log_file"
            cat /tmp/unsafe_coverage.stat >> "$log_file"
            rm -f /tmp/unsafe_coverage.stat
        fi
    )
else
    echo "Skipping $crate_dir (no Cargo.toml)"
fi

echo
echo "==== Summary ===="
if [[ ${#failed[@]} -eq 0 ]]; then
    echo "All benchmarks completed successfully."
else
    echo "Failed crates:"
    for f in "${failed[@]}"; do
        echo "   - $f"
    done
fi