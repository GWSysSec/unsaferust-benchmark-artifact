
use loom::sync::atomic::AtomicBool;
use loom::sync::atomic::Ordering::{Acquire, Relaxed, Release, SeqCst, AcqRel};
use loom::sync::Arc;
use loom::thread;

#[test]
fn atomic_bool_swap_basic() {
    loom::model(|| {
        let atom = AtomicBool::new(true);


        let prev = atom.swap(false, SeqCst);
        assert_eq!(prev, true);
        assert_eq!(atom.load(SeqCst), false);


        let prev2 = atom.swap(true, SeqCst);
        assert_eq!(prev2, false);
        assert_eq!(atom.load(SeqCst), true);


        let prev3 = atom.swap(true, SeqCst);
        assert_eq!(prev3, true);
        assert_eq!(atom.load(SeqCst), true);


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



        let saw_false_count = (if !our_swap { 1 } else { 0 }) + (if !their_swap { 1 } else { 0 });
        assert_eq!(saw_false_count, 1);


        assert_eq!(flag.load(Acquire), true);


        assert_ne!(our_swap, their_swap);


        assert!(our_swap == false || their_swap == false);

        assert!(our_swap == true || their_swap == true);
    });
}

#[test]
fn atomic_bool_compare_exchange_weak_success_and_failure() {
    loom::model(|| {
        let atom = AtomicBool::new(false);


        let result = atom.compare_exchange_weak(false, true, SeqCst, SeqCst);
        assert_eq!(result, Ok(false));
        assert_eq!(atom.load(SeqCst), true);


        let result2 = atom.compare_exchange_weak(false, true, SeqCst, SeqCst);
        assert_eq!(result2, Err(true));
        assert_eq!(atom.load(SeqCst), true);


        let result3 = atom.compare_exchange_weak(true, false, SeqCst, SeqCst);
        assert_eq!(result3, Ok(true));
        assert_eq!(atom.load(SeqCst), false);


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


        let success_count = (if our_result == Ok(false) { 1 } else { 0 })
            + (if their_result == Ok(false) { 1 } else { 0 });


        assert!(success_count <= 1);


        if success_count == 1 {
            assert_eq!(atom.load(Acquire), true);
        }


        assert!(our_result.is_ok() || our_result.is_err());
        assert!(their_result.is_ok() || their_result.is_err());
    });
}

#[test]
fn atomic_bool_fetch_and_basic() {
    loom::model(|| {
        let atom = AtomicBool::new(true);


        let prev = atom.fetch_and(true, SeqCst);
        assert_eq!(prev, true);
        assert_eq!(atom.load(SeqCst), true);


        let prev2 = atom.fetch_and(false, SeqCst);
        assert_eq!(prev2, true);
        assert_eq!(atom.load(SeqCst), false);


        let prev3 = atom.fetch_and(true, SeqCst);
        assert_eq!(prev3, false);
        assert_eq!(atom.load(SeqCst), false);


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


        assert_eq!(atom.load(SeqCst), false);


        assert!(our_prev == true || their_prev == true);


        if our_prev == false {
            assert_eq!(their_prev, true);
        }
        if their_prev == false {
            assert_eq!(our_prev, true);
        }


        assert!(our_prev == true || our_prev == false);
        assert!(their_prev == true || their_prev == false);
    });
}

#[test]
fn atomic_bool_fetch_nand_truth_table() {
    loom::model(|| {


        let atom = AtomicBool::new(true);
        let prev = atom.fetch_nand(true, SeqCst);
        assert_eq!(prev, true);
        assert_eq!(atom.load(SeqCst), false);


        let prev2 = atom.fetch_nand(true, SeqCst);
        assert_eq!(prev2, false);
        assert_eq!(atom.load(SeqCst), true);


        let prev3 = atom.fetch_nand(false, SeqCst);
        assert_eq!(prev3, true);
        assert_eq!(atom.load(SeqCst), true);


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


        let handle = thread::spawn(move || {
            atom2.fetch_nand(true, SeqCst)
        });

        let our_prev = atom.fetch_nand(true, SeqCst);
        let their_prev = handle.join().unwrap();

        let final_val = atom.load(SeqCst);





        assert_eq!(final_val, true);


        assert_ne!(our_prev, their_prev);
        assert!(our_prev == true || their_prev == true);
        assert!(our_prev == false || their_prev == false);
    });
}

#[test]
fn atomic_bool_fetch_or_basic() {
    loom::model(|| {
        let atom = AtomicBool::new(false);


        let prev = atom.fetch_or(false, SeqCst);
        assert_eq!(prev, false);
        assert_eq!(atom.load(SeqCst), false);


        let prev2 = atom.fetch_or(true, SeqCst);
        assert_eq!(prev2, false);
        assert_eq!(atom.load(SeqCst), true);


        let prev3 = atom.fetch_or(false, SeqCst);
        assert_eq!(prev3, true);
        assert_eq!(atom.load(SeqCst), true);


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


        assert_eq!(atom.load(Acquire), true);


        let saw_false_count = (if !our_prev { 1u32 } else { 0 }) + (if !their_prev { 1 } else { 0 });
        assert_eq!(saw_false_count, 1);


        assert_ne!(our_prev, their_prev);


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


        let result = atom.fetch_update(SeqCst, SeqCst, |val| Some(!val));
        assert_eq!(result, Ok(false));
        assert_eq!(atom.load(SeqCst), true);


        let result2 = atom.fetch_update(SeqCst, SeqCst, |val| Some(!val));
        assert_eq!(result2, Ok(true));
        assert_eq!(atom.load(SeqCst), false);


        let result3 = atom.fetch_update(SeqCst, SeqCst, |val| {
            if !val { Some(true) } else { None }
        });
        assert_eq!(result3, Ok(false));
        assert_eq!(atom.load(SeqCst), true);


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


        let handle = thread::spawn(move || {
            atom2.fetch_update(SeqCst, SeqCst, |val| {
                if !val { Some(true) } else { None }
            })
        });

        let our_result = atom.fetch_update(SeqCst, SeqCst, |val| {
            if !val { Some(true) } else { None }
        });

        let their_result = handle.join().unwrap();


        assert_eq!(atom.load(SeqCst), true);


        let our_ok = our_result.is_ok();
        let their_ok = their_result.is_ok();
        assert_ne!(our_ok, their_ok);


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


        let result = atom.fetch_update(SeqCst, SeqCst, |_| None);
        assert_eq!(result, Err(true));
        assert_eq!(atom.load(SeqCst), true);

        atom.store(false, SeqCst);
        let result2 = atom.fetch_update(SeqCst, SeqCst, |_| None);
        assert_eq!(result2, Err(false));
        assert_eq!(atom.load(SeqCst), false);


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


        assert_eq!(their_prev, false);


        assert_eq!(atom.load(Acquire), true);


        assert!(val_after == false || val_after == true);


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

            let prev = atom2.fetch_and(false, SeqCst);
            prev
        });


        let our_prev = atom.fetch_or(true, SeqCst);
        let their_prev = handle.join().unwrap();





        assert!(our_prev == true || our_prev == false);
        assert!(their_prev == true || their_prev == false);



        assert!(our_prev == true || their_prev == true);

        let final_val = atom.load(SeqCst);



        assert!(final_val == true || final_val == false);
    });
}