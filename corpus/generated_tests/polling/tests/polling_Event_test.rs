use polling::Event;

#[test]
fn constructors_set_expected_key_and_readiness_bits() {
    let all = Event::all(11);
    assert_eq!(all.key, 11);
    assert!(all.readable);
    assert!(all.writable);

    let writable = Event::writable(22);
    assert_eq!(writable.key, 22);
    assert!(!writable.readable);
    assert!(writable.writable);

    let equivalent_all = Event::new(11, true, true);
    assert_eq!(all.key, equivalent_all.key);
    assert_eq!(all.readable, equivalent_all.readable);
    assert_eq!(all.writable, equivalent_all.writable);

    let equivalent_writable = Event::new(22, false, true);
    assert_eq!(writable.key, equivalent_writable.key);
    assert_eq!(writable.readable, equivalent_writable.readable);
    assert_eq!(writable.writable, equivalent_writable.writable);
}

#[test]
fn interrupt_flag_can_be_added_chained_and_cleared_without_changing_readiness() {
    let mut event = Event::all(7);
    assert_eq!(event.key, 7);
    assert!(event.readable);
    assert!(event.writable);
    assert!(!event.is_interrupt());

    event.set_interrupt(true);
    assert_eq!(event.key, 7);
    assert!(event.readable);
    assert!(event.writable);

    let enabled_by_setter = event.is_interrupt();

    let chained = Event::all(7).with_interrupt();
    assert_eq!(chained.key, 7);
    assert!(chained.readable);
    assert!(chained.writable);
    assert_eq!(chained.is_interrupt(), enabled_by_setter);

    event.set_interrupt(false);
    assert_eq!(event.key, 7);
    assert!(event.readable);
    assert!(event.writable);
    assert!(!event.is_interrupt());

    let mut writable = Event::writable(8).with_interrupt();
    writable.set_interrupt(false);
    assert_eq!(writable.key, 8);
    assert!(!writable.readable);
    assert!(writable.writable);
    assert!(!writable.is_interrupt());
}

#[test]
fn priority_flag_can_be_added_chained_and_cleared_without_changing_readiness() {
    let mut event = Event::writable(9);
    assert_eq!(event.key, 9);
    assert!(!event.readable);
    assert!(event.writable);
    assert!(!event.is_priority());

    event.set_priority(true);
    assert_eq!(event.key, 9);
    assert!(!event.readable);
    assert!(event.writable);

    let enabled_by_setter = event.is_priority();

    let chained = Event::writable(9).with_priority();
    assert_eq!(chained.key, 9);
    assert!(!chained.readable);
    assert!(chained.writable);
    assert_eq!(chained.is_priority(), enabled_by_setter);

    event.set_priority(false);
    assert_eq!(event.key, 9);
    assert!(!event.readable);
    assert!(event.writable);
    assert!(!event.is_priority());

    let mut all = Event::all(10).with_priority();
    all.set_priority(false);
    assert_eq!(all.key, 10);
    assert!(all.readable);
    assert!(all.writable);
    assert!(!all.is_priority());
}

#[test]
fn interrupt_and_priority_flags_are_independent_when_supported() {
    let mut event = Event::all(42);

    event.set_interrupt(true);
    let interrupt_after_set = event.is_interrupt();
    let priority_after_interrupt_set = event.is_priority();

    event.set_priority(true);
    let priority_after_set = event.is_priority();

    assert_eq!(event.key, 42);
    assert!(event.readable);
    assert!(event.writable);

    if interrupt_after_set {
        assert!(event.is_interrupt());
    } else {
        assert!(!event.is_interrupt());
    }

    if priority_after_set {
        assert!(event.is_priority());
    } else {
        assert!(!event.is_priority());
    }

    if !priority_after_interrupt_set && priority_after_set {
        event.set_interrupt(false);
        assert!(!event.is_interrupt());
        assert!(event.is_priority());
    }

    event.set_priority(false);
    assert!(!event.is_priority());
}

#[test]
fn constructed_events_report_no_connection_error_or_unsupported() {
    let plain = Event::all(100);
    assert!(matches!(plain.is_err(), None | Some(false)));

    let writable = Event::writable(101);
    assert!(matches!(writable.is_err(), None | Some(false)));

    let with_extra = Event::all(102).with_interrupt().with_priority();
    assert!(matches!(with_extra.is_err(), None | Some(false)));

    assert_eq!(plain.key, 100);
    assert_eq!(writable.key, 101);
    assert_eq!(with_extra.key, 102);
}