use loom::thread;
use std::cell::Cell;

loom::thread_local! {
    static TLS_COUNTER: Cell<usize> = Cell::new(0);
    static TLS_NAME: String = String::from("loom-worker");
}

#[test]
fn local_key_with_and_try_with_are_thread_local_and_return_values() {
    loom::model(|| {
        let initial = TLS_COUNTER.with(|counter| counter.get());
        assert_eq!(initial, 0);

        let after_set = TLS_COUNTER.with(|counter| {
            counter.set(10);
            counter.get()
        });
        assert_eq!(after_set, 10);

        let try_result = TLS_COUNTER
            .try_with(|counter| {
                let previous = counter.replace(counter.get() + 5);
                (previous, counter.get())
            })
            .expect("thread-local counter should be accessible");
        assert_eq!(try_result, (10, 15));

        let name_len = TLS_NAME
            .try_with(|name| {
                assert_eq!(name.as_str(), "loom-worker");
                name.len()
            })
            .expect("thread-local string should be accessible");
        assert_eq!(name_len, "loom-worker".len());

        let handle = thread::spawn(|| {
            let spawned_initial = TLS_COUNTER.with(|counter| counter.get());
            assert_eq!(spawned_initial, 0);

            let spawned_after_updates = TLS_COUNTER.with(|counter| {
                counter.set(3);
                counter.set(counter.get() * 7);
                counter.get()
            });
            assert_eq!(spawned_after_updates, 21);

            let observed_with_try = TLS_COUNTER
                .try_with(|counter| {
                    let old = counter.replace(34);
                    assert_eq!(old, 21);
                    counter.get()
                })
                .expect("spawned thread-local counter should be accessible");
            assert_eq!(observed_with_try, 34);

            let label = TLS_NAME.with(|name| {
                assert_eq!(name.as_bytes()[0], b'l');
                name.clone()
            });
            assert_eq!(label, "loom-worker");

            (
                spawned_initial,
                spawned_after_updates,
                observed_with_try,
                label,
            )
        });

        thread::yield_now();

        let main_still_unchanged = TLS_COUNTER.with(|counter| counter.get());
        assert_eq!(main_still_unchanged, 15);

        let joined = handle.join().expect("spawned loom thread should finish");
        assert_eq!(joined.0, 0);
        assert_eq!(joined.1, 21);
        assert_eq!(joined.2, 34);
        assert_eq!(joined.3, "loom-worker");

        let final_main_value = TLS_COUNTER
            .try_with(|counter| {
                counter.set(counter.get() + 1);
                counter.get()
            })
            .expect("main thread-local counter should remain accessible");
        assert_eq!(final_main_value, 16);
    });
}

#[test]
fn local_key_try_with_can_chain_computations_without_mutating_other_keys() {
    loom::model(|| {
        let before_name = TLS_NAME.with(|name| name.clone());
        assert_eq!(before_name, "loom-worker");

        let derived = TLS_COUNTER
            .try_with(|counter| {
                assert_eq!(counter.get(), 0);
                counter.set(2);
                counter.get()
            })
            .and_then(|value| TLS_NAME.try_with(|name| format!("{name}:{value}")))
            .expect("thread-local values should be accessible");

        assert_eq!(derived, "loom-worker:2");

        let counter_after_chain = TLS_COUNTER.with(|counter| {
            counter.set(counter.get() + 40);
            counter.get()
        });
        assert_eq!(counter_after_chain, 42);

        let name_after_chain = TLS_NAME
            .try_with(|name| {
                assert_eq!(name, "loom-worker");
                name.clone()
            })
            .expect("thread-local name should remain accessible");
        assert_eq!(name_after_chain, before_name);
    });
}