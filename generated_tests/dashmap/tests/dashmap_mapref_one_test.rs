use dashmap::DashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
struct Record {
    name: String,
    scores: Vec<u32>,
    active: bool,
}

#[test]
fn ref_map_projects_nested_value_and_keeps_map_usable_after_drop() {
    let map: DashMap<&'static str, Record> = DashMap::new();

    assert!(map.is_empty());
    assert_eq!(map.insert(
        "alice",
        Record {
            name: "Alice".to_string(),
            scores: vec![10, 20, 30],
            active: true,
        },
    ), None);
    assert_eq!(map.insert(
        "bob",
        Record {
            name: "Bob".to_string(),
            scores: vec![7, 8],
            active: false,
        },
    ), None);

    let scores = map
        .get("alice")
        .expect("alice should be present")
        .map(|record| &record.scores);

    assert_eq!(scores.len(), 3);
    assert_eq!(&scores[..], &[10, 20, 30]);
    assert_eq!(scores.iter().sum::<u32>(), 60);
    drop(scores);

    let replaced = map.insert(
        "alice",
        Record {
            name: "Alicia".to_string(),
            scores: vec![100],
            active: true,
        },
    );
    assert!(replaced.is_some());

    let updated_name = map
        .get("alice")
        .expect("updated alice should be present")
        .map(|record| &record.name);
    assert_eq!(updated_name.as_str(), "Alicia");
    assert_eq!(map.len(), 2);
}

#[test]
fn ref_try_map_success_and_failure_preserve_expected_access() {
    let map: DashMap<&'static str, Record> = DashMap::new();

    map.insert(
        "active",
        Record {
            name: "Active User".to_string(),
            scores: vec![42, 55],
            active: true,
        },
    );
    map.insert(
        "inactive",
        Record {
            name: "Inactive User".to_string(),
            scores: vec![1],
            active: false,
        },
    );

    let active_name = map
        .get("active")
        .expect("active record should exist")
        .try_map(|record| {
            if record.active {
                Some(&record.name)
            } else {
                None
            }
        })
        .expect("active record should map to its name");

    assert_eq!(active_name.as_str(), "Active User");
    assert_eq!(active_name.len(), "Active User".len());
    drop(active_name);

    let inactive_result = map
        .get("inactive")
        .expect("inactive record should exist")
        .try_map(|record| {
            if record.active {
                Some(&record.name)
            } else {
                None
            }
        });

    assert!(inactive_result.is_err());

    let original_ref = inactive_result.expect_err("inactive record should be returned on failed map");
    assert_eq!(original_ref.key(), &"inactive");
    assert_eq!(original_ref.value().name, "Inactive User");
    assert_eq!(original_ref.value().scores, vec![1]);
    assert!(!original_ref.value().active);
    drop(original_ref);

    assert!(map.contains_key("active"));
    assert!(map.contains_key("inactive"));
    assert_eq!(map.len(), 2);
}

#[test]
fn refmut_downgrade_commits_mutation_and_allows_read_projection() {
    let map: DashMap<&'static str, Record> = DashMap::new();

    map.insert(
        "carol",
        Record {
            name: "Carol".to_string(),
            scores: vec![3, 4],
            active: false,
        },
    );

    let mut writable = map.get_mut("carol").expect("carol should be present");
    writable.value_mut().scores.push(5);
    writable.value_mut().active = true;
    writable.value_mut().name.push_str(" Smith");

    let readable = writable.downgrade();

    assert_eq!(readable.key(), &"carol");
    assert_eq!(readable.value().name, "Carol Smith");
    assert_eq!(readable.value().scores, vec![3, 4, 5]);
    assert!(readable.value().active);

    let projected_scores = readable.map(|record| &record.scores);
    assert_eq!(projected_scores.as_slice(), &[3, 4, 5]);
    assert_eq!(projected_scores.iter().copied().max(), Some(5));
    drop(projected_scores);

    map.alter(&"carol", |_, mut record| {
        record.scores.push(6);
        record
    });

    let final_record = map.get("carol").expect("carol should still be present");
    assert_eq!(final_record.value().scores, vec![3, 4, 5, 6]);
    assert_eq!(map.len(), 1);
}