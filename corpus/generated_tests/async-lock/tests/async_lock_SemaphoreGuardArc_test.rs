use std::sync::Arc;

use async_lock::{Semaphore, SemaphoreGuardArc};

#[test]
fn arc_guard_forget_consumes_permit_but_releases_arc_reference() {
    let semaphore = Arc::new(Semaphore::new(2));

    assert_eq!(Arc::strong_count(&semaphore), 1);

    let forgotten = semaphore
        .try_acquire_arc()
        .expect("first permit should be available");
    assert_eq!(
        Arc::strong_count(&semaphore),
        2,
        "an arc guard should hold one Arc reference"
    );

    let normal = semaphore
        .try_acquire_arc()
        .expect("second permit should be available");
    assert_eq!(
        Arc::strong_count(&semaphore),
        3,
        "two arc guards should each hold an Arc reference"
    );

    assert!(
        semaphore.try_acquire_arc().is_none(),
        "both initial permits should be exhausted"
    );

    SemaphoreGuardArc::forget(forgotten);

    assert_eq!(
        Arc::strong_count(&semaphore),
        2,
        "forgetting an arc guard must still release the guard's Arc reference"
    );
    assert!(
        semaphore.try_acquire_arc().is_none(),
        "the forgotten guard's permit must not be returned"
    );

    drop(normal);

    assert_eq!(
        Arc::strong_count(&semaphore),
        1,
        "dropping the remaining guard should release its Arc reference"
    );

    let recycled = semaphore
        .try_acquire_arc()
        .expect("dropping a non-forgotten guard should return its permit");
    assert_eq!(Arc::strong_count(&semaphore), 2);

    assert!(
        semaphore.try_acquire_arc().is_none(),
        "only the normally dropped permit should be available; the forgotten one remains consumed"
    );

    semaphore.add_permits(1);

    let added = semaphore
        .try_acquire_arc()
        .expect("an explicitly added permit should become available");
    assert_eq!(
        Arc::strong_count(&semaphore),
        3,
        "both live arc guards should be counted"
    );

    assert!(
        semaphore.try_acquire_arc().is_none(),
        "the semaphore should again be exhausted after taking the recycled and added permits"
    );

    drop(recycled);
    drop(added);

    let first_after_cleanup = semaphore
        .try_acquire_arc()
        .expect("recycled permit should be available after cleanup");
    let second_after_cleanup = semaphore
        .try_acquire_arc()
        .expect("added permit should be available after cleanup");

    assert!(
        semaphore.try_acquire_arc().is_none(),
        "there should be exactly two usable permits after cleanup"
    );

    drop(first_after_cleanup);
    drop(second_after_cleanup);

    assert_eq!(
        Arc::strong_count(&semaphore),
        1,
        "all guard Arc references should be released at the end"
    );
}