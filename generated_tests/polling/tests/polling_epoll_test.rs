use polling::{Event, Events};
use std::num::NonZeroUsize;

#[test]
fn events_capacity_is_reported_and_stable_after_clear_and_iteration() {
    let capacity = NonZeroUsize::new(7).unwrap();
    let mut events = Events::with_capacity(capacity);

    assert_eq!(events.capacity(), capacity);
    assert_eq!(events.iter().count(), 0);

    events.clear();

    assert_eq!(events.capacity(), capacity);
    assert_eq!(events.iter().count(), 0);

    let larger_capacity = NonZeroUsize::new(32).unwrap();
    let larger = Events::with_capacity(larger_capacity);
    assert_eq!(larger.capacity(), larger_capacity);
    assert_eq!(larger.iter().count(), 0);
}

#[test]
fn event_readable_writable_and_priority_state_is_reported() {
    let event = Event::new(99, true, false);

    assert_eq!(event.key, 99);
    assert!(event.readable);
    assert!(!event.writable);
    assert!(!event.is_priority());

    let readable = Event::readable(10);
    assert_eq!(readable.key, 10);
    assert!(readable.readable);
    assert!(!readable.writable);
    assert!(!readable.is_priority());

    let writable = Event::writable(11);
    assert_eq!(writable.key, 11);
    assert!(!writable.readable);
    assert!(writable.writable);
    assert!(!writable.is_priority());
}

#[test]
fn event_error_helpers_are_consistent_for_regular_events() {
    let event = Event::writable(123);

    assert_eq!(event.key, 123);
    assert!(!event.readable);
    assert!(event.writable);
    assert_eq!(event.is_err(), Some(false));
    assert_eq!(event.is_connect_failed(), Some(false));
    assert_eq!(event.is_connect_failed(), event.is_err());
}