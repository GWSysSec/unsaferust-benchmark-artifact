use loom::cell::Cell;
use loom::sync::Arc;
use loom::thread;

#[test]
fn cell_replace_basic() {
    loom::model(|| {
        let cell = Cell::new(42u32);


        let val = cell.get();
        assert_eq!(val, 42);


        let old = cell.replace(100);
        assert_eq!(old, 42);


        let current = cell.get();
        assert_eq!(current, 100);


        let old2 = cell.replace(200);
        assert_eq!(old2, 100);

        let current2 = cell.get();
        assert_eq!(current2, 200);


        let old3 = cell.replace(200);
        assert_eq!(old3, 200);

        let current3 = cell.get();
        assert_eq!(current3, 200);


        let old4 = cell.replace(0);
        assert_eq!(old4, 200);
        assert_eq!(cell.get(), 0);
    });
}

#[test]
fn cell_take_basic() {
    loom::model(|| {
        let cell = Cell::new(99i32);


        let val = cell.get();
        assert_eq!(val, 99);


        let taken = cell.take();
        assert_eq!(taken, 99);


        let after_take = cell.get();
        assert_eq!(after_take, 0);


        let taken2 = cell.take();
        assert_eq!(taken2, 0);

        let after_take2 = cell.get();
        assert_eq!(after_take2, 0);


        cell.set(555);
        assert_eq!(cell.get(), 555);
        let taken3 = cell.take();
        assert_eq!(taken3, 555);
        assert_eq!(cell.get(), 0);
    });
}

#[test]
fn cell_swap_basic() {
    loom::model(|| {
        let cell_a = Cell::new(10u64);
        let cell_b = Cell::new(20u64);


        assert_eq!(cell_a.get(), 10);
        assert_eq!(cell_b.get(), 20);


        cell_a.swap(&cell_b);


        assert_eq!(cell_a.get(), 20);
        assert_eq!(cell_b.get(), 10);


        cell_a.swap(&cell_b);
        assert_eq!(cell_a.get(), 10);
        assert_eq!(cell_b.get(), 20);
    });
}

#[test]
fn cell_swap_same_values() {
    loom::model(|| {
        let cell_a = Cell::new(42u32);
        let cell_b = Cell::new(42u32);

        assert_eq!(cell_a.get(), 42);
        assert_eq!(cell_b.get(), 42);

        cell_a.swap(&cell_b);

        assert_eq!(cell_a.get(), 42);
        assert_eq!(cell_b.get(), 42);


        cell_a.set(1);
        cell_b.set(2);
        assert_eq!(cell_a.get(), 1);
        assert_eq!(cell_b.get(), 2);

        cell_a.swap(&cell_b);
        assert_eq!(cell_a.get(), 2);
        assert_eq!(cell_b.get(), 1);
    });
}

#[test]
fn cell_replace_take_combined() {
    loom::model(|| {
        let cell = Cell::new(String::from("hello"));


        let initial = cell.replace(String::from("world"));
        assert_eq!(initial, "hello");

        let current = cell.replace(String::from("foo"));
        assert_eq!(current, "world");

        assert_eq!(cell.replace(String::from("bar")), "foo");


        let taken = cell.take();
        assert_eq!(taken, "bar");

        let after_take = cell.take();
        assert_eq!(after_take, "");


        let old = cell.replace(String::from("baz"));
        assert_eq!(old, "");

        let final_val = cell.take();
        assert_eq!(final_val, "baz");
    });
}

#[test]
fn cell_swap_with_defaults() {
    loom::model(|| {
        let cell_a: Cell<i32> = Cell::new(0);
        let cell_b: Cell<i32> = Cell::new(77);

        assert_eq!(cell_a.get(), 0);
        assert_eq!(cell_b.get(), 77);

        cell_a.swap(&cell_b);

        assert_eq!(cell_a.get(), 77);
        assert_eq!(cell_b.get(), 0);


        let taken = cell_a.take();
        assert_eq!(taken, 77);
        assert_eq!(cell_a.get(), 0);


        assert_eq!(cell_a.get(), 0);
        assert_eq!(cell_b.get(), 0);
    });
}

#[test]
fn cell_replace_boundary_values() {
    loom::model(|| {
        let cell = Cell::new(u64::MAX);

        assert_eq!(cell.get(), u64::MAX);

        let old = cell.replace(u64::MIN);
        assert_eq!(old, u64::MAX);
        assert_eq!(cell.get(), u64::MIN);

        let old2 = cell.replace(u64::MAX / 2);
        assert_eq!(old2, u64::MIN);
        assert_eq!(cell.get(), u64::MAX / 2);

        let old3 = cell.replace(1);
        assert_eq!(old3, u64::MAX / 2);
        assert_eq!(cell.get(), 1);

        let taken = cell.take();
        assert_eq!(taken, 1);
        assert_eq!(cell.get(), 0);
    });
}

#[test]
fn cell_swap_multiple_rounds() {
    loom::model(|| {
        let a = Cell::new(1i32);
        let b = Cell::new(2i32);
        let c = Cell::new(3i32);


        a.swap(&b);
        assert_eq!(a.get(), 2);
        assert_eq!(b.get(), 1);

        b.swap(&c);
        assert_eq!(b.get(), 3);
        assert_eq!(c.get(), 1);


        assert_eq!(a.get(), 2);
        assert_eq!(b.get(), 3);
        assert_eq!(c.get(), 1);


        let old_a = a.replace(10);
        assert_eq!(old_a, 2);
        assert_eq!(a.get(), 10);
    });
}

#[test]
fn cell_take_with_bool() {
    loom::model(|| {
        let cell = Cell::new(true);

        assert_eq!(cell.get(), true);

        let taken = cell.take();
        assert_eq!(taken, true);


        assert_eq!(cell.get(), false);

        let taken2 = cell.take();
        assert_eq!(taken2, false);
        assert_eq!(cell.get(), false);

        cell.set(true);
        assert_eq!(cell.get(), true);

        let replaced = cell.replace(false);
        assert_eq!(replaced, true);
        assert_eq!(cell.get(), false);
    });
}

#[test]
fn cell_operations_in_thread() {
    loom::model(|| {
        let data = Arc::new(loom::sync::Mutex::new(0u32));

        let data2 = data.clone();
        let handle = thread::spawn(move || {
            let cell = Cell::new(50u32);
            let old = cell.replace(100);
            assert_eq!(old, 50);
            let taken = cell.take();
            assert_eq!(taken, 100);
            assert_eq!(cell.get(), 0);

            if let Ok(mut guard) = data2.lock() {
                *guard = 42;
            }
        });

        handle.join().unwrap();

        if let Ok(guard) = data.lock() {
            assert_eq!(*guard, 42);
        }


        let main_cell = Cell::new(999u32);
        let swapped = main_cell.replace(0);
        assert_eq!(swapped, 999);
        assert_eq!(main_cell.get(), 0);
    });
}