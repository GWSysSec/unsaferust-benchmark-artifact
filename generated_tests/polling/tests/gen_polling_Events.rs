use polling::{Event, Events, Poller};
use std::io::{self, Write};
use std::net::{TcpListener, TcpStream};
use std::time::Duration;

fn tcp_pair() -> io::Result<(TcpStream, TcpStream)> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let a = TcpStream::connect(listener.local_addr()?)?;
    let (b, _) = listener.accept()?;
    Ok((a, b))
}

#[test]
fn capacity_is_positive_and_stable() {
    let events = Events::new();
    let cap = events.capacity();
    assert!(cap.get() >= 1);
    assert_ne!(cap.get(), 0);

    assert_eq!(cap, events.capacity());
    assert_eq!(events.len(), 0);
    assert!(events.is_empty());

    let cap2 = events.capacity();
    assert_eq!(cap.get(), cap2.get());
    assert_eq!(events.iter().count(), 0);
    assert_eq!(events.capacity().get(), cap.get());
}

#[test]
fn capacity_unchanged_after_clear() {
    let mut events = Events::new();
    let initial_cap = events.capacity();
    assert!(initial_cap.get() > 0);
    assert_eq!(events.len(), 0);

    events.clear();
    let cap_after_clear = events.capacity();
    assert_eq!(initial_cap.get(), cap_after_clear.get());
    assert_eq!(events.len(), 0);
    assert!(events.is_empty());
    assert_eq!(events.iter().count(), 0);

    events.clear();
    events.clear();
    assert_eq!(events.capacity().get(), initial_cap.get());
    assert_ne!(events.capacity().get(), 0);
}

#[test]
fn capacity_accommodates_events_after_wait() -> io::Result<()> {
    let poller = Poller::new()?;
    let (reader, mut writer) = tcp_pair()?;
    unsafe {
        poller.add(&reader, Event::readable(42))?;
    }

    let mut events = Events::new();
    let cap_before = events.capacity();
    assert!(cap_before.get() >= 1);
    assert_eq!(events.len(), 0);
    assert!(events.is_empty());

    writer.write_all(&[1])?;
    let n = poller.wait(&mut events, Some(Duration::from_secs(5)))?;
    assert_eq!(n, 1);
    assert_eq!(events.len(), 1);
    assert!(!events.is_empty());

    let cap_after = events.capacity();

    assert!(cap_after.get() >= events.len());
    assert!(cap_after.get() >= cap_before.get());
    assert_ne!(cap_after.get(), 0);

    assert_eq!(
        events.iter().next().unwrap().with_no_extra(),
        Event::readable(42)
    );

    events.clear();
    assert_eq!(events.len(), 0);
    assert_eq!(events.capacity().get(), cap_after.get());

    poller.delete(&reader)?;
    Ok(())
}

#[test]
fn capacity_multiple_instances_consistent() {
    let a = Events::new();
    let b = Events::new();
    let c = Events::new();
    assert_eq!(a.capacity().get(), b.capacity().get());
    assert_eq!(b.capacity().get(), c.capacity().get());
    assert!(a.capacity().get() >= 1);
    assert_eq!(a.len(), 0);
    assert_eq!(b.len(), 0);
    assert_eq!(c.len(), 0);
    assert!(a.is_empty());
    assert!(b.is_empty());
    assert!(c.is_empty());
}