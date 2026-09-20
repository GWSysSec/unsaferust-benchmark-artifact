use loom::cell::Cell;

#[test]
fn cell_swap_replace_and_take_chain_copy_values() {
    loom::model(|| {
        let first = Cell::new(10_i32);
        let second = Cell::new(20_i32);
        let third = Cell::new(30_i32);

        first.swap(&second);
        assert_eq!(first.get(), 20);
        assert_eq!(second.get(), 10);
        assert_eq!(third.get(), 30);

        let old_second = second.replace(99);
        assert_eq!(old_second, 10);
        assert_eq!(second.get(), 99);

        let old_third = third.take();
        assert_eq!(old_third, 30);
        assert_eq!(third.get(), 0);

        third.swap(&first);
        assert_eq!(third.get(), 20);
        assert_eq!(first.get(), 0);
        assert_eq!(second.get(), 99);

        let old_first = first.replace(-7);
        assert_eq!(old_first, 0);
        assert_eq!(first.get(), -7);
    });
}

#[test]
fn cell_replace_and_take_move_only_values() {
    loom::model(|| {
        let cell = Cell::new(String::from("initial"));

        let old = cell.replace(String::from("updated"));
        assert_eq!(old, "initial");

        let current = cell.take();
        assert_eq!(current, "updated");

        let default_after_take = cell.replace(String::from("final"));
        assert_eq!(default_after_take, "");

        let final_value = cell.take();
        assert_eq!(final_value, "final");

        let default_again = cell.take();
        assert_eq!(default_again, "");
    });
}

#[test]
fn cell_take_leaves_default_for_nested_option_workflow() {
    loom::model(|| {
        let values: Cell<Option<Vec<i32>>> = Cell::new(Some(vec![1, 2, 3]));

        let first_take = values.take();
        assert_eq!(first_take, Some(vec![1, 2, 3]));

        let replaced_default = values.replace(Some(vec![4, 5]));
        assert_eq!(replaced_default, None);

        let second_take = values.take();
        assert_eq!(second_take, Some(vec![4, 5]));

        let second_default = values.replace(Some(Vec::new()));
        assert_eq!(second_default, None);

        let empty_vec = values.take();
        assert_eq!(empty_vec, Some(Vec::new()));

        let final_default = values.take();
        assert_eq!(final_default, None);
    });
}