# Runtime library for performance measurement

Currently we measure

- Total CPU clock cycles
- Heap usage  
- Unsafe code coverage

## Compile and Link

To enable the runtime libraries, compile them separately in the `perf` directory:

### Individual Features

```bash
make heap      # Heap usage tracking
make cpu       # CPU cycle counting
make coverage  # Unsafe code coverage
```

### Combined Features

```bash
make both        # Heap + CPU cycle counting
make all_three   # All three features: heap + cpu + coverage
```

It will generate `target/release/libunsafe_perf.rlib` and its dependent libraries
in `target/release/deps`.

To link this library, in the target crate's `.cargo/config.toml`, add

```toml
[build]
rustflags = [
  "-Z", "unstable-options",
  "--extern", "force:unsafe_perf=/you_path/perf/target/release/libunsafe_perf.rlib",
  "-L", "/you_path/perf/target/release/deps"
]
```

## Check output

If compiled and linked correctly, the target program writes one JSON file per
binary under `$UNSAFE_STAT_DIR` (or `/tmp/` if it is unset):

- **Heap tracking**: `<pid>.<binary>.heap.json`
- **CPU cycles**: `<pid>.<binary>.cpu_cycle.json`
- **Unsafe coverage**: `<pid>.<binary>.coverage.json`

## Available Make Targets

- `make heap` - Build with heap tracker only
- `make cpu` - Build with CPU cycle counter only
- `make coverage` - Build with unsafe coverage only
- `make both` - Build with heap tracker and CPU cycle counter
- `make all_three` - Build with all three features
- `make clean` - Remove build artifacts
