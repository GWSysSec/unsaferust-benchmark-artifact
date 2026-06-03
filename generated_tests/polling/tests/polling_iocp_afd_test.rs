#[cfg(windows)]
#[test]
fn afd_get_or_init_initializes_once_and_reuses_value() {
    use polling::iocp::afd::LazyCell;
    use std::sync::atomic::{AtomicUsize, Ordering};

    let cell = LazyCell::<String>::new();
    let calls = AtomicUsize::new(0);

    let first = cell.get_or_init(|| {
        calls.fetch_add(1, Ordering::SeqCst);
        String::from("initialized")
    });

    assert_eq!(first, "initialized");
    assert_eq!(calls.load(Ordering::SeqCst), 1);

    let first_ptr = first.as_ptr();

    let second = cell.get_or_init(|| {
        calls.fetch_add(1, Ordering::SeqCst);
        String::from("replacement")
    });

    assert_eq!(second, "initialized");
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(second.as_ptr(), first_ptr);
}

#[cfg(not(windows))]
#[test]
fn non_windows_public_event_workflow_sanity_check() {
    use polling::{Event, Events};
    use std::num::NonZeroUsize;

    let mut event = Event::new(7, true, false);
    assert_eq!(event.key, 7);
    assert!(event.readable);
    assert!(!event.writable);
    assert!(!event.is_interrupt());
    assert!(!event.is_priority());

    event.set_interrupt(true);
    event.set_priority(true);
    assert!(event.is_interrupt());
    assert!(event.is_priority());

    event.clear_extra();
    assert!(!event.is_interrupt());
    assert!(!event.is_priority());

    let writable = Event::writable(11).with_interrupt().with_priority();
    assert_eq!(writable.key, 11);
    assert!(!writable.readable);
    assert!(writable.writable);
    assert!(writable.is_interrupt());
    assert!(writable.is_priority());

    let stripped = writable.with_no_extra();
    assert_eq!(stripped.key, 11);
    assert!(stripped.writable);
    assert!(!stripped.is_interrupt());
    assert!(!stripped.is_priority());

    let mut events = Events::with_capacity(NonZeroUsize::new(4).unwrap());
    assert!(events.is_empty());
    assert_eq!(events.len(), 0);
    assert!(events.capacity().get() >= 4);
    assert_eq!(events.iter().count(), 0);

    events.clear();
    assert!(events.is_empty());
}