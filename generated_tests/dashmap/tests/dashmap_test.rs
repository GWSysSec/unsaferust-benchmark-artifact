use dashmap::DashMap;
use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hash, Hasher};

fn hash_with_build_hasher<S, T>(build_hasher: &S, value: &T) -> u64
where
    S: BuildHasher,
    T: Hash + ?Sized,
{
    let mut hasher = build_hasher.build_hasher();
    value.hash(&mut hasher);
    hasher.finish()
}

#[test]
fn shard_constructors_hashing_and_basic_workflow() {
    let map: DashMap<String, i32> = DashMap::with_shard_amount(4);

    assert!(map.is_empty());

    assert_eq!(map.insert("alpha".to_string(), 10), None);
    assert_eq!(map.insert("beta".to_string(), 20), None);
    assert_eq!(map.insert("gamma".to_string(), 30), None);
    assert_eq!(map.len(), 3);

    let alpha = String::from("alpha");
    let alpha_hash = map.hash_usize(&alpha);
    assert_eq!(alpha_hash, map.hash_usize(&alpha));

    assert_eq!(
        map.view("alpha", |key, value| (key.clone(), *value)),
        Some(("alpha".to_string(), 10))
    );

    assert_eq!(map.remove("beta"), Some(("beta".to_string(), 20)));
    assert_eq!(map.len(), 2);
}

#[test]
fn custom_hasher_with_shards_is_observable_through_public_api() {
    let hasher = RandomState::new();
    let map: DashMap<String, usize, RandomState> =
        DashMap::with_hasher_and_shard_amount(hasher, 8);

    assert!(map.is_empty());

    let key = String::from("consistent-key");
    let direct_hash = hash_with_build_hasher(map.hasher(), &key);
    assert_eq!(direct_hash, hash_with_build_hasher(map.hasher(), &key));

    map.insert(key.clone(), 1);
    map.alter(&key, |observed_key, old_value| {
        assert_eq!(observed_key, &key);
        old_value + 41
    });

    assert_eq!(map.view(&key, |_key, value| *value), Some(42));
    assert!(map.contains_key(&key));
}

#[test]
fn capacity_shard_constructor_remove_if_retain_and_alter_all_workflow() {
    let map: DashMap<&'static str, i32> = DashMap::with_capacity_and_shard_amount(32, 4);

    assert!(map.capacity() >= 32);

    assert_eq!(map.insert("keep-even", 2), None);
    assert_eq!(map.insert("remove-odd", 3), None);
    assert_eq!(map.insert("mutate-then-keep", 5), None);
    assert_eq!(map.insert("mutate-then-remove", 7), None);
    assert_eq!(map.len(), 4);

    let not_removed = map.remove_if("keep-even", |key, value| {
        assert_eq!(*key, "keep-even");
        *value % 2 != 0
    });
    assert_eq!(not_removed, None);
    assert!(map.contains_key("keep-even"));

    let removed = map.remove_if("remove-odd", |key, value| {
        assert_eq!(*key, "remove-odd");
        *value % 2 != 0
    });
    assert_eq!(removed, Some(("remove-odd", 3)));
    assert!(!map.contains_key("remove-odd"));

    let kept_after_mutation = map.remove_if_mut("mutate-then-keep", |key, value| {
        assert_eq!(*key, "mutate-then-keep");
        *value += 10;
        false
    });
    assert_eq!(kept_after_mutation, None);
    assert_eq!(map.view("mutate-then-keep", |_key, value| *value), Some(15));

    let removed_after_mutation = map.remove_if_mut("mutate-then-remove", |key, value| {
        assert_eq!(*key, "mutate-then-remove");
        *value += 1;
        *value == 8
    });
    assert_eq!(removed_after_mutation, Some(("mutate-then-remove", 8)));
    assert!(!map.contains_key("mutate-then-remove"));

    map.alter_all(|key, value| {
        if key.starts_with("keep") {
            value * 10
        } else {
            value + 1
        }
    });

    assert_eq!(map.view("keep-even", |_key, value| *value), Some(20));
    assert_eq!(map.view("mutate-then-keep", |_key, value| *value), Some(16));

    map.retain(|key, value| {
        if *key == "mutate-then-keep" {
            *value += 100;
            true
        } else {
            *value >= 20
        }
    });

    assert_eq!(map.len(), 2);
    assert_eq!(map.view("keep-even", |_key, value| *value), Some(20));
    assert_eq!(map.view("mutate-then-keep", |_key, value| *value), Some(116));
    assert_eq!(map.view("remove-odd", |_key, value| *value), None);
}

#[test]
fn try_entry_inserts_modifies_and_respects_locked_shard() {
    let map: DashMap<&'static str, Vec<i32>> = DashMap::with_shard_amount(4);

    {
        let mut inserted = map
            .try_entry("numbers")
            .expect("unlocked vacant entry should be available")
            .or_insert(Vec::new());
        inserted.push(1);
        inserted.push(2);
    }

    assert_eq!(
        map.view("numbers", |_key, value| value.clone()),
        Some(vec![1, 2])
    );

    {
        let mut existing = map
            .try_entry("numbers")
            .expect("unlocked occupied entry should be available")
            .and_modify(|values| values.push(3))
            .or_insert_with(|| vec![99]);
        existing.push(4);
    }

    assert_eq!(
        map.view("numbers", |key, value| {
            assert_eq!(*key, "numbers");
            value.iter().sum::<i32>()
        }),
        Some(10)
    );

    let guard = map.get("numbers").expect("numbers should be present");
    let locked_attempt = map.try_entry("numbers");
    assert!(locked_attempt.is_none());
    drop(guard);

    {
        let mut after_unlock = map
            .try_entry("numbers")
            .expect("entry should be available after read guard is dropped")
            .or_insert(Vec::new());
        after_unlock.push(5);
    }

    assert_eq!(
        map.view("numbers", |_key, value| value.clone()),
        Some(vec![1, 2, 3, 4, 5])
    );
}