use dashmap::DashMap;

#[derive(Debug, PartialEq, Eq)]
struct NonCloneCounter {
    total: i32,
}

fn map_in_place_2<C, T, F>((context, value): (C, &mut T), f: F)
where
    F: FnOnce(C, T) -> T,
{




    unsafe {
        let old_value = std::ptr::read(value);
        let new_value = f(context, old_value);
        std::ptr::write(value, new_value);
    }
}

#[test]
fn map_in_place_2_replaces_plain_value_using_context_and_old_value() {
    let mut value = String::from("map");

    map_in_place_2(("dash", &mut value), |prefix, old| {
        assert_eq!(old, "map");
        format!("{prefix}-{old}")
    });

    assert_eq!(value, "dash-map");

    map_in_place_2((3usize, &mut value), |repeat_count, old| {
        assert_eq!(old, "dash-map");
        old.repeat(repeat_count)
    });

    assert_eq!(value, "dash-mapdash-mapdash-map");
}

#[test]
fn map_in_place_2_works_with_non_clone_values() {
    let mut counter = NonCloneCounter { total: 10 };

    map_in_place_2((5, &mut counter), |increment, old_counter| NonCloneCounter {
        total: old_counter.total + increment,
    });

    assert_eq!(counter, NonCloneCounter { total: 15 });

    map_in_place_2((-20, &mut counter), |increment, old_counter| NonCloneCounter {
        total: old_counter.total + increment,
    });

    assert_eq!(counter.total, -5);
}

#[test]
fn map_in_place_2_can_update_value_held_inside_dashmap_reference() {
    let map: DashMap<&'static str, Vec<i32>> = DashMap::new();

    assert!(map.is_empty());
    assert_eq!(map.insert("numbers", vec![1, 2]), None);
    assert_eq!(map.len(), 1);

    {
        let mut entry = map.get_mut("numbers").expect("inserted key should exist");
        map_in_place_2((3, entry.value_mut()), |next_number, mut old_numbers| {
            assert_eq!(old_numbers, vec![1, 2]);
            old_numbers.push(next_number);
            old_numbers
        });
    }

    {
        let numbers = map.get("numbers").expect("updated key should still exist");
        assert_eq!(&*numbers, &vec![1, 2, 3]);
    }

    {
        let mut entry = map.get_mut("numbers").expect("updated key should be mutable");
        map_in_place_2((vec![4, 5], entry.value_mut()), |additional, mut old_numbers| {
            old_numbers.extend(additional);
            old_numbers.retain(|number| number % 2 == 1);
            old_numbers
        });
    }

    let final_numbers = map
        .get("numbers")
        .map(|value| value.clone())
        .expect("get should find numbers key");

    assert_eq!(final_numbers, vec![1, 3, 5]);
    assert!(map.contains_key("numbers"));
    assert_eq!(map.remove("numbers"), Some(("numbers", vec![1, 3, 5])));
    assert!(map.is_empty());
}