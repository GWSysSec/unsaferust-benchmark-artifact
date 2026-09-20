
use loom::sync::atomic::AtomicPtr;
use loom::sync::Arc;
use loom::thread;
use std::sync::atomic::Ordering::{AcqRel, Acquire, Relaxed, Release, SeqCst};

#[test]
fn atomic_ptr_with_mut_basic() {
    loom::model(|| {
        let mut val_a: u64 = 42;
        let mut val_b: u64 = 99;
        let ptr_a: *mut u64 = &mut val_a;
        let ptr_b: *mut u64 = &mut val_b;

        let mut atomic = AtomicPtr::new(ptr_a);

        // Verify initial value via with_mut
        let initial = atomic.with_mut(|p| *p);
        assert_eq!(initial, ptr_a);
        assert_ne!(initial, ptr_b);

        // Mutate via with_mut
        atomic.with_mut(|p| {
            *p = ptr_b;
        });

        // Verify mutation took effect
        let after_mut = atomic.with_mut(|p| *p);
        assert_eq!(after_mut, ptr_b);
        assert_ne!(after_mut, ptr_a);

        // Load should also reflect the change
        let loaded = atomic.load(Relaxed);
        assert_eq!(loaded, ptr_b);

        // with_mut returning a computed value
        let is_b = atomic.with_mut(|p| *p == ptr_b);
        assert!(is_b);

        // Set back to ptr_a
        atomic.with_mut(|p| {
            *p = ptr_a;
        });
        let final_val = atomic.load(SeqCst);
        assert_eq!(final_val, ptr_a);
    });
}

#[test]
fn atomic_ptr_swap_single_thread() {
    loom::model(|| {
        let mut val_a: u32 = 10;
        let mut val_b: u32 = 20;
        let mut val_c: u32 = 30;
        let ptr_a: *mut u32 = &mut val_a;
        let ptr_b: *mut u32 = &mut val_b;
        let ptr_c: *mut u32 = &mut val_c;

        let atomic = AtomicPtr::new(ptr_a);

        // First swap: a -> b, returns a
        let old = atomic.swap(ptr_b, SeqCst);
        assert_eq!(old, ptr_a);
        assert_eq!(atomic.load(SeqCst), ptr_b);

        // Second swap: b -> c, returns b
        let old2 = atomic.swap(ptr_c, SeqCst);
        assert_eq!(old2, ptr_b);
        assert_eq!(atomic.load(SeqCst), ptr_c);

        // Third swap: c -> a, returns c
        let old3 = atomic.swap(ptr_a, Release);
        assert_eq!(old3, ptr_c);
        assert_eq!(atomic.load(Acquire), ptr_a);

        // Swap with same value
        let old4 = atomic.swap(ptr_a, Relaxed);
        assert_eq!(old4, ptr_a);
        assert_eq!(atomic.load(Relaxed), ptr_a);
    });
}

#[test]
fn atomic_ptr_swap_concurrent() {
    loom::model(|| {
        let mut val_a: usize = 100;
        let mut val_b: usize = 200;
        let ptr_a: *mut usize = &mut val_a;
        let ptr_b: *mut usize = &mut val_b;

        let atomic = Arc::new(AtomicPtr::new(ptr_a));
        let atomic2 = atomic.clone();

        let handle = thread::spawn(move || {
            let old = atomic2.swap(ptr_b, AcqRel);
            old
        });

        let old_main = atomic.swap(ptr_a, AcqRel);
        let old_thread = handle.join().unwrap();

        // One of them must have seen ptr_a as the old value initially
        // Both swapped, so at least one got ptr_a back
        let saw_a = (old_main == ptr_a) || (old_thread == ptr_a);
        assert!(saw_a);

        // The final value must be either ptr_a or ptr_b
        let final_val = atomic.load(SeqCst);
        let valid_final = final_val == ptr_a || final_val == ptr_b;
        assert!(valid_final);
    });
}

#[test]
fn atomic_ptr_compare_exchange_weak_success_and_failure() {
    loom::model(|| {
        let mut val_a: i32 = 1;
        let mut val_b: i32 = 2;
        let mut val_c: i32 = 3;
        let ptr_a: *mut i32 = &mut val_a;
        let ptr_b: *mut i32 = &mut val_b;
        let ptr_c: *mut i32 = &mut val_c;

        let atomic = AtomicPtr::new(ptr_a);

        // Failure case: current doesn't match
        let result = atomic.compare_exchange_weak(ptr_b, ptr_c, SeqCst, SeqCst);
        assert_eq!(result, Err(ptr_a));
        assert_eq!(atomic.load(SeqCst), ptr_a);

        // Success case: current matches
        let result2 = atomic.compare_exchange_weak(ptr_a, ptr_b, SeqCst, SeqCst);
        // compare_exchange_weak may spuriously fail, but in loom it behaves like strong
        assert_eq!(result2, Ok(ptr_a));
        assert_eq!(atomic.load(SeqCst), ptr_b);

        // Another success
        let result3 = atomic.compare_exchange_weak(ptr_b, ptr_c, AcqRel, Acquire);
        assert_eq!(result3, Ok(ptr_b));
        assert_eq!(atomic.load(Acquire), ptr_c);

        // Failure again
        let result4 = atomic.compare_exchange_weak(ptr_a, ptr_b, SeqCst, SeqCst);
        assert_eq!(result4, Err(ptr_c));
    });
}

#[test]
fn atomic_ptr_compare_exchange_weak_concurrent() {
    loom::model(|| {
        let mut val_a: u64 = 0;
        let mut val_b: u64 = 1;
        let mut val_c: u64 = 2;
        let ptr_a: *mut u64 = &mut val_a;
        let ptr_b: *mut u64 = &mut val_b;
        let ptr_c: *mut u64 = &mut val_c;

        let atomic = Arc::new(AtomicPtr::new(ptr_a));
        let atomic2 = atomic.clone();

        let handle = thread::spawn(move || {
            // Try to CAS from a -> b
            atomic2.compare_exchange_weak(ptr_a, ptr_b, SeqCst, SeqCst)
        });

        // Main thread tries CAS from a -> c
        let main_result = atomic.compare_exchange_weak(ptr_a, ptr_c, SeqCst, SeqCst);
        let thread_result = handle.join().unwrap();

        // At most one can succeed (both tried to swap from ptr_a)
        let main_ok = main_result.is_ok();
        let thread_ok = thread_result.is_ok();

        // They can't both succeed since both expect ptr_a
        if main_ok && thread_ok {
            panic!("Both CAS operations should not succeed simultaneously");
        }

        // At least one must succeed (the first one to run sees ptr_a)
        assert!(main_ok || thread_ok);

        let final_val = atomic.load(SeqCst);
        // Final value is either ptr_b or ptr_c depending on who won
        assert!(final_val == ptr_b || final_val == ptr_c);
    });
}

#[test]
fn atomic_ptr_fetch_update_success() {
    loom::model(|| {
        let mut val_a: u32 = 10;
        let mut val_b: u32 = 20;
        let ptr_a: *mut u32 = &mut val_a;
        let ptr_b: *mut u32 = &mut val_b;

        let atomic = AtomicPtr::new(ptr_a);

        // fetch_update that always succeeds (returns Some)
        let result = atomic.fetch_update(SeqCst, SeqCst, |current| {
            assert_eq!(current, ptr_a);
            Some(ptr_b)
        });
        assert_eq!(result, Ok(ptr_a));
        assert_eq!(atomic.load(SeqCst), ptr_b);

        // fetch_update that returns None (failure)
        let result2 = atomic.fetch_update(SeqCst, SeqCst, |current| {
            assert_eq!(current, ptr_b);
            None
        });
        assert_eq!(result2, Err(ptr_b));
        assert_eq!(atomic.load(SeqCst), ptr_b);

        // fetch_update back to ptr_a
        let result3 = atomic.fetch_update(AcqRel, Acquire, |_| Some(ptr_a));
        assert_eq!(result3, Ok(ptr_b));
        assert_eq!(atomic.load(Acquire), ptr_a);
    });
}

#[test]
fn atomic_ptr_fetch_update_conditional() {
    loom::model(|| {
        let mut val_a: u8 = 1;
        let mut val_b: u8 = 2;
        let mut val_c: u8 = 3;
        let ptr_a: *mut u8 = &mut val_a;
        let ptr_b: *mut u8 = &mut val_b;
        let ptr_c: *mut u8 = &mut val_c;

        let atomic = AtomicPtr::new(ptr_a);

        // Conditional update: only update if current == ptr_b (should fail)
        let result = atomic.fetch_update(SeqCst, SeqCst, |current| {
            if current == ptr_b {
                Some(ptr_c)
            } else {
                None
            }
        });
        assert_eq!(result, Err(ptr_a));
        assert_eq!(atomic.load(SeqCst), ptr_a);

        // Now set to ptr_b and try again
        atomic.swap(ptr_b, SeqCst);
        assert_eq!(atomic.load(SeqCst), ptr_b);

        let result2 = atomic.fetch_update(SeqCst, SeqCst, |current| {
            if current == ptr_b {
                Some(ptr_c)
            } else {
                None
            }
        });
        assert_eq!(result2, Ok(ptr_b));
        assert_eq!(atomic.load(SeqCst), ptr_c);

        // Identity update (set to same value)
        let result3 = atomic.fetch_update(SeqCst, SeqCst, |current| Some(current));
        assert_eq!(result3, Ok(ptr_c));
        assert_eq!(atomic.load(SeqCst), ptr_c);
    });
}

#[test]
fn atomic_ptr_fetch_update_concurrent() {
    loom::model(|| {
        let mut vals: [usize; 3] = [0, 1, 2];
        let ptr_0: *mut usize = &mut vals[0];
        let ptr_1: *mut usize = &mut vals[1];
        let ptr_2: *mut usize = &mut vals[2];

        let atomic = Arc::new(AtomicPtr::new(ptr_0));
        let atomic2 = atomic.clone();

        let handle = thread::spawn(move || {
            // Thread tries to update: if current is ptr_0, set to ptr_1
            atomic2.fetch_update(SeqCst, SeqCst, |current| {
                if current == ptr_0 {
                    Some(ptr_1)
                } else {
                    None
                }
            })
        });

        // Main tries to update: if current is ptr_0, set to ptr_2
        let main_result = atomic.fetch_update(SeqCst, SeqCst, |current| {
            if current == ptr_0 {
                Some(ptr_2)
            } else {
                None
            }
        });

        let thread_result = handle.join().unwrap();

        // Exactly one should succeed
        let main_ok = main_result.is_ok();
        let thread_ok = thread_result.is_ok();
        assert!(main_ok || thread_ok);

        let final_val = atomic.load(SeqCst);
        assert!(final_val == ptr_1 || final_val == ptr_2);

        // If main succeeded, final should be ptr_2 (unless thread ran after)
        // If thread succeeded, final should be ptr_1 (unless main ran after)
        // But since both only update from ptr_0, only one can succeed
        if main_ok && !thread_ok {
            assert_eq!(final_val, ptr_2);
        }
        if thread_ok && !main_ok {
            assert_eq!(final_val, ptr_1);
        }
    });
}

#[test]
fn atomic_ptr_with_mut_chained_operations() {
    loom::model(|| {
        let mut values: [u64; 4] = [10, 20, 30, 40];
        let ptrs: Vec<*mut u64> = values.iter_mut().map(|v| v as *mut u64).collect();

        let mut atomic = AtomicPtr::new(ptrs[0]);

        // Chain multiple with_mut calls
        let p0 = atomic.with_mut(|p| {
            let old = *p;
            *p = ptrs[1];
            old
        });
        assert_eq!(p0, ptrs[0]);

        let p1 = atomic.with_mut(|p| {
            let old = *p;
            *p = ptrs[2];
            old
        });
        assert_eq!(p1, ptrs[1]);

        let p2 = atomic.with_mut(|p| {
            let old = *p;
            *p = ptrs[3];
            old
        });
        assert_eq!(p2, ptrs[2]);

        let final_val = atomic.load(Relaxed);
        assert_eq!(final_val, ptrs[3]);

        // with_mut can also just read
        let read_val = atomic.with_mut(|p| *p);
        assert_eq!(read_val, ptrs[3]);
    });
}

#[test]
fn atomic_ptr_swap_null_pointers() {
    loom::model(|| {
        let mut val: i32 = 42;
        let ptr: *mut i32 = &mut val;
        let null_ptr: *mut i32 = std::ptr::null_mut();

        let atomic = AtomicPtr::new(null_ptr);

        // Initial is null
        assert_eq!(atomic.load(SeqCst), null_ptr);
        assert!(atomic.load(SeqCst).is_null());

        // Swap null -> valid
        let old = atomic.swap(ptr, SeqCst);
        assert!(old.is_null());
        assert_eq!(atomic.load(SeqCst), ptr);
        assert!(!atomic.load(SeqCst).is_null());

        // Swap valid -> null
        let old2 = atomic.swap(null_ptr, SeqCst);
        assert_eq!(old2, ptr);
        assert!(atomic.load(SeqCst).is_null());

        // compare_exchange_weak with null
        let result = atomic.compare_exchange_weak(null_ptr, ptr, SeqCst, SeqCst);
        assert_eq!(result, Ok(null_ptr));
        assert_eq!(atomic.load(SeqCst), ptr);
    });
}