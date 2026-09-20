#[cfg(any(
    target_os = "macos",
    target_os = "ios",
    target_os = "tvos",
    target_os = "watchos",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "dragonfly"
))]
#[test]
fn kqueue_events_capacity_is_stable_across_clear_and_iteration() {
    let mut events = polling::kqueue::with_capacity(4);

    assert_eq!(events.capacity(), 4);
    assert_eq!(events.iter().count(), 0);

    events.clear();

    assert_eq!(events.capacity(), 4);
    assert_eq!(events.iter().count(), 0);

    let empty_capacity = polling::kqueue::with_capacity(0);
    assert_eq!(empty_capacity.capacity(), 0);
    assert_eq!(empty_capacity.iter().count(), 0);

    let larger = polling::kqueue::with_capacity(16);
    assert_eq!(larger.capacity(), 16);
    assert_eq!(larger.iter().count(), 0);
}

#[cfg(any(
    target_os = "macos",
    target_os = "ios",
    target_os = "tvos",
    target_os = "watchos",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "dragonfly"
))]
#[test]
fn kqueue_event_extra_status_methods_are_consistent_after_flag_toggles() {
    let mut event = polling::Event::new(123, true, false);

    assert_eq!(event.key, 123);
    assert!(event.readable);
    assert!(!event.writable);
    assert!(!event.is_hup());
    assert!(!event.is_pri());
    assert_eq!(event.is_connect_failed(), None);
    assert_eq!(event.is_err(), None);

    event.set_hup(true);
    assert!(!event.is_hup());
    assert!(!event.is_pri());
    assert_eq!(event.is_connect_failed(), None);
    assert_eq!(event.is_err(), None);

    event.set_pri(true);
    assert!(!event.is_hup());
    assert!(!event.is_pri());
    assert_eq!(event.is_connect_failed(), None);
    assert_eq!(event.is_err(), None);

    event.set_hup(false);
    event.set_pri(false);
    assert!(!event.is_hup());
    assert!(!event.is_pri());
    assert_eq!(event.is_connect_failed(), None);
    assert_eq!(event.is_err(), None);

    let mut all = polling::Event::all(777);
    assert_eq!(all.key, 777);
    assert!(all.readable);
    assert!(all.writable);

    all.set_hup(true);
    all.set_pri(true);
    assert!(!all.is_hup());
    assert!(!all.is_pri());
    assert_eq!(all.is_connect_failed(), None);
    assert_eq!(all.is_err(), None);
}

#[cfg(not(any(
    target_os = "macos",
    target_os = "ios",
    target_os = "tvos",
    target_os = "watchos",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
    target_os = "dragonfly"
)))]
#[test]
fn non_kqueue_platform_sanity_check() {
    let event = polling::Event::readable(5);

    assert_eq!(event.key, 5);
    assert!(event.readable);
    assert!(!event.writable);
}