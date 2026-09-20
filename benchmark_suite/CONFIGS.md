# Benchmark Configurations

This document details the configuration for the 18 selected benchmark crates used in our artifact.

| Crate              | Bench Run Config                                                                                                                       |
| :----------------- | :------------------------------------------------------------------------------------------------------------------------------------- |
| **matrixmultiply** | Default                                                                                                                                |
| **arrayvec**       | Default                                                                                                                                |
| **ndarray**        | Default                                                                                                                                |
| **hashbrown**      | Default                                                                                                                                |
| **async-task**     | Default                                                                                                                                |
| **getrandom**      | Default                                                                                                                                |
| **httparse**       | Default                                                                                                                                |
| **smallvec**       | Default                                                                                                                                |
| **memchr**         | Using _rebar_, a rust tool that runs cargo under the hood                                                                              |
| **jpeg-decoder**   | Default                                                                                                                                |
| **semver**         | Default                                                                                                                                |
| **rayon**          | Build `rayon-demo` with `cargo build --release`, only call `nbody bench` with `--bodies 500` as parameters                             |
| **jni**            | Using `cargo bench` but with `--features invocation` passed as arguments                                                               |
| **parking_lot**    | Build `benchmarks` folder using `cargo build --release`. Parameters for executables: `./mutex 2 4 10 2 4` and `./rwlock 4 4 4 10 2 4`. |
| **simd-json**      | Default                                                                                                                                |
| **ring**           | Build and run all benchmarks by passing `--benches` to default `cargo bench` command                                                   |
| **tokio**          | Default                                                                                                                                |
| **petgraph**       | Default                                                                                                                                |

> **Default** means standard `cargo bench` was used and the benchmark suite was provided by the developers.

## Dynamic Characteristics

The following table summarizes the dynamic unsafe code characteristics observed during benchmarking.

| Benchmark          | CPU Cycles | Heap Usage | Unsafe Loads | Unsafe Stores | Unsafe Calls | Unsafe Inst | Fn     | Time  | Unsafe SLOC | Unsafe Coverage |
| :----------------- | :--------- | :--------- | :----------- | :------------ | :----------- | :---------- | :----- | :---- | :---------- | :-------------- |
| **matrixmultiply** | 36.86%     | 97.73%     | 7.23%        | 6.65%         | 53.80%       | 41.57%      | 34.07% | 74s   | 2.1K        | 80.56%          |
| **arrayvec**       | 33.72%     | 0.00%      | 0.00%        | 5.71%         | 0.00%        | 22.67%      | 72.22% | 41s   | 223         | 100%            |
| **ndarray**        | 32.92%     | 0.23%      | 4.61%        | 4.80%         | 2.63%        | 23.36%      | 91.62% | 1931s | 1.0k        | 83.33%          |
| **hashbrown**      | 32.66%     | 93.11%     | 10.06%       | 6.80%         | 3.00%        | 24.70%      | 88.41% | 212s  | 811         | 80.00%          |
| **async-task**     | 31.76%     | 99.90%     | 20.00%       | 8.07%         | 9.54%        | 42.88%      | 61.90% | 18s   | 428         | 50.91%          |
| **getrandom**      | 86.12%     | 0.00%      | 13.86%       | 0.00%         | 66.15%       | 6.02%       | 42.65% | 18s   | 417         | 41.67%          |
| **httparse**       | 1.27%      | 0.00%      | 18.63%       | 14.37%        | 1.48%        | 1.08%       | 40.86% | 107s  | 623         | 46.54%          |
| **smallvec**       | 9.08%      | 1.35%      | 0.66%        | 11.46%        | 11.77%       | 13.07%      | 58.12% | 130s  | 437         | 93.33%          |
| **memchr**         | 25.55%     | 0.50%      | 11.20%       | 0.00%         | 2.89%        | 5.38%       | 16.66% | 502s  | 1.3K        | 29.63%          |
| **jpeg-decoder**   | 5.79%      | 6.79%      | 5.24%        | 2.88%         | 36.06%       | 4.02%       | 16.32% | 332s  | 537         | 52.76%          |
| **semver**         | 36.37%     | 80.07%     | 4.85%        | 1.62%         | 13.59%       | 3.35%       | 31.37% | 14s   | 142         | 65.00%          |
| **rayon**          | 24.24%     | 58.90%     | 9.42%        | 14.67%        | 21.20%       | 6.97%       | 99.04% | 407s  | 1.6k        | 29.10%          |
| **jni**            | 28.89%     | 9.31%      | 16.17%       | 3.21%         | 20.10%       | 11.55%      | 42.02% | 85s   | 461         | 45.57%          |
| **parking_lot**    | 1.58%      | 0.00%      | 0.00%        | 32.06%        | 0.00%        | 2.10%       | 2.14%  | 154s  | 2.8K        | 100%            |
| **simd-json**      | 9.41%      | 5.33%      | 10.90%       | 0.00%         | 13.51%       | 8.12%       | 12.82% | 613s  | 1.3k        | 44.62%          |
| **ring**           | 5.57%      | 0.46%      | 11.12%       | 4.65%         | 8.79%        | 4.03%       | 10.42% | 106s  | 566         | 65.60%          |
| **tokio**          | 0.90%      | 87.00%     | 19.19%       | 17.23%        | 9.87%        | 5.95%       | 28.93% | 1236s | 1.9K        | 28.09%          |
| **petgraph**       | 17.10%     | 30.31%     | 13.52%       | 8.56%         | 6.00%        | 15.76%      | 77.25% | 320s  | 45          | 100%            |

**Summary Statistics:**

- **Min Coverage**: 28.09%
- **Max Coverage**: 100%
- **Geomean Coverage**: 57.98%
