use std::num::NonZeroUsize;

#[cfg(unix)]
#[test]
fn poll_events_capacity_is_preserved_for_empty_buffers() {
    let capacity = NonZeroUsize::new(3).unwrap();
    let mut events = polling::Events::with_capacity(capacity);

    assert_eq!(events.capacity(), capacity);
    assert_eq!(events.iter().count(), 0);

    events.clear();

    assert_eq!(events.capacity(), capacity);
    assert_eq!(events.iter().count(), 0);

    let one_capacity = NonZeroUsize::new(1).unwrap();
    let one = polling::Events::with_capacity(one_capacity);
    assert_eq!(one.capacity(), one_capacity);
    assert_eq!(one.iter().count(), 0);

    let larger_capacity = NonZeroUsize::new(17).unwrap();
    let larger = polling::Events::with_capacity(larger_capacity);
    assert_eq!(larger.capacity(), larger_capacity);
    assert_eq!(larger.iter().count(), 0);
}

#[cfg(unix)]
#[test]
fn poll_event_basic_flags_are_reported_correctly() {
    let event = polling::Event::new(91, true, false);

    assert_eq!(event.key, 91);
    assert!(event.readable);
    assert!(!event.writable);
    assert!(!event.is_priority());
    assert_eq!(event.is_err(), Some(false));
}

#[cfg(unix)]
#[test]
fn poll_event_interest_constructors_preserve_interest_shape() {
    let readable = polling::Event::readable(7);

    assert_eq!(readable.key, 7);
    assert!(readable.readable);
    assert!(!readable.writable);
    assert!(!readable.is_priority());
    assert_eq!(readable.is_err(), Some(false));

    let writable = polling::Event::writable(8);

    assert_eq!(writable.key, 8);
    assert!(!writable.readable);
    assert!(writable.writable);
    assert!(!writable.is_priority());
    assert_eq!(writable.is_err(), Some(false));
}