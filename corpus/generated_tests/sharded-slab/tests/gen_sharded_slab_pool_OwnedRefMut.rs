use sharded_slab::Pool;
use sharded_slab::pool::OwnedRef;
use sharded_slab::pool::OwnedRefMut;
use std::sync::Arc;

#[test]
fn test_owned_ref_mut_downgrade_basic() {
    let pool: Arc<Pool<String>> = Arc::new(Pool::new());

    // Create an owned mutable reference
    let mut owned_mut: OwnedRefMut<String> = pool.clone().create_owned().unwrap();

    // Get the key before downgrade
    let key = owned_mut.key();

    // Write data through the mutable reference
    owned_mut.push_str("hello");
    owned_mut.push_str(" world");

    // Verify the mutable reference has the correct content
    assert_eq!(&*owned_mut as &String, "hello world");
    assert_eq!(owned_mut.len(), 11);

    // Downgrade to an immutable OwnedRef
    let owned_ref: OwnedRef<String> = owned_mut.downgrade();

    // Verify the key is preserved after downgrade
    assert_eq!(owned_ref.key(), key);

    // Verify the content is preserved after downgrade
    assert_eq!(&*owned_ref as &String, "hello world");
    assert_eq!(owned_ref.len(), 11);

    // Verify we can still look up the item via the pool
    let pool_ref = pool.get(key);
    assert!(pool_ref.is_some());
    assert_eq!(&*pool_ref.unwrap() as &String, "hello world");
}

#[test]
fn test_owned_ref_mut_downgrade_multiple_items() {
    let pool: Arc<Pool<String>> = Arc::new(Pool::new());

    // Create multiple items
    let mut owned_mut1: OwnedRefMut<String> = pool.clone().create_owned().unwrap();
    let mut owned_mut2: OwnedRefMut<String> = pool.clone().create_owned().unwrap();
    let mut owned_mut3: OwnedRefMut<String> = pool.clone().create_owned().unwrap();

    let key1 = owned_mut1.key();
    let key2 = owned_mut2.key();
    let key3 = owned_mut3.key();

    // Keys should be distinct
    assert_ne!(key1, key2);
    assert_ne!(key2, key3);
    assert_ne!(key1, key3);

    // Write different data to each
    owned_mut1.push_str("first");
    owned_mut2.push_str("second");
    owned_mut3.push_str("third");

    // Downgrade all of them
    let ref1: OwnedRef<String> = owned_mut1.downgrade();
    let ref2: OwnedRef<String> = owned_mut2.downgrade();
    let ref3: OwnedRef<String> = owned_mut3.downgrade();

    // Verify keys are preserved
    assert_eq!(ref1.key(), key1);
    assert_eq!(ref2.key(), key2);
    assert_eq!(ref3.key(), key3);

    // Verify content is preserved and distinct
    assert_eq!(&*ref1 as &String, "first");
    assert_eq!(&*ref2 as &String, "second");
    assert_eq!(&*ref3 as &String, "third");
}

#[test]
fn test_owned_ref_mut_downgrade_then_drop() {
    let pool: Arc<Pool<String>> = Arc::new(Pool::new());

    let key;
    {
        let mut owned_mut: OwnedRefMut<String> = pool.clone().create_owned().unwrap();
        key = owned_mut.key();
        owned_mut.push_str("temporary data");

        // Downgrade and then drop the resulting OwnedRef
        let owned_ref: OwnedRef<String> = owned_mut.downgrade();
        assert_eq!(&*owned_ref as &String, "temporary data");
        assert_eq!(owned_ref.key(), key);
        // owned_ref drops here
    }

    // After the OwnedRef is dropped, the slot should be cleared
    // The pool may or may not return the item depending on implementation
    // but we can verify the pool itself is still functional
    let mut new_owned: OwnedRefMut<String> = pool.clone().create_owned().unwrap();
    let new_key = new_owned.key();
    new_owned.push_str("new data");
    assert_eq!(&*new_owned as &String, "new data");

    // The pool should still be operational
    let downgraded = new_owned.downgrade();
    assert_eq!(downgraded.key(), new_key);
    assert_eq!(&*downgraded as &String, "new data");
}

#[test]
fn test_owned_ref_mut_downgrade_with_complex_data() {
    let pool: Arc<Pool<String>> = Arc::new(Pool::new());

    let mut owned_mut: OwnedRefMut<String> = pool.clone().create_owned().unwrap();
    let key = owned_mut.key();

    // Build up a complex string
    for i in 0..100 {
        owned_mut.push_str(&format!("{},", i));
    }

    let expected_len = owned_mut.len();
    assert!(expected_len > 100);

    // Verify content before downgrade
    assert!(owned_mut.starts_with("0,1,2,3,"));
    assert!(owned_mut.ends_with("99,"));

    // Downgrade
    let owned_ref: OwnedRef<String> = owned_mut.downgrade();

    // Verify content after downgrade
    assert_eq!(owned_ref.key(), key);
    assert_eq!(owned_ref.len(), expected_len);
    assert!(owned_ref.starts_with("0,1,2,3,"));
    assert!(owned_ref.ends_with("99,"));
    assert!(owned_ref.contains("50,"));
}

#[test]
fn test_owned_ref_mut_downgrade_static_lifetime() {
    fn requires_static<T: 'static>(t: &T) -> bool {
        let _ = t;
        true
    }

    let pool: Arc<Pool<String>> = Arc::new(Pool::new());

    let mut owned_mut: OwnedRefMut<String> = pool.clone().create_owned().unwrap();
    owned_mut.push_str("static lifetime test");
    let key = owned_mut.key();

    // The OwnedRefMut should be 'static
    assert!(requires_static(&owned_mut));

    // Downgrade to OwnedRef
    let owned_ref: OwnedRef<String> = owned_mut.downgrade();

    // The OwnedRef should also be 'static
    assert!(requires_static(&owned_ref));
    assert_eq!(owned_ref.key(), key);
    assert_eq!(&*owned_ref as &String, "static lifetime test");

    // Can be moved into a struct with 'static requirement
    struct StaticHolder {
        data: OwnedRef<String>,
    }

    let holder = StaticHolder { data: owned_ref };
    assert!(requires_static(&holder));
    assert_eq!(&*holder.data as &String, "static lifetime test");
}

#[test]
fn test_owned_ref_mut_downgrade_concurrent_access() {
    let pool: Arc<Pool<String>> = Arc::new(Pool::new());

    // Create and populate an item
    let mut owned_mut: OwnedRefMut<String> = pool.clone().create_owned().unwrap();
    let key = owned_mut.key();
    owned_mut.push_str("shared data");

    // Downgrade to immutable
    let owned_ref: OwnedRef<String> = owned_mut.downgrade();
    assert_eq!(&*owned_ref as &String, "shared data");

    // While holding the OwnedRef, we can still create new items in the pool
    let mut another_mut: OwnedRefMut<String> = pool.clone().create_owned().unwrap();
    let another_key = another_mut.key();
    another_mut.push_str("another item");

    assert_ne!(key, another_key);

    // Both references should be valid simultaneously
    assert_eq!(&*owned_ref as &String, "shared data");
    assert_eq!(&*another_mut as &String, "another item");

    // Downgrade the second one too
    let another_ref: OwnedRef<String> = another_mut.downgrade();
    assert_eq!(&*another_ref as &String, "another item");
    assert_eq!(&*owned_ref as &String, "shared data");
    assert_eq!(owned_ref.key(), key);
    assert_eq!(another_ref.key(), another_key);
}

#[test]
fn test_owned_ref_mut_downgrade_with_threads() {
    use std::thread;

    let pool: Arc<Pool<String>> = Arc::new(Pool::new());

    let mut owned_mut: OwnedRefMut<String> = pool.clone().create_owned().unwrap();
    let key = owned_mut.key();
    owned_mut.push_str("thread-safe data");

    // Downgrade to OwnedRef which is Send + 'static
    let owned_ref: OwnedRef<String> = owned_mut.downgrade();

    // Move the OwnedRef to another thread
    let handle = thread::spawn(move || {
        assert_eq!(&*owned_ref as &String, "thread-safe data");
        assert_eq!(owned_ref.key(), key);
        owned_ref.len()
    });

    let len = handle.join().unwrap();
    assert_eq!(len, 16);

    // The pool is still usable after the OwnedRef is consumed in another thread
    let pool2 = pool.clone();
    let mut new_item: OwnedRefMut<String> = pool2.create_owned().unwrap();
    new_item.push_str("after thread");
    let new_key = new_item.key();
    let new_ref = new_item.downgrade();
    assert_eq!(&*new_ref as &String, "after thread");
    assert_eq!(new_ref.key(), new_key);
}

#[test]
fn test_owned_ref_mut_downgrade_empty_string() {
    let pool: Arc<Pool<String>> = Arc::new(Pool::new());

    // Create without writing anything (default empty string)
    let owned_mut: OwnedRefMut<String> = pool.clone().create_owned().unwrap();
    let key = owned_mut.key();

    // Verify it's empty before downgrade
    assert_eq!(&*owned_mut as &String, "");
    assert_eq!(owned_mut.len(), 0);
    assert!(owned_mut.is_empty());

    // Downgrade empty item
    let owned_ref: OwnedRef<String> = owned_mut.downgrade();

    // Verify empty state is preserved
    assert_eq!(owned_ref.key(), key);
    assert_eq!(&*owned_ref as &String, "");
    assert_eq!(owned_ref.len(), 0);
    assert!(owned_ref.is_empty());
}