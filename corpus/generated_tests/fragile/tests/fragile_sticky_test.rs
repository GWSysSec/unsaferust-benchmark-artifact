use std::collections::BTreeMap;
use std::thread;

#[test]
fn lowercase_sticky_try_get_mut_supports_origin_thread_mutation_and_rejects_other_threads() {
    fragile::stack_token!(token);

    let mut initial_scores = BTreeMap::new();
    initial_scores.insert("alpha".to_string(), 10);
    initial_scores.insert("beta".to_string(), 20);

    let mut wrapped = fragile::Sticky::new(initial_scores);

    assert!(wrapped.is_valid());
    assert_eq!(wrapped.try_get(&token).unwrap().len(), 2);
    assert_eq!(wrapped.try_get(&token).unwrap().get("alpha"), Some(&10));
    assert_eq!(wrapped.try_get(&token).unwrap().get("beta"), Some(&20));

    {
        let scores = wrapped
            .try_get_mut(&token)
            .expect("sticky value should be mutably accessible on its originating thread");
        *scores.get_mut("alpha").unwrap() += 5;
        scores.insert("gamma".to_string(), 30);
        assert_eq!(scores.get("alpha"), Some(&15));
        assert_eq!(scores.len(), 3);
    }

    assert!(wrapped.is_valid());
    assert_eq!(wrapped.get(&token).get("alpha"), Some(&15));
    assert_eq!(wrapped.get(&token).get("gamma"), Some(&30));

    let handle = thread::spawn(move || {
        fragile::stack_token!(other_thread_token);

        assert!(!wrapped.is_valid());
        assert!(wrapped.try_get(&other_thread_token).is_err());
        assert!(wrapped.try_get_mut(&other_thread_token).is_err());

        wrapped
    });

    let mut wrapped = handle
        .join()
        .expect("worker thread should return the sticky wrapper without panicking");

    assert!(wrapped.is_valid());
    assert_eq!(wrapped.try_get(&token).unwrap().get("alpha"), Some(&15));
    assert_eq!(wrapped.try_get(&token).unwrap().get("gamma"), Some(&30));

    {
        let scores = wrapped
            .try_get_mut(&token)
            .expect("sticky value should be accessible again after returning to origin thread");
        let removed = scores.remove("beta");
        assert_eq!(removed, Some(20));
        scores.entry("gamma".to_string()).and_modify(|score| *score *= 2);
        scores.insert("delta".to_string(), 40);
    }

    let final_scores = wrapped.try_into_inner().ok().unwrap();

    assert_eq!(final_scores.len(), 3);
    assert_eq!(final_scores.get("alpha"), Some(&15));
    assert_eq!(final_scores.get("beta"), None);
    assert_eq!(final_scores.get("gamma"), Some(&60));
    assert_eq!(final_scores.get("delta"), Some(&40));
}