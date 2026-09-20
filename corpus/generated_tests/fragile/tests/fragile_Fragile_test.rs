use fragile::Fragile;
use std::cell::RefCell;
use std::rc::Rc;
use std::thread;

#[test]
fn try_get_mut_allows_multi_step_mutation_on_origin_thread() {
    let mut wrapped = Fragile::new(vec!["alpha".to_string(), "beta".to_string()]);

    assert!(wrapped.is_valid());
    assert_eq!(wrapped.get().len(), 2);

    {
        let values = match wrapped.try_get_mut() {
            Ok(values) => values,
            Err(_) => panic!("try_get_mut should succeed on the creating thread"),
        };
        values.push("gamma".to_string());
        values[0].push_str("-updated");
    }

    assert_eq!(
        wrapped.get(),
        &vec![
            "alpha-updated".to_string(),
            "beta".to_string(),
            "gamma".to_string()
        ]
    );

    {
        let values = match wrapped.try_get_mut() {
            Ok(values) => values,
            Err(_) => panic!("try_get_mut should remain valid after prior mutable borrow ends"),
        };
        values.retain(|value| value.contains('a'));
        values.push("delta".to_string());
    }

    let inner = wrapped.into_inner();
    assert_eq!(
        inner,
        vec![
            "alpha-updated".to_string(),
            "beta".to_string(),
            "gamma".to_string(),
            "delta".to_string()
        ]
    );
}

#[test]
fn try_get_mut_rejects_access_from_non_origin_thread_and_recovers_after_return() {
    let mut wrapped = Fragile::new(String::from("created on main thread"));

    {
        let value = match wrapped.try_get_mut() {
            Ok(value) => value,
            Err(_) => panic!("try_get_mut should succeed before sending to another thread"),
        };
        value.push_str(" and mutated once");
    }

    let handle = thread::spawn(move || {
        let mut wrapped = wrapped;

        assert!(!wrapped.is_valid());
        assert!(wrapped.try_get_mut().is_err());
        assert!(wrapped.try_get().is_err());

        wrapped
    });

    let mut wrapped = handle.join().expect("worker thread should not panic");

    assert!(wrapped.is_valid());
    assert_eq!(wrapped.get(), "created on main thread and mutated once");

    {
        let value = match wrapped.try_get_mut() {
            Ok(value) => value,
            Err(_) => panic!("try_get_mut should succeed again after returning to origin thread"),
        };
        value.push_str(" after return");
    }

    assert_eq!(
        wrapped.into_inner(),
        "created on main thread and mutated once after return"
    );
}

#[test]
fn try_get_mut_supports_non_send_inner_values_without_exposing_them_cross_thread() {
    let inner = Rc::new(RefCell::new(vec![1, 2, 3]));
    let mut wrapped = Fragile::new(inner);

    {
        let shared = match wrapped.try_get_mut() {
            Ok(shared) => shared,
            Err(_) => panic!("try_get_mut should succeed for non-Send inner value on origin thread"),
        };
        shared.borrow_mut().push(4);
        assert_eq!(Rc::strong_count(shared), 1);
    }

    let handle = thread::spawn(move || {
        let mut wrapped = wrapped;

        assert!(!wrapped.is_valid());
        assert!(wrapped.try_get_mut().is_err());

        wrapped
    });

    let mut wrapped = handle.join().expect("worker thread should not panic");

    {
        let shared = match wrapped.try_get_mut() {
            Ok(shared) => shared,
            Err(_) => panic!("try_get_mut should succeed after wrapper returns to origin thread"),
        };
        shared.borrow_mut().extend([5, 6]);
        assert_eq!(shared.borrow().as_slice(), &[1, 2, 3, 4, 5, 6]);
    }

    let shared = wrapped.into_inner();
    assert_eq!(Rc::strong_count(&shared), 1);
    assert_eq!(shared.borrow().as_slice(), &[1, 2, 3, 4, 5, 6]);
}