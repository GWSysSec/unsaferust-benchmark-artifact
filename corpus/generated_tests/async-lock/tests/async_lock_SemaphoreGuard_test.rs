use async_lock::{Semaphore, SemaphoreGuard};

#[test]
fn forgotten_guard_permanently_consumes_one_permit_until_replenished() {
    let semaphore = Semaphore::new(2);

    let first = semaphore
        .try_acquire()
        .expect("first permit should be immediately available");
    let second = semaphore
        .try_acquire()
        .expect("second permit should be immediately available");

    assert!(
        semaphore.try_acquire().is_none(),
        "all initial permits should be exhausted"
    );

    SemaphoreGuard::forget(first);

    drop(second);

    let replacement_for_second = semaphore
        .try_acquire()
        .expect("dropping a normal guard should release its permit");

    assert!(
        semaphore.try_acquire().is_none(),
        "the forgotten guard's permit must not be released by drop"
    );

    drop(replacement_for_second);

    let recycled_normal_permit = semaphore
        .try_acquire()
        .expect("the non-forgotten permit should continue to cycle normally");

    assert!(
        semaphore.try_acquire().is_none(),
        "only one permit should be available because the other was forgotten"
    );

    drop(recycled_normal_permit);
}

#[test]
fn add_permits_restores_capacity_after_forget() {
    let semaphore = Semaphore::new(1);

    let guard = semaphore
        .try_acquire()
        .expect("single initial permit should be available");
    assert!(
        semaphore.try_acquire().is_none(),
        "single permit should be exhausted while held"
    );

    SemaphoreGuard::forget(guard);

    assert!(
        semaphore.try_acquire().is_none(),
        "forgotten permit should remain unavailable"
    );

    semaphore.add_permits(1);

    let restored = semaphore
        .try_acquire()
        .expect("added permit should become available after forget");
    assert!(
        semaphore.try_acquire().is_none(),
        "only the explicitly added permit should be available"
    );

    drop(restored);

    let restored_again = semaphore
        .try_acquire()
        .expect("dropping the restored guard should release the added permit");
    assert!(
        semaphore.try_acquire().is_none(),
        "the forgotten original permit should still be unavailable"
    );

    drop(restored_again);
}

#[test]
fn forgetting_one_of_several_blocking_acquisitions_preserves_other_release_paths() {
    let semaphore = Semaphore::new(3);

    let forgotten = semaphore.acquire_blocking();
    let normal_one = semaphore.acquire_blocking();
    let normal_two = semaphore.acquire_blocking();

    assert!(
        semaphore.try_acquire().is_none(),
        "three held guards should exhaust three permits"
    );

    SemaphoreGuard::forget(forgotten);
    drop(normal_one);

    let reacquired_one = semaphore
        .try_acquire()
        .expect("dropping one normal guard should release exactly one permit");
    assert!(
        semaphore.try_acquire().is_none(),
        "the forgotten permit and remaining held guard should keep capacity exhausted"
    );

    drop(normal_two);

    let reacquired_two = semaphore
        .try_acquire()
        .expect("dropping the second normal guard should release another permit");
    assert!(
        semaphore.try_acquire().is_none(),
        "forgotten permit should still reduce total usable capacity by one"
    );

    drop(reacquired_one);
    drop(reacquired_two);

    let available_a = semaphore.try_acquire();
    let available_b = semaphore.try_acquire();
    let unavailable_c = semaphore.try_acquire();

    assert!(available_a.is_some(), "first normal permit should be reusable");
    assert!(available_b.is_some(), "second normal permit should be reusable");
    assert!(
        unavailable_c.is_none(),
        "third permit should still be unavailable because it was forgotten"
    );
}