use curl::multi::WaitFd;

#[test]
fn waitfd_write_and_priority_read_flags_are_configurable_and_initially_unset() {
    curl::init();

    let mut fd = WaitFd::new();

    assert!(
        !curl::multi::WaitFd::received_write(&fd),
        "a newly-created WaitFd must not report a write event before waiting"
    );
    assert!(
        !curl::multi::WaitFd::received_priority_read(&fd),
        "a newly-created WaitFd must not report a priority-read event before waiting"
    );

    curl::multi::WaitFd::poll_on_write(&mut fd, true);
    assert!(
        !curl::multi::WaitFd::received_write(&fd),
        "requesting write polling alone must not synthesize a received write event"
    );

    curl::multi::WaitFd::poll_on_priority_read(&mut fd, true);
    assert!(
        !curl::multi::WaitFd::received_priority_read(&fd),
        "requesting priority-read polling alone must not synthesize a received priority-read event"
    );

    curl::multi::WaitFd::poll_on_write(&mut fd, false);
    curl::multi::WaitFd::poll_on_priority_read(&mut fd, false);

    assert!(
        !curl::multi::WaitFd::received_write(&fd),
        "disabling write polling must leave received write status unset without a wait"
    );
    assert!(
        !curl::multi::WaitFd::received_priority_read(&fd),
        "disabling priority-read polling must leave received priority-read status unset without a wait"
    );
}

#[test]
fn waitfd_builder_methods_can_be_chained_and_toggled_repeatedly() {
    curl::init();

    let mut fd = WaitFd::new();

    curl::multi::WaitFd::poll_on_write(
        curl::multi::WaitFd::poll_on_priority_read(
            curl::multi::WaitFd::poll_on_write(&mut fd, true),
            true,
        ),
        false,
    );

    assert!(
        !curl::multi::WaitFd::received_write(&fd),
        "chained configuration should not mark write as received before a wait"
    );
    assert!(
        !curl::multi::WaitFd::received_priority_read(&fd),
        "chained configuration should not mark priority-read as received before a wait"
    );

    curl::multi::WaitFd::poll_on_write(&mut fd, true)
        .poll_on_priority_read(false)
        .poll_on_priority_read(true)
        .poll_on_write(false)
        .poll_on_write(true);

    assert!(
        !curl::multi::WaitFd::received_write(&fd),
        "repeated write polling toggles should not create a received write event"
    );
    assert!(
        !curl::multi::WaitFd::received_priority_read(&fd),
        "repeated priority-read polling toggles should not create a received priority-read event"
    );
}