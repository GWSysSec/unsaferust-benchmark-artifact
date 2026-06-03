use std::sync::Arc;

use async_lock::{Semaphore, SemaphoreGuard, SemaphoreGuardArc};

#[test]
fn forgetting_borrowed_semaphore_guard_consumes_permit_without_releasing_it() {
    let semaphore = Semaphore::new(2);

    let first = semaphore
        .try_acquire()
        .expect("first permit should be available");
    let second = semaphore
        .try_acquire()
        .expect("second permit should be available");

    assert!(
        semaphore.try_acquire().is_none(),
        "both permits should be exhausted while both guards are held"
    );

    SemaphoreGuard::forget(first);

    assert!(
        semaphore.try_acquire().is_none(),
        "forgetting a guard should not make its permit available"
    );

    drop(second);

    let recycled = semaphore
        .try_acquire()
        .expect("dropping the non-forgotten guard should release exactly one permit");

    assert!(
        semaphore.try_acquire().is_none(),
        "only the non-forgotten permit should have been recycled"
    );

    drop(recycled);

    let recycled_again = semaphore
        .try_acquire()
        .expect("the recycled permit should continue to be reusable");

    assert!(
        semaphore.try_acquire().is_none(),
        "the forgotten permit should still be unavailable"
    );

    semaphore.add_permits(1);

    let replenished = semaphore
        .try_acquire()
        .expect("adding one permit should compensate for the forgotten guard");

    assert!(
        semaphore.try_acquire().is_none(),
        "the semaphore should again have exactly two permits in circulation"
    );

    drop(recycled_again);
    drop(replenished);

    let after_cleanup_one = semaphore
        .try_acquire()
        .expect("first permit should be available after cleanup");
    let after_cleanup_two = semaphore
        .try_acquire()
        .expect("second permit should be available after cleanup");

    assert!(
        semaphore.try_acquire().is_none(),
        "no extra permits should have been created"
    );

    drop(after_cleanup_one);
    drop(after_cleanup_two);
}

#[test]
fn forgetting_arc_semaphore_guard_consumes_permit_and_drops_guard_arc_reference() {
    let semaphore = Arc::new(Semaphore::new(1));

    assert_eq!(
        Arc::strong_count(&semaphore),
        1,
        "only the test's Arc should exist initially"
    );

    let guard = semaphore
        .try_acquire_arc()
        .expect("the only permit should be available");

    assert_eq!(
        Arc::strong_count(&semaphore),
        2,
        "an arc guard should retain an Arc reference to the semaphore"
    );

    assert!(
        semaphore.try_acquire_arc().is_none(),
        "the only permit should be exhausted while the arc guard is held"
    );

    SemaphoreGuardArc::forget(guard);

    assert_eq!(
        Arc::strong_count(&semaphore),
        1,
        "forgeting an arc guard should consume the permit but drop its Arc reference"
    );

    assert!(
        semaphore.try_acquire_arc().is_none(),
        "the forgotten arc guard's permit should remain unavailable"
    );

    semaphore.add_permits(1);

    let replacement = semaphore
        .try_acquire_arc()
        .expect("adding a permit should make one arc guard acquirable again");

    assert_eq!(
        Arc::strong_count(&semaphore),
        2,
        "the replacement arc guard should hold an Arc reference"
    );

    drop(replacement);

    assert_eq!(
        Arc::strong_count(&semaphore),
        1,
        "dropping the replacement guard should release its Arc reference"
    );

    assert!(
        semaphore.try_acquire_arc().is_some(),
        "dropping the replacement guard should also release its permit"
    );
}