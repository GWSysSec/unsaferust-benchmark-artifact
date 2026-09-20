#[cfg(windows)]
#[test]
fn iocp_events_report_capacity_and_remain_empty_through_clear() {
    let mut events = polling::iocp::with_capacity(8);

    assert_eq!(events.capacity(), 8);
    assert_eq!(events.iter().count(), 0);

    events.clear();

    assert_eq!(events.capacity(), 8);
    assert_eq!(events.iter().count(), 0);

    let larger = polling::iocp::with_capacity(32);
    assert_eq!(larger.capacity(), 32);
    assert_eq!(larger.iter().count(), 0);
}

#[cfg(windows)]
#[test]
fn iocp_event_extra_flags_can_be_toggled_and_cleared() {
    let mut event = polling::Event::new(42, true, false);

    assert_eq!(event.key, 42);
    assert!(event.readable);
    assert!(!event.writable);
    assert!(!event.is_hup());
    assert!(!event.is_pri());

    event.set_hup(true);
    assert!(event.is_hup());
    assert!(!event.is_pri());
    assert_eq!(event.is_connect_failed(), Some(false));
    assert_eq!(event.is_err(), Some(false));

    event.set_pri(true);
    assert!(event.is_hup());
    assert!(event.is_pri());

    event.set_hup(false);
    assert!(!event.is_hup());
    assert!(event.is_pri());

    event.set_pri(false);
    assert!(!event.is_hup());
    assert!(!event.is_pri());
    assert_eq!(event.key, 42);
    assert!(event.readable);
    assert!(!event.writable);
}

#[cfg(windows)]
#[test]
fn iocp_completion_preserves_the_event_it_was_created_with() {
    let mut event = polling::Event::all(99);
    event.set_hup(true);
    event.set_pri(true);

    let completion = polling::iocp::Completion::new(event);
    let stored = completion.event();

    assert_eq!(stored.key, 99);
    assert!(stored.readable);
    assert!(stored.writable);
    assert!(stored.is_hup());
    assert!(stored.is_pri());
    assert_eq!(stored.is_connect_failed(), Some(false));
    assert_eq!(stored.is_err(), Some(false));
}

#[cfg(not(windows))]
#[test]
fn public_event_and_events_workflow_compiles_on_non_windows_platforms() {
    use std::num::NonZeroUsize;

    let mut event = polling::Event::readable(7);

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
    assert_eq!(event.key, 7);
    assert!(event.readable);
    assert!(!event.writable);

    let capacity = NonZeroUsize::new(4).unwrap();
    let mut events = polling::Events::with_capacity(capacity);

    assert_eq!(events.capacity(), capacity);
    assert!(events.is_empty());
    assert_eq!(events.len(), 0);
    assert_eq!(events.iter().count(), 0);

    events.clear();

    assert_eq!(events.capacity(), capacity);
    assert!(events.is_empty());
}