use polling::Event;

#[test]
fn event_all_constructor() {
    let ev = Event::all(42);
    assert_eq!(ev.key, 42);
    assert!(ev.readable);
    assert!(ev.writable);
    assert!(!ev.is_interrupt());
    assert!(!ev.is_priority());

    assert_ne!(ev.is_err(), Some(true));
    assert_ne!(ev.is_connect_failed(), Some(true));
}

#[test]
fn event_writable_constructor() {
    let ev = Event::writable(7);
    assert_eq!(ev.key, 7);
    assert!(!ev.readable);
    assert!(ev.writable);
    assert!(!ev.is_interrupt());
    assert!(!ev.is_priority());

    let none = Event::none(7);
    assert!(!none.readable);
    assert!(!none.writable);
    assert_ne!(ev.writable, none.writable);
}

#[test]
fn event_set_and_with_interrupt() {
    let mut ev = Event::readable(1);
    assert!(!ev.is_interrupt());

    ev.set_interrupt(true);


    let after_set = ev.is_interrupt();

    ev.set_interrupt(false);
    assert!(!ev.is_interrupt());

    let with = Event::readable(1).with_interrupt();
    assert_eq!(with.key, 1);
    assert!(with.readable);
    assert_eq!(with.is_interrupt(), after_set);
}

#[test]
fn event_set_and_with_priority() {
    let mut ev = Event::writable(99);
    assert!(!ev.is_priority());

    ev.set_priority(true);
    let after_set = ev.is_priority();

    ev.set_priority(false);
    assert!(!ev.is_priority());

    let with = Event::writable(99).with_priority();
    assert_eq!(with.key, 99);
    assert!(with.writable);
    assert!(!with.readable);
    assert_eq!(with.is_priority(), after_set);
}

#[test]
fn event_combined_flags() {
    let ev = Event::all(123).with_interrupt().with_priority();
    assert_eq!(ev.key, 123);
    assert!(ev.readable);
    assert!(ev.writable);

    let int_active = ev.is_interrupt();
    let prio_active = ev.is_priority();

    let plain = Event::all(123);
    assert!(!plain.is_interrupt());
    assert!(!plain.is_priority());


    if int_active {
        assert_ne!(ev.is_interrupt(), plain.is_interrupt());
    }
    if prio_active {
        assert_ne!(ev.is_priority(), plain.is_priority());
    }


    assert_ne!(ev.is_err(), Some(true));
    assert_ne!(ev.is_connect_failed(), Some(true));
}

#[test]
fn event_error_predicates_on_normal_events() {
    let r = Event::readable(0);
    let w = Event::writable(0);
    let a = Event::all(0);
    let n = Event::none(0);

    for ev in [&r, &w, &a, &n] {

        assert_ne!(ev.is_err(), Some(true));
        assert_ne!(ev.is_connect_failed(), Some(true));
        assert!(!ev.is_interrupt());
        assert!(!ev.is_priority());
    }

    assert_eq!(r.key, 0);
    assert_eq!(w.key, 0);
    assert!(r.readable && !r.writable);
    assert!(w.writable && !w.readable);
    assert!(a.readable && a.writable);
    assert!(!n.readable && !n.writable);
}