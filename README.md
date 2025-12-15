# Understanding Unsafe Rust Dynamically: Behaviors and Benchmarks

We provide the artifacts for the paper in two formats, one as a docker image that has our prebuilt compiler and benchrmarks and instrumentation. Secondly as a repository that you can run and compile yourself.

# Setup

This repository uses **Git Submodules**. To set it up:

```bash
git clone --recursive git@github.com:GWSysSec/unsaferust-benchmark-artifact.git
cd unsaferust-benchmark-artifact
```

Or if already cloned:
```bash
./update-repos.sh
```

# Directory Structure
- `unsafe-rust-benchmark`: Submodule containing the compiler and benchmarks.
- `external/perf-lib`: Submodule containing the instrumentation library.
- `perf`: Symlink to `external/perf-lib/lib/perf` (for Docker compatibility).


# Docker getting started

To run the docker image please load our image into docker using

```bash
 docker load -i unsaferustbenchv2.tar
```

You can verify the image has been loaded using

```bash
 docker images
```

Once verified you can run our image using

```bash
 docker run -it unsaferustbench:v2.0
```

You should now be successfully inside our container and can first navigate to the `perf` folder

In this folder is our instrumentation and documentation for it.

To build our different instrumentation tools you can run the following commands

For unsafe coverage data for a benchmark you can run

```bash
make coverage
```

For unsafe inst. and fn. data for a benchmark you can run

```bash
make counter
```

For unsafe heap usage for a benchmark you can run

```bash
make heap
```

For unsafe CPU cycle data for a benchmark you can run

```bash
make cpu
```

Note: Instrumentations run one at a time.

After building your choice of instrumentation please run the corresponding `instrumentation` file inside the `perf/env` folder.

For example,to build and connect a tool

```bash
cd perf
make coverage
cd env
source coverage.sh
```

You can now navigate to the benchmarks folder to run our tools and gather data.

Some instrumentations give you data in the terminal itself, but all of them make coresponding files in the temp folder

An example is as follows

```bash
# Having built our coverage tool using make coverage and changed the env using source coverage.sh
cd ../../benchmarks/arrayvec-0.7.6
cargo bench
nano /tmp/unsafe_coverage.stat
```

# Building our compiler from Scratch (Non-Docker flow)

To build our compiler from scratch and run our tools and experiments, please first pull our repo and build our rust compiler using `x.py`

```bash
cd unsafe-rust-benchmark
./x.py build && ./x.py install
```

After successfully building our compiler you should see a `build` folder inside the `unsafe-rust-benchmark` folder.

You will now need to either add this to your path if you do not currently have rust on your system or use a toolchain like `rustup` to add our compiler so that it can be called.

For `rustup`, an example. While inside `unsafe-rust-benchmark` after build the compiler

```bash
rustup toolchain link stage1 build/host/stage1
rustup toolchain link stage2 build/host/stage2
```

You should now verify that your `rustc`version is correct

```bash
# should print out rustc 1.80.0-dev
rustc --version
```

Now the instructions to build and change flags for our runtime tools inside the `perf` folder are the same as listed above in docker getting started guide. Build your choice of instrumentation singularly and then change the env to match the instrumentation. All outputs will be inside `/tmp/*.stat` where each tool will have its own named file, e.g for `make coverage` the `cargo bench` will produce `unsafe_coverage.stat` as a file.

```bash
cd ../perf
make coverage
cd env
source coverage

cd ../../benchmarks/arrayvec-0.7.6
cargo bench
nano /tmp/unsafe_coverage.stat
```

# FAQ + ERROR Handling

### Compiler build failures

If at anypoint the compiler fails in its build process please retry using `./x.py build` or `./x.py build --stage 1` folowed by `./x.py build --stage 2`.

### Cargo failures/ Instruments not showing data

You can also build cargo this way by passing `cargo` as an argument but if you already have a cargo you can add our `perf/env` flags to your `.cargo/config.toml` file as followed

```bash
[build]
# Please note the stage 1 compiler also works
rustc = "/path/to/your/stage2/bin/rustc"
rustflags = [
    "--emit=llvm-ir",
    "-C", "unsafe_include_native_lib=false",
    "-C", "llvm-args=-enable-instmarker",
    # "-C", "llvm-args=-enable-heap-tracker",
    # "-C", "llvm-args=-enable-unsafe-function-tracker",
    # "-C", "llvm-args=-enable-unsafe-inst-counter",
    "-C", "llvm-args=-enable-dynamic-line-count",
    # "-C", "llvm-args=-enable-cpu-cycle-count",
    "-Z", "unstable-options",
    "--extern", "force:unsafe_perf=/path/to/your/perf/target/release/libunsafe_perf.rlib",
    "-L", "/path/to/your/perf/target/release/deps"
]
```

Remember to only use one instrumentation at a time

### New crate is not showing instrumentation data

We require certain flags in the `Cargo.toml` of the crate to be active, depending on the benchmark suite these should be added to the `Cargo.toml` of the crate being tested. Our selection already have these set for you.

```bash
...
[profile.bench]
debug = true # Or 2

[profile.release]
debug = true # Or 2
```
