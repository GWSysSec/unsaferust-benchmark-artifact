
use loom::sync::atomic::AtomicBool;
use loom::sync::atomic::Ordering::{Acquire, Relaxed, Release, SeqCst, AcqRel};
use loom::sync::Arc;
use loom::thread;

#[test]
fn atomic_bool_swap_basic() {
    loom::model(|| {
        let atom = AtomicBool::new(true);

        // swap from true to false
        let prev = atom.swap(false, SeqCst);
        assert_eq!(prev, true);
        assert_eq!(atom.load(SeqCst), false);

        // swap from false to true
        let prev2 = atom.swap(true, SeqCst);
        assert_eq!(prev2, false);
        assert_eq!(atom.load(SeqCst), true);

        // swap same value
        let prev3 = atom.swap(true, SeqCst);
        assert_eq!(prev3, true);
        assert_eq!(atom.load(SeqCst), true);

        // swap back to false again
        let prev4 = atom.swap(false, Relaxed);
        assert_eq!(prev4, true);
        assert_eq!(atom.load(Relaxed), false);
    });
}

#[test]
fn atomic_bool_swap_concurrent() {
    loom::model(|| {
        let flag = Arc::new(AtomicBool::new(false));
        let flag2 = flag.clone();

        let handle = thread::spawn(move || {
            flag2.swap(true, Release)
        });

        let our_swap = flag.swap(true, Release);
        let their_swap = handle.join().unwrap();

        // Exactly one of them should have seen false (the initial value)
        // and the other should have seen true (the value set by the first swap)
        let saw_false_count = (if !our_swap { 1 } else { 0 }) + (if !their_swap { 1 } else { 0 });
        assert_eq!(saw_false_count, 1);

        // After both swaps, the value must be true
        assert_eq!(flag.load(Acquire), true);

        // Verify the two results are different
        assert_ne!(our_swap, their_swap);

        // One must be false
        assert!(our_swap == false || their_swap == false);
        // One must be true
        assert!(our_swap == true || their_swap == true);
    });
}

#[test]
fn atomic_bool_compare_exchange_weak_success_and_failure() {
    loom::model(|| {
        let atom = AtomicBool::new(false);

        // Successful CAS: current matches
        let result = atom.compare_exchange_weak(false, true, SeqCst, SeqCst);
        assert_eq!(result, Ok(false));
        assert_eq!(atom.load(SeqCst), true);

        // Failed CAS: current does not match
        let result2 = atom.compare_exchange_weak(false, true, SeqCst, SeqCst);
        assert_eq!(result2, Err(true));
        assert_eq!(atom.load(SeqCst), true);

        // Successful CAS back to false
        let result3 = atom.compare_exchange_weak(true, false, SeqCst, SeqCst);
        assert_eq!(result3, Ok(true));
        assert_eq!(atom.load(SeqCst), false);

        // Another failure case
        let result4 = atom.compare_exchange_weak(true, false, SeqCst, SeqCst);
        assert_eq!(result4, Err(false));
    });
}

#[test]
fn atomic_bool_compare_exchange_weak_concurrent() {
    loom::model(|| {
        let atom = Arc::new(AtomicBool::new(false));
        let atom2 = atom.clone();

        let handle = thread::spawn(move || {
            atom2.compare_exchange_weak(false, true, AcqRel, Acquire)
        });

        let our_result = atom.compare_exchange_weak(false, true, AcqRel, Acquire);
        let their_result = handle.join().unwrap();

        // At most one can succeed with Ok(false)
        let success_count = (if our_result == Ok(false) { 1 } else { 0 })
            + (if their_result == Ok(false) { 1 } else { 0 });
        // compare_exchange_weak can spuriously fail, so 0 successes is possible
        // but at most 1 can succeed
        assert!(success_count <= 1);

        // If one succeeded, the final value is true
        if success_count == 1 {
            assert_eq!(atom.load(Acquire), true);
        }

        // Verify result types are correct
        assert!(our_result.is_ok() || our_result.is_err());
        assert!(their_result.is_ok() || their_result.is_err());
    });
}

#[test]
fn atomic_bool_fetch_and_basic() {
    loom::model(|| {
        let atom = AtomicBool::new(true);

        // true AND true = true, returns old value true
        let prev = atom.fetch_and(true, SeqCst);
        assert_eq!(prev, true);
        assert_eq!(atom.load(SeqCst), true);

        // true AND false = false, returns old value true
        let prev2 = atom.fetch_and(false, SeqCst);
        assert_eq!(prev2, true);
        assert_eq!(atom.load(SeqCst), false);

        // false AND true = false, returns old value false
        let prev3 = atom.fetch_and(true, SeqCst);
        assert_eq!(prev3, false);
        assert_eq!(atom.load(SeqCst), false);

        // false AND false = false, returns old value false
        let prev4 = atom.fetch_and(false, SeqCst);
        assert_eq!(prev4, false);
        assert_eq!(atom.load(SeqCst), false);
    });
}

#[test]
fn atomic_bool_fetch_and_concurrent() {
    loom::model(|| {
        let atom = Arc::new(AtomicBool::new(true));
        let atom2 = atom.clone();

        let handle = thread::spawn(move || {
            atom2.fetch_and(false, SeqCst)
        });

        let our_prev = atom.fetch_and(false, SeqCst);
        let their_prev = handle.join().unwrap();

        // Final value must be false (true AND false AND false = false)
        assert_eq!(atom.load(SeqCst), false);

        // At least one of them must have seen true (the initial value)
        assert!(our_prev == true || their_prev == true);

        // If one saw false, the other must have gone first
        if our_prev == false {
            assert_eq!(their_prev, true);
        }
        if their_prev == false {
            assert_eq!(our_prev, true);
        }

        // Both results are valid booleans (trivially true but confirms type)
        assert!(our_prev == true || our_prev == false);
        assert!(their_prev == true || their_prev == false);
    });
}

#[test]
fn atomic_bool_fetch_nand_truth_table() {
    loom::model(|| {
        // NAND truth table:
        // true NAND true = false (NOT(true AND true))
        let atom = AtomicBool::new(true);
        let prev = atom.fetch_nand(true, SeqCst);
        assert_eq!(prev, true);
        assert_eq!(atom.load(SeqCst), false);

        // false NAND true = true (NOT(false AND true))
        let prev2 = atom.fetch_nand(true, SeqCst);
        assert_eq!(prev2, false);
        assert_eq!(atom.load(SeqCst), true);

        // true NAND false = true (NOT(true AND false))
        let prev3 = atom.fetch_nand(false, SeqCst);
        assert_eq!(prev3, true);
        assert_eq!(atom.load(SeqCst), true);

        // true NAND true = false again
        let prev4 = atom.fetch_nand(true, SeqCst);
        assert_eq!(prev4, true);
        assert_eq!(atom.load(SeqCst), false);
    });
}

#[test]
fn atomic_bool_fetch_nand_concurrent_toggle() {
    loom::model(|| {
        let atom = Arc::new(AtomicBool::new(true));
        let atom2 = atom.clone();

        // Both threads do fetch_nand(true), which toggles the value
        let handle = thread::spawn(move || {
            atom2.fetch_nand(true, SeqCst)
        });

        let our_prev = atom.fetch_nand(true, SeqCst);
        let their_prev = handle.join().unwrap();

        let final_val = atom.load(SeqCst);

        // Starting from true:
        // If thread A goes first: true NAND true = false, then thread B: false NAND true = true
        // If thread B goes first: true NAND true = false, then thread A: false NAND true = true
        // In both cases, final value is true
        assert_eq!(final_val, true);

        // One thread saw true (went first), the other saw false (went second)
        assert_ne!(our_prev, their_prev);
        assert!(our_prev == true || their_prev == true);
        assert!(our_prev == false || their_prev == false);
    });
}

#[test]
fn atomic_bool_fetch_or_basic() {
    loom::model(|| {
        let atom = AtomicBool::new(false);

        // false OR false = false
        let prev = atom.fetch_or(false, SeqCst);
        assert_eq!(prev, false);
        assert_eq!(atom.load(SeqCst), false);

        // false OR true = true
        let prev2 = atom.fetch_or(true, SeqCst);
        assert_eq!(prev2, false);
        assert_eq!(atom.load(SeqCst), true);

        // true OR false = true
        let prev3 = atom.fetch_or(false, SeqCst);
        assert_eq!(prev3, true);
        assert_eq!(atom.load(SeqCst), true);

        // true OR true = true
        let prev4 = atom.fetch_or(true, SeqCst);
        assert_eq!(prev4, true);
        assert_eq!(atom.load(SeqCst), true);
    });
}

#[test]
fn atomic_bool_fetch_or_concurrent_set_once() {
    loom::model(|| {
        let atom = Arc::new(AtomicBool::new(false));
        let atom2 = atom.clone();

        let handle = thread::spawn(move || {
            atom2.fetch_or(true, Release)
        });

        let our_prev = atom.fetch_or(true, Release);
        let their_prev = handle.join().unwrap();

        // Final value must be true (false OR true OR true = true)
        assert_eq!(atom.load(Acquire), true);

        // Exactly one of them saw false (the initial value)
        let saw_false_count = (if !our_prev { 1u32 } else { 0 }) + (if !their_prev { 1 } else { 0 });
        assert_eq!(saw_false_count, 1);

        // The other saw true
        assert_ne!(our_prev, their_prev);

        // Verify ordering of observations
        if our_prev == false {
            assert_eq!(their_prev, true);
        } else {
            assert_eq!(their_prev, false);
        }
    });
}

#[test]
fn atomic_bool_fetch_update_success() {
    loom::model(|| {
        let atom = AtomicBool::new(false);

        // Successful update: toggle the value
        let result = atom.fetch_update(SeqCst, SeqCst, |val| Some(!val));
        assert_eq!(result, Ok(false));
        assert_eq!(atom.load(SeqCst), true);

        // Another successful update: toggle back
        let result2 = atom.fetch_update(SeqCst, SeqCst, |val| Some(!val));
        assert_eq!(result2, Ok(true));
        assert_eq!(atom.load(SeqCst), false);

        // Conditional update: only set to true if currently false
        let result3 = atom.fetch_update(SeqCst, SeqCst, |val| {
            if !val { Some(true) } else { None }
        });
        assert_eq!(result3, Ok(false));
        assert_eq!(atom.load(SeqCst), true);

        // Conditional update: should fail since value is now true
        let result4 = atom.fetch_update(SeqCst, SeqCst, |val| {
            if !val { Some(true) } else { None }
        });
        assert_eq!(result4, Err(true));
        assert_eq!(atom.load(SeqCst), true);
    });
}

#[test]
fn atomic_bool_fetch_update_concurrent() {
    loom::model(|| {
        let atom = Arc::new(AtomicBool::new(false));
        let atom2 = atom.clone();

        // Both threads try to set false -> true using fetch_update
        let handle = thread::spawn(move || {
            atom2.fetch_update(SeqCst, SeqCst, |val| {
                if !val { Some(true) } else { None }
            })
        });

        let our_result = atom.fetch_update(SeqCst, SeqCst, |val| {
            if !val { Some(true) } else { None }
        });

        let their_result = handle.join().unwrap();

        // Final value must be true
        assert_eq!(atom.load(SeqCst), true);

        // Exactly one should succeed
        let our_ok = our_result.is_ok();
        let their_ok = their_result.is_ok();
        assert_ne!(our_ok, their_ok);

        // The successful one saw false
        if our_ok {
            assert_eq!(our_result, Ok(false));
            assert_eq!(their_result, Err(true));
        } else {
            assert_eq!(their_result, Ok(false));
            assert_eq!(our_result, Err(true));
        }
    });
}

#[test]
fn atomic_bool_fetch_update_always_none() {
    loom::model(|| {
        let atom = AtomicBool::new(true);

        // fetch_update that always returns None should fail
        let result = atom.fetch_update(SeqCst, SeqCst, |_| None);
        assert_eq!(result, Err(true));
        assert_eq!(atom.load(SeqCst), true);

        atom.store(false, SeqCst);
        let result2 = atom.fetch_update(SeqCst, SeqCst, |_| None);
        assert_eq!(result2, Err(false));
        assert_eq!(atom.load(SeqCst), false);

        // fetch_update that always returns Some should succeed
        let result3 = atom.fetch_update(SeqCst, SeqCst, |_| Some(true));
        assert_eq!(result3, Ok(false));
        assert_eq!(atom.load(SeqCst), true);

        let result4 = atom.fetch_update(SeqCst, SeqCst, |val| Some(val));
        assert_eq!(result4, Ok(true));
        assert_eq!(atom.load(SeqCst), true);
    });
}

#[test]
fn atomic_bool_swap_with_ordering_variants() {
    loom::model(|| {
        let atom = Arc::new(AtomicBool::new(false));
        let atom2 = atom.clone();

        let handle = thread::spawn(move || {
            let prev = atom2.swap(true, Release);
            prev
        });

        thread::yield_now();
        let val_after = atom.load(Acquire);
        let their_prev = handle.join().unwrap();

        // their_prev must be false since they swapped from initial
        assert_eq!(their_prev, false);

        // After the thread completes, value must be true
        assert_eq!(atom.load(Acquire), true);

        // val_after is either false (read before swap) or true (read after swap)
        assert!(val_after == false || val_after == true);

        // Verify we can swap again
        let final_prev = atom.swap(false, SeqCst);
        assert_eq!(final_prev, true);
        assert_eq!(atom.load(SeqCst), false);
    });
}

#[test]
fn atomic_bool_combined_operations() {
    loom::model(|| {
        let atom = Arc::new(AtomicBool::new(true));
        let atom2 = atom.clone();

        let handle = thread::spawn(move || {
            // Thread does fetch_and(false) to clear the flag
            let prev = atom2.fetch_and(false, SeqCst);
            prev
        });

        // Main thread does fetch_or(true) to set the flag
        let our_prev = atom.fetch_or(true, SeqCst);
        let their_prev = handle.join().unwrap();

        // our_prev from fetch_or(true) on initial true: always true (OR with true is idempotent on true)
        // But the other thread might have cleared it first
        // If other thread goes first: true -> false (fetch_and returns true), then we do false OR true = true (returns false)
        // If we go first: true OR true = true (returns true), then other does true AND false = false (returns true)
        assert!(our_prev == true || our_prev == false);
        assert!(their_prev == true || their_prev == false);

        // their_prev from fetch_and: they saw whatever was current
        // At least one must have seen true (the initial value)
        assert!(our_prev == true || their_prev == true);

        let final_val = atom.load(SeqCst);
        // Final value depends on ordering:
        // Case 1: fetch_and first, fetch_or second -> true AND false = false, then false OR true = true -> final = true
        // Case 2: fetch_or first, fetch_and second -> true OR true = true, then true AND false = false -> final = false
        assert!(final_val == true || final_val == false);
    });
}