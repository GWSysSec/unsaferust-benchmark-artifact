use fragile::Fragile;
use std::thread;

#[test]
fn lowercase_fragile_try_get_mut_supports_origin_thread_workflow() {
    let mut wrapped = Fragile::new(vec![
        "alpha".to_string(),
        "beta".to_string(),
        "gamma".to_string(),
    ]);

    assert!(wrapped.is_valid());
    assert_eq!(wrapped.get().len(), 3);
    assert_eq!(wrapped.get()[1], "beta");

    {
        let values = wrapped
            .try_get_mut()
            .expect("try_get_mut should succeed on the thread that created the fragile value");
        values[0].push_str("-edited");
        values.retain(|value| value.contains('a'));
        values.push("delta".to_string());
    }

    assert!(wrapped.is_valid());
    assert_eq!(
        wrapped.get(),
        &vec![
            "alpha-edited".to_string(),
            "beta".to_string(),
            "gamma".to_string(),
            "delta".to_string(),
        ]
    );

    {
        let values = wrapped
            .try_get_mut()
            .expect("try_get_mut should remain usable after an earlier mutable borrow ends");
        values.sort();
        values.dedup();
        values.push("epsilon".to_string());
    }

    assert_eq!(wrapped.get().first().map(String::as_str), Some("alpha-edited"));
    assert!(wrapped.get().iter().any(|value| value == "epsilon"));

    let inner = wrapped.into_inner();
    assert_eq!(inner.len(), 5);
    assert!(inner.contains(&"delta".to_string()));
}

#[test]
fn lowercase_fragile_try_get_mut_rejects_access_from_non_origin_thread() {
    let mut wrapped = Fragile::new(vec![10, 20, 30]);

    {
        let numbers = wrapped
            .try_get_mut()
            .expect("initial mutation should succeed on the creating thread");
        numbers.push(40);
        numbers[0] = 5;
    }

    assert_eq!(wrapped.get(), &vec![5, 20, 30, 40]);

    let join_handle = thread::spawn(move || {
        assert!(!wrapped.is_valid());
        assert!(
            wrapped.try_get_mut().is_err(),
            "try_get_mut must fail when called from a different thread"
        );
        wrapped
    });

    let mut wrapped = join_handle
        .join()
        .expect("worker thread should return the fragile wrapper without panicking");

    assert!(wrapped.is_valid());
    assert_eq!(wrapped.get(), &vec![5, 20, 30, 40]);

    {
        let numbers = wrapped
            .try_get_mut()
            .expect("try_get_mut should work again after the value returns to its origin thread");
        numbers.push(50);
        numbers.retain(|number| *number >= 20);
    }

    assert_eq!(wrapped.try_into_inner().ok(), Some(vec![20, 30, 40, 50]));
}