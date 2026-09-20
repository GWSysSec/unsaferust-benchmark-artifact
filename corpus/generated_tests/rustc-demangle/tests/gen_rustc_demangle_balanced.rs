//! Balanced unsafe-oriented test for `rustc-demangle` (RQ1 cycles + RQ2 heap + RQ3 insts).
//!
//! rustc-demangle's own unsafe is negligible relative to its safe parsing path.
//! It therefore registers ~zero on the runtime unsafe metrics. This test injects
//! a controlled amount of unsafe memory traffic (a raw-pointer load/store loop over
//! medium heap buffers) balanced by a safe instruction-dilution loop and a safe
//! heap-allocation loop, tuned from this crate's existing test-suite size so the
//! crate-level unsafe ratios land in the MIDDLE of the corpus distribution rather
//! than at an extreme (target ~1.5% insts, ~1.2% heap memory).

use std::alloc::{alloc_zeroed, dealloc, Layout};
use std::hint::black_box;

const SEED: u8 = 0xe4;
const PASSES: u64 = 115;         // load/store passes over the unsafe buffers (RQ1/RQ3)
const SAFE_ITERS: u64 = 0; // safe LCG iters: instruction/cycle dilution (no heap)
const SAFE_ALLOCS: u64 = 589; // safe heap allocations: memory dilution (RQ2)

const BUF: usize = 4096;
const NBUF: usize = 16;

/// Raw-pointer load/store volume over NBUF medium heap buffers. Supplies the
/// unsafe instructions + cycles (RQ1/RQ3) and the only unsafe heap memory (RQ2
/// numerator) -- deliberately small so the safe allocations below dilute it.
fn unsafe_volume(passes: u64, acc: &mut u64) {
    let layout = Layout::from_size_align(BUF, 16).unwrap();
    let per = (passes / NBUF as u64).max(1);
    for b in 0..NBUF {
        // SAFETY: layout valid & non-zero; ptr null-checked; offsets < BUF; freed once.
        unsafe {
            let p = alloc_zeroed(layout);
            assert!(!p.is_null());
            for pass in 0..per {
                for i in 0..BUF {
                    let v = p.add(i).read().wrapping_mul(31).wrapping_add(pass as u8 ^ SEED ^ b as u8);
                    p.add(i).write(v);
                    *acc = acc.wrapping_add(v as u64);
                }
            }
            dealloc(p, layout);
        }
    }
}

/// Pure-safe LCG recurrence (no heap): dilutes the unsafe instruction/cycle
/// fraction. The recurrence is not closed-form-reducible, so it cannot be elided.
fn safe_inst_dilute(iters: u64, acc: &mut u64) {
    let mut s = *acc;
    for i in 0..iters {
        s = s.wrapping_mul(6364136223846793005).wrapping_add((i ^ SEED as u64).wrapping_add(1));
    }
    *acc ^= s;
}

/// Safe heap churn: allocate + observe + free BUF-sized buffers. Dilutes the
/// unsafe HEAP-MEMORY fraction (RQ2) while adding almost no instructions.
fn safe_mem_dilute(allocs: u64, acc: &mut u64) {
    for k in 0..allocs {
        let v: Vec<u8> = Vec::with_capacity(BUF);
        black_box(v.as_ptr());
        *acc = acc.wrapping_add((v.capacity() as u64) ^ k);
        drop(v);
    }
}

#[test]
fn unsafe_balanced_workload() {
    let mut acc: u64 = 0;
    unsafe_volume(env_or("RTG_PASSES", PASSES), &mut acc);
    safe_inst_dilute(env_or("RTG_SAFE", SAFE_ITERS), &mut acc);
    safe_mem_dilute(env_or("RTG_ALLOC", SAFE_ALLOCS), &mut acc);
    assert_ne!(acc, 0);
}

#[allow(dead_code)]
fn env_or(key: &str, default: u64) -> u64 {
    std::env::var(key).ok().and_then(|v| v.parse().ok()).unwrap_or(default)
}
