use fragile::{Fragile, SemiSticky, Sticky};
use std::rc::Rc;

#[test]
fn stack_token_allows_semisticky_multi_step_access() {
    fragile::stack_token!(token);
    let mut values = SemiSticky::new(vec![1, 2, 3]);

    assert!(values.is_valid());
    assert_eq!(values.get(&token).as_slice(), &[1, 2, 3]);
    assert_eq!(values.try_get(&token).unwrap().len(), 3);

    values.get_mut(&token).push(4);
    values.try_get_mut(&token).unwrap().push(5);

    assert_eq!(values.get(&token).as_slice(), &[1, 2, 3, 4, 5]);

    let inner = values.try_into_inner().ok().unwrap();
    assert_eq!(inner, vec![1, 2, 3, 4, 5]);
}

#[test]
fn stack_token_allows_sticky_mutation_and_extraction() {
    fragile::stack_token!(token);
    let mut text = Sticky::new(String::from("fragile"));

    assert!(text.is_valid());
    assert_eq!(text.get(&token), "fragile");

    text.get_mut(&token).push_str(" crate");
    assert_eq!(text.try_get(&token).unwrap(), "fragile crate");

    text.try_get_mut(&token).unwrap().push('!');
    assert_eq!(text.get(&token), "fragile crate!");

    let extracted = text.into_inner();
    assert_eq!(extracted, "fragile crate!");
}

#[test]
fn independent_stack_tokens_work_for_same_thread_workflow() {
    fragile::stack_token!(first_token);
    fragile::stack_token!(second_token);

    let mut wrapped = SemiSticky::new(Rc::new(String::from("initial")));

    assert_eq!(Rc::strong_count(wrapped.get(&first_token)), 1);
    assert_eq!(wrapped.get(&first_token).as_str(), "initial");
    assert_eq!(wrapped.get(&second_token).as_str(), "initial");

    {
        let replacement = wrapped.get_mut(&second_token);
        *replacement = Rc::new(String::from("updated"));
    }

    assert_eq!(wrapped.try_get(&first_token).unwrap().as_str(), "updated");
    assert_eq!(Rc::strong_count(wrapped.get(&second_token)), 1);

    let extracted = wrapped.into_inner();
    assert_eq!(extracted.as_str(), "updated");
}

#[test]
fn fragile_and_stack_token_wrappers_can_be_used_together() {
    fragile::stack_token!(token);

    let mut fragile_counter = Fragile::new(10usize);
    let mut sticky_log = Sticky::new(Vec::<String>::new());

    assert!(fragile_counter.is_valid());
    assert!(sticky_log.is_valid());
    assert_eq!(*fragile_counter.try_get().unwrap(), 10);

    *fragile_counter.get_mut() += 5;
    sticky_log
        .get_mut(&token)
        .push(format!("counter={}", fragile_counter.get()));

    *fragile_counter.try_get_mut().unwrap() *= 2;
    sticky_log
        .try_get_mut(&token)
        .unwrap()
        .push(format!("counter={}", fragile_counter.get()));

    assert_eq!(*fragile_counter.get(), 30);
    assert_eq!(
        sticky_log.get(&token).as_slice(),
        &["counter=15".to_string(), "counter=30".to_string()]
    );

    assert_eq!(fragile_counter.into_inner(), 30);
    assert_eq!(
        sticky_log.try_into_inner().ok().unwrap(),
        vec!["counter=15".to_string(), "counter=30".to_string()]
    );
}