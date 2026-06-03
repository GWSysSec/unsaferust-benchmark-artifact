#[cfg(any(target_os = "illumos", target_os = "solaris"))]
#[test]
fn port_events_capacity_is_preserved_across_empty_workflow() {
    let mut events = polling::port::with_capacity(0);

    assert_eq!(events.capacity(), 0);
    assert_eq!(events.iter().count(), 0);

    events.clear();

    assert_eq!(events.capacity(), 0);
    assert_eq!(events.iter().count(), 0);

    let mut larger = polling::port::with_capacity(16);

    assert_eq!(larger.capacity(), 16);
    assert_eq!(larger.iter().count(), 0);

    larger.clear();

    assert_eq!(larger.capacity(), 16);
    assert_eq!(larger.iter().count(), 0);
}

#[cfg(any(target_os = "illumos", target_os = "solaris"))]
#[test]
fn port_event_hup_and_pri_flags_can_be_toggled_independently() {
    let mut event = polling::Event::new(42, true, false);

    assert_eq!(event.key, 42);
    assert!(event.readable);
    assert!(!event.writable);
    assert!(!event.is_hup());
    assert!(!event.is_pri());

    event.set_hup(true);

    assert!(event.is_hup());
    assert!(!event.is_pri());
    assert_eq!(event.key, 42);
    assert!(event.readable);
    assert!(!event.writable);

    event.set_pri(true);

    assert!(event.is_hup());
    assert!(event.is_pri());

    event.set_hup(false);

    assert!(!event.is_hup());
    assert!(event.is_pri());

    event.set_pri(false);

    assert!(!event.is_hup());
    assert!(!event.is_pri());
}

#[cfg(any(target_os = "illumos", target_os = "solaris"))]
#[test]
fn port_event_error_queries_are_stable_while_flags_change() {
    let mut event = polling::Event::writable(7);

    let initial_connect_failed = event.is_connect_failed();
    let initial_err = event.is_err();

    assert_eq!(event.key, 7);
    assert!(!event.readable);
    assert!(event.writable);
    assert!(!event.is_hup());
    assert!(!event.is_pri());

    event.set_hup(true);
    let hup_connect_failed = event.is_connect_failed();
    let hup_err = event.is_err();

    assert!(event.is_hup());
    assert!(!event.is_pri());
    assert_eq!(hup_connect_failed.is_some(), initial_connect_failed.is_some());
    assert_eq!(hup_err.is_some(), initial_err.is_some());

    event.set_pri(true);
    let hup_and_pri_connect_failed = event.is_connect_failed();
    let hup_and_pri_err = event.is_err();

    assert!(event.is_hup());
    assert!(event.is_pri());
    assert_eq!(
        hup_and_pri_connect_failed.is_some(),
        hup_connect_failed.is_some()
    );
    assert_eq!(hup_and_pri_err.is_some(), hup_err.is_some());

    event.set_hup(false);
    event.set_pri(false);

    assert!(!event.is_hup());
    assert!(!event.is_pri());
    assert_eq!(event.is_connect_failed().is_some(), initial_connect_failed.is_some());
    assert_eq!(event.is_err().is_some(), initial_err.is_some());
}