use polling::{Event, Events, Poller};
use std::num::NonZeroUsize;
use std::time::Duration;

#[test]
fn events_capacity_is_reported_and_preserved_across_operations() -> std::io::Result<()> {
    let requested_capacity = NonZeroUsize::new(4).unwrap();
    let mut events = Events::with_capacity(requested_capacity);

    assert_eq!(events.capacity(), requested_capacity);
    assert_eq!(events.len(), 0);
    assert!(events.is_empty());
    assert_eq!(events.iter().count(), 0);

    events.clear();

    assert_eq!(events.capacity(), requested_capacity);
    assert_eq!(events.len(), 0);
    assert!(events.is_empty());

    let poller = Poller::new()?;
    poller.notify()?;

    let observed = poller.wait(&mut events, Some(Duration::from_secs(1)))?;

    assert_eq!(
        observed, 0,
        "notify should wake the poller without inserting user-visible events"
    );
    assert_eq!(events.len(), 0);
    assert!(events.is_empty());
    assert_eq!(events.iter().count(), 0);
    assert_eq!(events.capacity(), requested_capacity);

    events.clear();

    assert_eq!(events.capacity(), requested_capacity);
    assert_eq!(events.len(), 0);
    assert!(events.is_empty());

    Ok(())
}

#[test]
fn event_extra_flags_can_be_cleared_without_affecting_events_capacity() {
    let requested_capacity = NonZeroUsize::new(1).unwrap();
    let events = Events::with_capacity(requested_capacity);

    let mut event = Event::all(7).with_interrupt().with_priority();

    assert_eq!(events.capacity(), requested_capacity);
    assert!(event.is_interrupt());
    assert!(event.is_priority());

    event.clear_extra();

    assert_eq!(events.capacity(), requested_capacity);
    assert!(!event.is_interrupt());
    assert!(!event.is_priority());

    let event_without_extra = Event::readable(11).with_interrupt().with_no_extra();

    assert_eq!(events.capacity(), requested_capacity);
    assert!(!event_without_extra.is_interrupt());
    assert!(!event_without_extra.is_priority());
}