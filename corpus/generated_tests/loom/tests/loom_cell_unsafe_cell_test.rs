use loom::cell::UnsafeCell;
use loom::sync::atomic::{AtomicBool, Ordering};
use loom::sync::Arc;
use loom::thread;

#[test]
fn unsafe_cell_with_and_with_mut_sequential_workflow() {
    loom::model(|| {
        let cell = UnsafeCell::new((String::from("loom"), 1usize));

        let new_len = unsafe {
            cell.with_mut(|ptr| {
                let value = &mut *ptr;
                assert_eq!(value.0, "loom");
                assert_eq!(value.1, 1);

                value.0.push_str("-checked");
                value.1 += value.0.len();

                value.0.len()
            })
        };

        assert_eq!(new_len, "loom-checked".len());

        let observed = unsafe {
            cell.with(|ptr| {
                let value = &*ptr;
                assert_eq!(value.0, "loom-checked");
                assert_eq!(value.1, 1 + "loom-checked".len());

                (value.0.clone(), value.1)
            })
        };

        assert_eq!(observed, (String::from("loom-checked"), 1 + "loom-checked".len()));

        let inner = cell.into_inner();
        assert_eq!(inner.0, "loom-checked");
        assert_eq!(inner.1, 1 + "loom-checked".len());
    });
}

#[test]
fn unsafe_cell_with_mut_publish_then_with_read_workflow() {
    struct Shared {
        data: UnsafeCell<Vec<u32>>,
        ready: AtomicBool,
    }

    loom::model(|| {
        let shared = Arc::new(Shared {
            data: UnsafeCell::new(Vec::new()),
            ready: AtomicBool::new(false),
        });

        let writer_shared = Arc::clone(&shared);
        let writer = thread::spawn(move || {
            let len_after_write = unsafe {
                writer_shared.data.with_mut(|ptr| {
                    let data = &mut *ptr;
                    assert!(data.is_empty());

                    data.push(10);
                    data.push(20);
                    data.push(30);

                    data.len()
                })
            };

            assert_eq!(len_after_write, 3);
            writer_shared.ready.store(true, Ordering::Release);
            len_after_write
        });

        let reader_shared = Arc::clone(&shared);
        let reader = thread::spawn(move || {
            if !reader_shared.ready.load(Ordering::Acquire) {
                return None;
            }

            let snapshot = unsafe {
                reader_shared.data.with(|ptr| {
                    let data = &*ptr;
                    assert_eq!(data.len(), 3);
                    assert_eq!(data[0], 10);
                    assert_eq!(data[1], 20);
                    assert_eq!(data[2], 30);

                    data.iter().copied().sum::<u32>()
                })
            };

            Some(snapshot)
        });

        let written_len = writer.join().expect("writer thread panicked");
        assert_eq!(written_len, 3);

        let maybe_sum = reader.join().expect("reader thread panicked");
        assert!(matches!(maybe_sum, None | Some(60)));

        assert!(shared.ready.load(Ordering::Acquire));

        let final_state = unsafe {
            shared.data.with(|ptr| {
                let data = &*ptr;
                assert_eq!(data.as_slice(), &[10, 20, 30]);

                (data.len(), data.iter().copied().sum::<u32>())
            })
        };

        assert_eq!(final_state, (3, 60));
    });
}