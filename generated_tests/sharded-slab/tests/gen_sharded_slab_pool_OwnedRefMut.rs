use sharded_slab::Pool;
use sharded_slab::pool::OwnedRef;
use sharded_slab::pool::OwnedRefMut;
use std::sync::Arc;

#[test]
fn test_owned_ref_mut_downgrade_basic() {
    let pool: Arc<Pool<String>> = Arc::new(Pool::new());


    let mut owned_mut: OwnedRefMut<String> = pool.clone().create_owned().unwrap();


    let key = owned_mut.key();


    owned_mut.push_str("hello");
    owned_mut.push_str(" world");


    assert_eq!(&*owned_mut as &String, "hello world");
    assert_eq!(owned_mut.len(), 11);


    let owned_ref: OwnedRef<String> = owned_mut.downgrade();


    assert_eq!(owned_ref.key(), key);


    assert_eq!(&*owned_ref as &String, "hello world");
    assert_eq!(owned_ref.len(), 11);


    let pool_ref = pool.get(key);
    assert!(pool_ref.is_some());
    assert_eq!(&*pool_ref.unwrap() as &String, "hello world");
}

#[test]
fn test_owned_ref_mut_downgrade_multiple_items() {
    let pool: Arc<Pool<String>> = Arc::new(Pool::new());


    let mut owned_mut1: OwnedRefMut<String> = pool.clone().create_owned().unwrap();
    let mut owned_mut2: OwnedRefMut<String> = pool.clone().create_owned().unwrap();
    let mut owned_mut3: OwnedRefMut<String> = pool.clone().create_owned().unwrap();

    let key1 = owned_mut1.key();
    let key2 = owned_mut2.key();
    let key3 = owned_mut3.key();


    assert_ne!(key1, key2);
    assert_ne!(key2, key3);
    assert_ne!(key1, key3);


    owned_mut1.push_str("first");
    owned_mut2.push_str("second");
    owned_mut3.push_str("third");


    let ref1: OwnedRef<String> = owned_mut1.downgrade();
    let ref2: OwnedRef<String> = owned_mut2.downgrade();
    let ref3: OwnedRef<String> = owned_mut3.downgrade();


    assert_eq!(ref1.key(), key1);
    assert_eq!(ref2.key(), key2);
    assert_eq!(ref3.key(), key3);


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


        let owned_ref: OwnedRef<String> = owned_mut.downgrade();
        assert_eq!(&*owned_ref as &String, "temporary data");
        assert_eq!(owned_ref.key(), key);

    }




    let mut new_owned: OwnedRefMut<String> = pool.clone().create_owned().unwrap();
    let new_key = new_owned.key();
    new_owned.push_str("new data");
    assert_eq!(&*new_owned as &String, "new data");


    let downgraded = new_owned.downgrade();
    assert_eq!(downgraded.key(), new_key);
    assert_eq!(&*downgraded as &String, "new data");
}

#[test]
fn test_owned_ref_mut_downgrade_with_complex_data() {
    let pool: Arc<Pool<String>> = Arc::new(Pool::new());

    let mut owned_mut: OwnedRefMut<String> = pool.clone().create_owned().unwrap();
    let key = owned_mut.key();


    for i in 0..100 {
        owned_mut.push_str(&format!("{},", i));
    }

    let expected_len = owned_mut.len();
    assert!(expected_len > 100);


    assert!(owned_mut.starts_with("0,1,2,3,"));
    assert!(owned_mut.ends_with("99,"));


    let owned_ref: OwnedRef<String> = owned_mut.downgrade();


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


    assert!(requires_static(&owned_mut));


    let owned_ref: OwnedRef<String> = owned_mut.downgrade();


    assert!(requires_static(&owned_ref));
    assert_eq!(owned_ref.key(), key);
    assert_eq!(&*owned_ref as &String, "static lifetime test");


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


    let mut owned_mut: OwnedRefMut<String> = pool.clone().create_owned().unwrap();
    let key = owned_mut.key();
    owned_mut.push_str("shared data");


    let owned_ref: OwnedRef<String> = owned_mut.downgrade();
    assert_eq!(&*owned_ref as &String, "shared data");


    let mut another_mut: OwnedRefMut<String> = pool.clone().create_owned().unwrap();
    let another_key = another_mut.key();
    another_mut.push_str("another item");

    assert_ne!(key, another_key);


    assert_eq!(&*owned_ref as &String, "shared data");
    assert_eq!(&*another_mut as &String, "another item");


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


    let owned_ref: OwnedRef<String> = owned_mut.downgrade();


    let handle = thread::spawn(move || {
        assert_eq!(&*owned_ref as &String, "thread-safe data");
        assert_eq!(owned_ref.key(), key);
        owned_ref.len()
    });

    let len = handle.join().unwrap();
    assert_eq!(len, 16);


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


    let owned_mut: OwnedRefMut<String> = pool.clone().create_owned().unwrap();
    let key = owned_mut.key();


    assert_eq!(&*owned_mut as &String, "");
    assert_eq!(owned_mut.len(), 0);
    assert!(owned_mut.is_empty());


    let owned_ref: OwnedRef<String> = owned_mut.downgrade();


    assert_eq!(owned_ref.key(), key);
    assert_eq!(&*owned_ref as &String, "");
    assert_eq!(owned_ref.len(), 0);
    assert!(owned_ref.is_empty());
}