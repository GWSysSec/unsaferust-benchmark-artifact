# Docker Setup Guide for Unsafe Rust Benchmark

This guide will help you build and run the Unsafe Rust Benchmark artifact in a Docker container, isolated from your production environment.

## Prerequisites

- Docker installed (version 20.10 or later recommended)
- At least 20GB of free disk space
- At least 8GB of RAM available for Docker
- 2-4 hours for the initial build

## Setup

This artifact uses **Git Submodules** to manage dependencies. To set up the repository:

```bash
git clone --recursive git@github.com:GWSysSec/unsaferust-benchmark-artifact.git
cd unsaferust-benchmark-artifact
```

If you have already cloned it without submodules:

```bash
./update-repos.sh
# OR manually:
git submodule update --init --recursive
```

## Directory Structure

- `unsafe-rust-benchmark/`: [Submodule] The modified Rust compiler and benchmarks (oscardev branch).
- `external/perf-lib/`: [Submodule] The external performance instrumentation library.
- `perf/`: [Symlink] Points to `external/perf-lib/lib/perf`, used by the Docker build.

### Option 2: Using docker-compose

```bash
# Build the image
docker-compose build

# Run the container
docker-compose run --rm unsaferust-bench
```

### Option 3: Manual Docker commands

```bash
# Build the image
docker build -t unsaferust-bench:local .

# Run the container
docker run -it unsaferust-bench:local
```

## Using the Container

Once inside the container, you'll be in the `/workspace/perf` directory.

### Step 1: Build an instrumentation tool

Choose ONE instrumentation at a time:

```bash
# For unsafe coverage tracking
make coverage

# For unsafe instruction/function counting
make counter

# For heap usage tracking
make heap

# For CPU cycle tracking
make cpu
```

### Step 2: Source the environment

After building an instrumentation, set up the environment:

```bash
cd env

# Source the corresponding environment file
source coverage.sh    # for coverage
# OR
source counter.sh     # for counter
# OR
source heap.sh        # for heap
# OR
source cpu.sh         # for cpu
```

### Step 3: Run a benchmark

Navigate to a benchmark and run it:

```bash
cd /workspace/benchmarks/arrayvec-0.7.6
cargo bench
```

### Step 4: View results

Results are stored in `/tmp/*.stat` files:

```bash
# For coverage
cat /tmp/unsafe_coverage.stat

# For heap tracking
cat /tmp/heap_stat.stat

# For CPU cycles
cat /tmp/cpu_cycle.stat
```

## Example Workflow

Complete example running coverage instrumentation:

```bash
# Inside the container
cd /workspace/perf
make coverage
cd env
source coverage.sh

# Run a benchmark
cd /workspace/benchmarks/arrayvec-0.7.6
cargo bench

# View results
cat /tmp/unsafe_coverage.stat
```

## Available Benchmarks

Benchmarks are located in `/workspace/benchmarks/`. Some examples:

- `arrayvec-0.7.6` - Array-backed vector implementation
- `serde-*` - Serialization framework
- `regex-*` - Regular expression engine
- And many more...

List all available benchmarks:

```bash
ls /workspace/benchmarks/
```

## Persisting Results

Results can be copied from the container to your host machine:

### Method 1: Docker cp

```bash
# From your host machine (in another terminal)
docker cp unsaferust-benchmark:/tmp/unsafe_coverage.stat ./results/
```

### Method 2: Using volumes (with docker-compose)

When using docker-compose, a `results` directory is mounted. Copy files there:

```bash
# Inside container
mkdir -p /workspace/results
cp /tmp/*.stat /workspace/results/
```

Files will appear in `./results/` on your host machine.

## Troubleshooting

### Build fails with "manifest path does not exist"

This is a known issue with the repository structure. The Dockerfile handles this by:
1. Initializing submodules properly
2. Retrying the build if it fails initially

### Out of memory during build

Increase Docker's memory allocation:
- Docker Desktop: Settings → Resources → Memory (set to at least 8GB)
- Linux: Modify `/etc/docker/daemon.json`

### Instrumentation not showing data

Make sure you:
1. Built the instrumentation with `make <tool>`
2. Sourced the correct environment file
3. Only use ONE instrumentation at a time

### Cargo failures

If cargo fails to find the custom compiler:
1. Verify you sourced the environment: `source env/coverage.sh`
2. Check the compiler path: `which rustc`
3. Should show: `/workspace/unsafe-rust-benchmark/build/.../rustc`

## Rebuilding Instrumentation

To switch to a different instrumentation:

```bash
cd /workspace/perf
make clean
make <new-instrumentation>
cd env
source <new-instrumentation>.sh
```

## Docker Image Management

```bash
# List images
docker images

# Remove the image
docker rmi unsaferust-bench:local

# Remove all stopped containers
docker container prune

# Clean up everything (BE CAREFUL!)
docker system prune -a
```

## Advanced Usage

### Running benchmarks in batch

```bash
# Create a script inside the container
cat > /workspace/run_all.sh << 'EOF'
#!/bin/bash
for bench in /workspace/benchmarks/*/; do
    echo "Running benchmark: $bench"
    cd "$bench"
    cargo bench || echo "Failed: $bench"
done
EOF

chmod +x /workspace/run_all.sh
/workspace/run_all.sh
```

### Extracting all results

```bash
# Inside container
cd /workspace
tar czf results.tar.gz /tmp/*.stat
# Then copy out: docker cp unsaferust-benchmark:/workspace/results.tar.gz ./
```

## Building Without Docker

If you prefer to build directly on your host system, see the main [README.md](README.md) for instructions. However, this will modify your system's Rust installation.
