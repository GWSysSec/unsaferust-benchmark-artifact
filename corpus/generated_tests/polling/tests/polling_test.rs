use polling::{Event, Events};
use std::num::NonZeroUsize;

#[test]
fn extra_event_flags_can_be_enabled_chained_disabled_and_cleared() {
    let mut event = Event::new(123, true, false);

    assert_eq!(event.key, 123);
    assert!(event.readable);
    assert!(!event.writable);
    assert!(!Event::is_interrupt(&event));
    assert!(!Event::is_priority(&event));

    Event::set_interrupt(&mut event, true);
    assert!(Event::is_interrupt(&event));
    assert!(!Event::is_priority(&event));
    assert_eq!(event.key, 123);
    assert!(event.readable);
    assert!(!event.writable);

    Event::set_priority(&mut event, true);
    assert!(Event::is_interrupt(&event));
    assert!(Event::is_priority(&event));

    Event::set_interrupt(&mut event, false);
    assert!(!Event::is_interrupt(&event));
    assert!(Event::is_priority(&event));

    Event::set_priority(&mut event, false);
    assert!(!Event::is_interrupt(&event));
    assert!(!Event::is_priority(&event));

    let chained = Event::writable(77).with_interrupt().with_priority();
    assert_eq!(chained.key, 77);
    assert!(!chained.readable);
    assert!(chained.writable);
    assert!(Event::is_interrupt(&chained));
    assert!(Event::is_priority(&chained));

    let cleared = chained.with_no_extra();
    assert_eq!(cleared.key, 77);
    assert!(!cleared.readable);
    assert!(cleared.writable);
    assert!(!Event::is_interrupt(&cleared));
    assert!(!Event::is_priority(&cleared));
}

#[test]
fn error_status_queries_are_stable_across_extra_flag_workflow() {
    let mut event = Event::all(9);

    assert_eq!(event.key, 9);
    assert!(event.readable);
    assert!(event.writable);

    let initial_connect_failed = Event::is_connect_failed(&event);
    let initial_err = Event::is_err(&event);
    if let Some(connect_failed) = initial_connect_failed {
        assert!(!connect_failed);
    }
    if let Some(err) = initial_err {
        assert!(!err);
    }

    Event::set_interrupt(&mut event, true);
    Event::set_priority(&mut event, true);

    assert!(Event::is_interrupt(&event));
    assert!(Event::is_priority(&event));
    assert_eq!(Event::is_connect_failed(&event), initial_connect_failed);
    assert_eq!(Event::is_err(&event), initial_err);

    Event::clear_extra(&mut event);

    assert!(!Event::is_interrupt(&event));
    assert!(!Event::is_priority(&event));
    assert_eq!(Event::is_connect_failed(&event), initial_connect_failed);
    assert_eq!(Event::is_err(&event), initial_err);
    assert_eq!(event.key, 9);
    assert!(event.readable);
    assert!(event.writable);
}

#[test]
fn events_capacity_is_nonzero_preserved_by_clear_and_iterates_empty_buffer() {
    let requested = NonZeroUsize::new(16).expect("literal is nonzero");
    let mut events = Events::with_capacity(requested);

    assert_eq!(Events::capacity(&events), requested);
    assert!(events.is_empty());
    assert_eq!(events.len(), 0);
    assert_eq!(events.iter().count(), 0);

    events.clear();

    assert_eq!(Events::capacity(&events), requested);
    assert!(events.is_empty());
    assert_eq!(events.len(), 0);
    assert_eq!(events.iter().count(), 0);

    let default_events = Events::new();
    assert!(Events::capacity(&default_events).get() > 0);
    assert!(default_events.is_empty());
    assert_eq!(default_events.len(), 0);
}