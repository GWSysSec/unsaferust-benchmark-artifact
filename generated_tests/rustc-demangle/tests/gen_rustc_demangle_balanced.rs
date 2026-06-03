









use std::alloc::{alloc_zeroed, dealloc, Layout};
use std::hint::black_box;

const SEED: u8 = 0xe4;
const PASSES: u64 = 115;
const SAFE_ITERS: u64 = 0;
const SAFE_ALLOCS: u64 = 589;

const BUF: usize = 4096;
const NBUF: usize = 16;




fn unsafe_volume(passes: u64, acc: &mut u64) {
    let layout = Layout::from_size_align(BUF, 16).unwrap();
    let per = (passes / NBUF as u64).max(1);
    for b in 0..NBUF {

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



fn safe_inst_dilute(iters: u64, acc: &mut u64) {
    let mut s = *acc;
    for i in 0..iters {
        s = s.wrapping_mul(6364136223846793005).wrapping_add((i ^ SEED as u64).wrapping_add(1));
    }
    *acc ^= s;
}



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
