use std::num::NonZeroUsize;
use std::time::Duration;

use polling::{Event, Events, Poller};

#[test]
fn event_extra_flags_can_be_set_and_cleared_without_changing_interest() {
    let mut event = Event::new(42, true, false);

    assert_eq!(event.key, 42);
    assert!(event.readable);
    assert!(!event.writable);
    assert!(!event.is_interrupt());
    assert!(!event.is_priority());
    assert_eq!(event.is_err(), Some(false));

    event.set_interrupt(true);
    event.set_priority(true);

    assert!(event.is_interrupt());
    assert!(event.is_priority());
    assert_eq!(event.is_err(), Some(false));

    event.clear_extra();

    assert_eq!(event.key, 42);
    assert!(event.readable);
    assert!(!event.writable);
    assert!(!event.is_interrupt());
    assert!(!event.is_priority());
    assert_eq!(event.is_err(), Some(false));

    let without_extra = Event::writable(7)
        .with_interrupt()
        .with_priority()
        .with_no_extra();

    assert_eq!(without_extra.key, 7);
    assert!(!without_extra.readable);
    assert!(without_extra.writable);
    assert!(!without_extra.is_interrupt());
    assert!(!without_extra.is_priority());
    assert_eq!(without_extra.is_err(), Some(false));
}

#[test]
fn events_collection_reports_capacity_and_clears_after_wait() {
    use std::net::{TcpListener, TcpStream};

    let requested_capacity = NonZeroUsize::new(8).unwrap();
    let mut events = Events::with_capacity(requested_capacity);

    assert!(events.is_empty());
    assert_eq!(events.len(), 0);
    assert!(events.capacity().get() >= requested_capacity.get());
    assert_eq!(events.iter().count(), 0);

    let listener = TcpListener::bind("127.0.0.1:0").expect("bind local listener");
    listener
        .set_nonblocking(true)
        .expect("make listener nonblocking");
    let addr = listener.local_addr().expect("read listener address");

    let poller = Poller::new().expect("create poller");

    unsafe {
        poller
            .add(&listener, Event::readable(1234))
            .expect("register listener");
    }

    let _client = TcpStream::connect(addr).expect("connect to listener");

    let count = poller
        .wait(&mut events, Some(Duration::from_secs(1)))
        .expect("wait for listener readiness");

    assert!(count >= 1);
    assert_eq!(events.len(), count);
    assert!(!events.is_empty());

    let observed: Vec<Event> = events.iter().collect();
    assert_eq!(observed.len(), count);
    assert!(
        observed
            .iter()
            .any(|event| event.key == 1234 && event.readable),
        "registered listener readiness event was not reported: {observed:?}"
    );

    events.clear();

    assert!(events.is_empty());
    assert_eq!(events.len(), 0);
    assert_eq!(events.iter().count(), 0);

    poller.delete(&listener).expect("delete listener registration");
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
fn kqueue_process_from_pid_can_register_current_process_and_be_deleted() {
    use polling::os::kqueue::{Process, ProcessOps};
    use std::num::NonZeroI32;

    let raw_pid = i32::try_from(std::process::id()).expect("current process id fits in i32");
    let pid = NonZeroI32::new(raw_pid).expect("current process id is non-zero");

    let process = unsafe { Process::from_pid(pid, ProcessOps::EXIT) };
    let poller = Poller::new().expect("create poller for process source");

    assert!(poller.supports_level() || poller.supports_edge());

    unsafe {
        poller
            .add(&process, Event::readable(9001))
            .expect("register current process by pid");
    }

    let mut events = Events::new();
    let count = poller
        .wait(&mut events, Some(Duration::from_millis(0)))
        .expect("nonblocking wait after process registration");

    assert_eq!(count, events.len());
    assert!(events.iter().all(|event| event.key == 9001));

    poller
        .delete(&process)
        .expect("delete current process registration");

    events.clear();
    assert!(events.is_empty());
}