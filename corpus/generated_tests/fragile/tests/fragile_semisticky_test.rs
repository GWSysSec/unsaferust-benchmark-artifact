use std::collections::BTreeMap;

#[test]
fn semisticky_try_get_mut_updates_nested_state_with_stack_token() {
    fragile::stack_token!(token);

    let mut inventory = BTreeMap::new();
    inventory.insert("apples".to_string(), 3);
    inventory.insert("bananas".to_string(), 5);

    let mut wrapped = fragile::SemiSticky::new(inventory);

    assert!(wrapped.is_valid());
    assert_eq!(wrapped.try_get(&token).unwrap().get("apples"), Some(&3));
    assert_eq!(wrapped.try_get(&token).unwrap().get("bananas"), Some(&5));

    {
        let values = wrapped
            .try_get_mut(&token)
            .expect("semisticky value should be mutably accessible with a valid stack token");
        *values.get_mut("apples").unwrap() += 4;
        values.insert("oranges".to_string(), 2);
        assert_eq!(values.len(), 3);
    }

    assert_eq!(wrapped.get(&token).get("apples"), Some(&7));
    assert_eq!(wrapped.get(&token).get("oranges"), Some(&2));
    assert_eq!(wrapped.get(&token).get("bananas"), Some(&5));

    {
        let values = wrapped
            .try_get_mut(&token)
            .expect("try_get_mut should remain usable after the first mutable borrow ends");
        let removed = values.remove("bananas");
        assert_eq!(removed, Some(5));
        values.entry("oranges".to_string()).and_modify(|count| *count *= 3);
    }

    let final_inventory = wrapped.try_into_inner().ok().unwrap();
    assert_eq!(final_inventory.get("apples"), Some(&7));
    assert_eq!(final_inventory.get("oranges"), Some(&6));
    assert!(!final_inventory.contains_key("bananas"));
    assert_eq!(final_inventory.len(), 2);
}

#[test]
fn semisticky_try_get_mut_handles_empty_collection_and_repeated_mutations() {
    fragile::stack_token!(token);

    let mut wrapped = fragile::SemiSticky::new(Vec::<String>::new());

    assert!(wrapped.is_valid());
    assert!(wrapped.try_get(&token).unwrap().is_empty());

    {
        let values = wrapped
            .try_get_mut(&token)
            .expect("empty semisticky vector should be mutably accessible");
        values.push("first".to_string());
        values.push("second".to_string());
    }

    assert_eq!(wrapped.try_get(&token).unwrap().as_slice(), ["first", "second"]);

    {
        let values = wrapped
            .try_get_mut(&token)
            .expect("semisticky vector should allow a later mutable borrow");
        values[0].push_str("-edited");
        values.retain(|value| value.contains("edited"));
        values.push("third".to_string());
    }

    assert_eq!(
        wrapped.get(&token).as_slice(),
        ["first-edited".to_string(), "third".to_string()]
    );

    let inner = wrapped.into_inner();
    assert_eq!(inner, vec!["first-edited".to_string(), "third".to_string()]);
}