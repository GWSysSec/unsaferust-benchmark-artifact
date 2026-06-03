use sharded_slab::Pool;
use sharded_slab::pool::OwnedRefMut;
use std::sync::Arc;

#[test]
fn create_returns_ref_mut_with_default_value() {
    let pool: Pool<String> = Pool::new();


    let mut item = pool.create().expect("create should return Some");
    let key = item.key();


    assert_eq!((*item).len(), 0);
    assert_eq!((*item).as_str(), "");
    assert!((*item).is_empty());


    (*item).push_str("hello");
    (*item).push_str(", world");
    assert_eq!((*item).as_str(), "hello, world");
    assert_eq!((*item).len(), 12);


    drop(item);


    let read_ref = pool.get(key).expect("key should be valid");
    assert_eq!((*read_ref).as_str(), "hello, world");
    assert_eq!(read_ref.key(), key);
}

#[test]
fn create_with_initializes_value_and_returns_key() {
    let pool: Pool<String> = Pool::new();


    assert!(pool.get(0).is_none() || pool.get(0).is_some());

    let key = pool
        .create_with(|s| {
            s.push_str("initialized");
        })
        .expect("create_with should succeed");


    let item = pool.get(key).expect("inserted key should be present");
    assert_eq!((*item).as_str(), "initialized");
    assert_eq!((*item).len(), 11);
    assert_eq!(item.key(), key);


    let key2 = pool
        .create_with(|s| {
            s.push_str("second");
        })
        .expect("create_with should succeed");
    assert_ne!(key, key2);

    let item2 = pool.get(key2).expect("second key should be present");
    assert_eq!((*item2).as_str(), "second");
    assert_ne!((*item).as_str(), (*item2).as_str());
}

#[test]
fn create_with_no_op_initializer_yields_default() {
    let pool: Pool<String> = Pool::new();

    let key = pool
        .create_with(|_| {

        })
        .expect("create_with should succeed");

    let item = pool.get(key).expect("key should be valid");
    assert_eq!((*item).as_str(), "");
    assert_eq!((*item).len(), 0);
    assert!((*item).is_empty());
    assert_eq!(item.key(), key);
}

#[test]
fn clear_removes_entry_and_returns_true_only_once() {
    let pool: Pool<String> = Pool::new();

    let key = pool
        .create_with(|s| s.push_str("to be cleared"))
        .expect("create_with should succeed");


    {
        let item = pool.get(key).expect("entry should exist before clear");
        assert_eq!((*item).as_str(), "to be cleared");
        assert_eq!((*item).len(), 13);
    }


    let cleared = pool.clear(key);
    assert!(cleared, "clear on a valid key should return true");


    let gone = pool.get(key);
    assert!(gone.is_none(), "entry should be absent after clear");


    let cleared_again = pool.clear(key);
    assert!(!cleared_again, "clear on a missing key should return false");
}

#[test]
fn clear_nonexistent_key_returns_false() {
    let pool: Pool<String> = Pool::new();


    let fake_key: usize = 0xDEAD_BEEF;
    let result = pool.clear(fake_key);
    assert!(!result, "clear on a never-allocated key should return false");


    let key = pool
        .create_with(|s| s.push_str("still works"))
        .expect("create_with should succeed after bogus clear");
    let item = pool.get(key).expect("entry should exist");
    assert_eq!((*item).as_str(), "still works");
    assert_eq!((*item).len(), 11);
    assert_eq!(item.key(), key);
    drop(item);
    assert!(pool.clear(key));
    assert!(pool.get(key).is_none());
}

#[test]
fn clear_allows_slot_reuse() {
    let pool: Pool<String> = Pool::new();

    let k1 = pool
        .create_with(|s| s.push_str("first"))
        .expect("first create_with");
    assert_eq!((*pool.get(k1).unwrap()).as_str(), "first");


    assert!(pool.clear(k1));
    assert!(pool.get(k1).is_none());

    let k2 = pool
        .create_with(|s| s.push_str("replaced"))
        .expect("second create_with");


    let item = pool.get(k2).expect("second entry should exist");
    assert_eq!((*item).as_str(), "replaced");
    assert_eq!((*item).len(), 8);
    assert!(!(*item).is_empty());


    drop(item);
    assert!(pool.clear(k2));
    assert!(pool.get(k2).is_none());
    assert!(!pool.clear(k2));
}

#[test]
fn create_owned_produces_static_mutable_handle() {
    let pool: Arc<Pool<String>> = Arc::new(Pool::new());


    let mut owned: OwnedRefMut<String> = pool
        .clone()
        .create_owned()
        .expect("create_owned should succeed");

    let key = owned.key();


    assert_eq!((*owned).as_str(), "");
    assert!((*owned).is_empty());
    assert_eq!((*owned).len(), 0);

    (*owned).push_str("owned mutable");
    assert_eq!((*owned).as_str(), "owned mutable");
    assert_eq!((*owned).len(), 13);


    fn requires_static<T: 'static>(_t: &T) {}
    requires_static(&owned);


    drop(owned);

    let read = pool.get(key).expect("entry should still be present");
    assert_eq!((*read).as_str(), "owned mutable");
    assert_eq!(read.key(), key);


    drop(read);
    assert!(pool.clear(key));
    assert!(pool.get(key).is_none());
}

#[test]
fn many_create_with_entries_are_independent() {
    let pool: Pool<String> = Pool::new();

    const N: usize = 64;
    let mut keys: Vec<usize> = Vec::with_capacity(N);

    for i in 0..N {
        let val = format!("entry-{}", i);
        let k = pool
            .create_with(|s| {
                s.push_str(&val);
            })
            .expect("create_with should succeed");
        keys.push(k);
    }


    let mut sorted = keys.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(sorted.len(), N, "all keys should be unique");


    for (i, &k) in keys.iter().enumerate() {
        let item = pool.get(k).expect("entry should exist");
        let expected = format!("entry-{}", i);
        assert_eq!((*item).as_str(), expected.as_str());
        assert_eq!(item.key(), k);
    }


    for &k in keys.iter().step_by(2) {
        assert!(pool.clear(k));
    }


    for (i, &k) in keys.iter().enumerate() {
        if i % 2 == 0 {
            assert!(pool.get(k).is_none(), "even index {} should be cleared", i);
        } else {
            let item = pool.get(k).expect("odd index should still exist");
            let expected = format!("entry-{}", i);
            assert_eq!((*item).as_str(), expected.as_str());
        }
    }


    for (i, &k) in keys.iter().enumerate() {
        if i % 2 != 0 {
            assert!(pool.clear(k));
        }
    }
}

#[test]
fn create_with_sequential_keys_and_clear_all() {
    let pool: Pool<String> = Pool::new();

    const N: usize = 32;
    let mut all_keys: Vec<usize> = Vec::with_capacity(N);

    for i in 0..N {
        let val = format!("val-{}", i);
        let k = pool
            .create_with(|s| {
                s.push_str(&val);
            })
            .expect("create_with should succeed");
        all_keys.push(k);
    }


    for (i, &k) in all_keys.iter().enumerate() {
        let item = pool.get(k).expect("entry should exist");
        assert_eq!((*item).as_str(), &format!("val-{}", i));
        assert_eq!(item.key(), k);
    }


    for k in &all_keys {
        assert!(pool.clear(*k));
    }
    for k in &all_keys {
        assert!(pool.get(*k).is_none());
    }
}