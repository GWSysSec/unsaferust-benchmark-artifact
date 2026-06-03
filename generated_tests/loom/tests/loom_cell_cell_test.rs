use loom::cell::Cell;

#[test]
fn cell_swap_replace_take_integer_workflow() {
    loom::model(|| {
        let left = Cell::new(7_i32);
        let right = Cell::new(-3_i32);
        let scratch = Cell::new(42_i32);

        assert_eq!(left.get(), 7);
        assert_eq!(right.get(), -3);
        assert_eq!(scratch.get(), 42);

        left.swap(&right);
        assert_eq!(left.get(), -3);
        assert_eq!(right.get(), 7);

        let old_left = left.replace(100);
        assert_eq!(old_left, -3);
        assert_eq!(left.get(), 100);

        let old_scratch = scratch.take();
        assert_eq!(old_scratch, 42);
        assert_eq!(scratch.get(), 0);

        scratch.swap(&right);
        assert_eq!(scratch.get(), 7);
        assert_eq!(right.get(), 0);

        let old_right = right.replace(left.take());
        assert_eq!(old_right, 0);
        assert_eq!(right.get(), 100);
        assert_eq!(left.get(), 0);
    });
}

#[test]
fn cell_replace_and_take_preserve_move_only_values() {
    loom::model(|| {
        let cell = Cell::new(String::from("alpha"));

        let old = cell.replace(String::from("beta"));
        assert_eq!(old, "alpha");

        let current = cell.replace(String::from("gamma"));
        assert_eq!(current, "beta");

        let taken = cell.take();
        assert_eq!(taken, "gamma");

        let default_after_take = cell.replace(String::from("delta"));
        assert_eq!(default_after_take, "");

        let final_value = cell.take();
        assert_eq!(final_value, "delta");

        let empty_again = cell.take();
        assert_eq!(empty_again, "");
    });
}

#[test]
fn cell_swap_move_only_values_between_multiple_cells() {
    loom::model(|| {
        let first = Cell::new(String::from("first"));
        let second = Cell::new(String::from("second"));
        let third = Cell::new(String::from("third"));

        first.swap(&second);

        let first_after_swap = first.replace(String::from("new-first"));
        let second_after_swap = second.replace(String::from("new-second"));
        assert_eq!(first_after_swap, "second");
        assert_eq!(second_after_swap, "first");

        second.swap(&third);

        let second_after_second_swap = second.take();
        let third_after_second_swap = third.replace(String::from("new-third"));
        assert_eq!(second_after_second_swap, "third");
        assert_eq!(third_after_second_swap, "new-second");

        let first_final = first.take();
        let second_final = second.take();
        let third_final = third.take();

        assert_eq!(first_final, "new-first");
        assert_eq!(second_final, "");
        assert_eq!(third_final, "new-third");
    });
}