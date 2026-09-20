#![cfg(feature = "futures")]

use loom::cell::UnsafeCell;
use loom::sync::atomic::AtomicUsize;
use std::sync::atomic::{AtomicUsize as StdAtomicUsize, Ordering};
use std::sync::Arc;
use std::task::{RawWaker, RawWakerVTable, Waker};

fn counting_waker(counter: Arc<StdAtomicUsize>) -> Waker {
    unsafe { Waker::from_raw(raw_counting_waker(counter)) }
}

fn raw_counting_waker(counter: Arc<StdAtomicUsize>) -> RawWaker {
    RawWaker::new(Arc::into_raw(counter) as *const (), &COUNTING_WAKER_VTABLE)
}

static COUNTING_WAKER_VTABLE: RawWakerVTable =
    RawWakerVTable::new(clone_counting_waker, wake_counting_waker, wake_by_ref_counting_waker, drop_counting_waker);

unsafe fn clone_counting_waker(ptr: *const ()) -> RawWaker {
    let counter = Arc::<StdAtomicUsize>::from_raw(ptr as *const StdAtomicUsize);
    let cloned = Arc::clone(&counter);
    let _ = Arc::into_raw(counter);
    raw_counting_waker(cloned)
}

unsafe fn wake_counting_waker(ptr: *const ()) {
    let counter = Arc::<StdAtomicUsize>::from_raw(ptr as *const StdAtomicUsize);
    counter.fetch_add(1, Ordering::SeqCst);
}

unsafe fn wake_by_ref_counting_waker(ptr: *const ()) {
    let counter = Arc::<StdAtomicUsize>::from_raw(ptr as *const StdAtomicUsize);
    counter.fetch_add(1, Ordering::SeqCst);
    let _ = Arc::into_raw(counter);
}

unsafe fn drop_counting_waker(ptr: *const ()) {
    drop(Arc::<StdAtomicUsize>::from_raw(ptr as *const StdAtomicUsize));
}

#[test]
fn atomic_waker_register_take_and_wake_workflow() {
    loom::model(|| {
        let state = AtomicUsize::new(0);
        let waker_slot = UnsafeCell::new(None::<Waker>);
        let wake_count = Arc::new(StdAtomicUsize::new(0));

        let borrowed_waker = counting_waker(Arc::clone(&wake_count));
        loom::future::atomic_waker::register_by_ref(&state, &waker_slot, &borrowed_waker);

        let taken = loom::future::atomic_waker::take_waker(&state, &waker_slot);
        assert!(taken.is_some());
        assert_eq!(wake_count.load(Ordering::SeqCst), 0);

        let taken = taken.unwrap();
        taken.wake_by_ref();
        assert_eq!(wake_count.load(Ordering::SeqCst), 1);

        assert!(loom::future::atomic_waker::take_waker(&state, &waker_slot).is_none());

        loom::future::atomic_waker::wake(&state, &waker_slot);
        assert_eq!(wake_count.load(Ordering::SeqCst), 1);

        let owned_waker = counting_waker(Arc::clone(&wake_count));
        loom::future::atomic_waker::register(&state, &waker_slot, owned_waker);

        loom::future::atomic_waker::wake(&state, &waker_slot);
        assert_eq!(wake_count.load(Ordering::SeqCst), 2);

        assert!(loom::future::atomic_waker::take_waker(&state, &waker_slot).is_none());
    });
}

#[test]
fn atomic_waker_replaces_registered_waker_and_wakes_latest() {
    loom::model(|| {
        let state = AtomicUsize::new(0);
        let waker_slot = UnsafeCell::new(None::<Waker>);

        let first_count = Arc::new(StdAtomicUsize::new(0));
        let second_count = Arc::new(StdAtomicUsize::new(0));

        loom::future::atomic_waker::register(
            &state,
            &waker_slot,
            counting_waker(Arc::clone(&first_count)),
        );
        loom::future::atomic_waker::register_by_ref(
            &state,
            &waker_slot,
            &counting_waker(Arc::clone(&second_count)),
        );

        loom::future::atomic_waker::wake(&state, &waker_slot);

        assert_eq!(first_count.load(Ordering::SeqCst), 0);
        assert_eq!(second_count.load(Ordering::SeqCst), 1);
        assert!(loom::future::atomic_waker::take_waker(&state, &waker_slot).is_none());

        loom::future::atomic_waker::register(
            &state,
            &waker_slot,
            counting_waker(Arc::clone(&first_count)),
        );

        let taken = loom::future::atomic_waker::take_waker(&state, &waker_slot);
        assert!(taken.is_some());

        taken.unwrap().wake();
        assert_eq!(first_count.load(Ordering::SeqCst), 1);
        assert_eq!(second_count.load(Ordering::SeqCst), 1);
    });
}

#[test]
fn atomic_waker_concurrent_register_and_wake_is_single_delivery_at_most() {
    loom::model(|| {
        let state = Arc::new(AtomicUsize::new(0));
        let waker_slot = Arc::new(UnsafeCell::new(None::<Waker>));
        let wake_count = Arc::new(StdAtomicUsize::new(0));

        let register_state = Arc::clone(&state);
        let register_slot = Arc::clone(&waker_slot);
        let register_count = Arc::clone(&wake_count);

        let register_thread = loom::thread::spawn(move || {
            let waker = counting_waker(register_count);
            loom::future::atomic_waker::register_by_ref(&register_state, &register_slot, &waker);
            loom::thread::yield_now();
        });

        let wake_state = Arc::clone(&state);
        let wake_slot = Arc::clone(&waker_slot);

        let wake_thread = loom::thread::spawn(move || {
            loom::thread::yield_now();
            loom::future::atomic_waker::wake(&wake_state, &wake_slot);
        });

        register_thread.join().unwrap();
        wake_thread.join().unwrap();

        let after_concurrent_wake = wake_count.load(Ordering::SeqCst);
        assert!(after_concurrent_wake <= 1);

        if let Some(waker) = loom::future::atomic_waker::take_waker(&state, &waker_slot) {
            waker.wake();
        }

        let final_count = wake_count.load(Ordering::SeqCst);
        assert!(final_count >= after_concurrent_wake);
        assert!(final_count <= 1);

        assert!(loom::future::atomic_waker::take_waker(&state, &waker_slot).is_none());
    });
}