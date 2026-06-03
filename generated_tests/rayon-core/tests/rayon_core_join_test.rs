use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

fn parallel_sum(values: &[usize]) -> usize {
    if values.len() <= 4 {
        values.iter().sum()
    } else {
        let mid = values.len() / 2;
        let (left, right) = values.split_at(mid);
        let (left_sum, right_sum) = rayon_core::join(|| parallel_sum(left), || parallel_sum(right));
        left_sum + right_sum
    }
}

fn parallel_all_even(values: &[usize]) -> bool {
    if values.len() <= 3 {
        values.iter().all(|value| value % 2 == 0)
    } else {
        let mid = values.len() / 2;
        let (left, right) = values.split_at(mid);
        let (left_even, right_even) =
            rayon_core::join(|| parallel_all_even(left), || parallel_all_even(right));
        left_even && right_even
    }
}

#[test]
fn join_composes_recursive_work_without_losing_results() {
    let values: Vec<usize> = (1..=32).collect();

    let (sum, even_count) = rayon_core::join(
        || parallel_sum(&values),
        || values.iter().filter(|value| **value % 2 == 0).count(),
    );

    assert_eq!(sum, 528);
    assert_eq!(even_count, 16);

    let (all_even_for_even_values, all_even_for_mixed_values) = rayon_core::join(
        || parallel_all_even(&[2, 4, 6, 8, 10, 12, 14, 16]),
        || parallel_all_even(&values),
    );

    assert!(all_even_for_even_values);
    assert!(!all_even_for_mixed_values);
}

#[test]
fn join_context_reports_context_and_runs_inside_custom_pool() {
    let pool = rayon_core::ThreadPoolBuilder::new()
        .num_threads(2)
        .thread_name(|index| format!("join-context-worker-{index}"))
        .build()
        .expect("custom thread pool should build");

    let observed_workers = Arc::new(AtomicUsize::new(0));

    let observed_workers_a = Arc::clone(&observed_workers);
    let observed_workers_b = Arc::clone(&observed_workers);
    let observed_workers_c = Arc::clone(&observed_workers);
    let observed_workers_d = Arc::clone(&observed_workers);

    let (
        ((first_sum, first_index, first_migrated), (second_sum, second_index, second_migrated)),
        ((third_sum, third_index, third_migrated), (fourth_sum, fourth_index, fourth_migrated)),
    ) = pool.join(
        move || {
            rayon_core::join_context(
                |context| {
                    observed_workers_a.fetch_add(1, Ordering::SeqCst);
                    let index = rayon_core::current_thread_index();
                    let subtotal = (1..=10).sum::<usize>();
                    (subtotal, index, context.migrated())
                },
                |context| {
                    observed_workers_b.fetch_add(1, Ordering::SeqCst);
                    let index = rayon_core::current_thread_index();
                    let subtotal = (11..=20).sum::<usize>();
                    (subtotal, index, context.migrated())
                },
            )
        },
        move || {
            rayon_core::join_context(
                |context| {
                    observed_workers_c.fetch_add(1, Ordering::SeqCst);
                    let index = rayon_core::current_thread_index();
                    let subtotal = (21..=30).sum::<usize>();
                    (subtotal, index, context.migrated())
                },
                |context| {
                    observed_workers_d.fetch_add(1, Ordering::SeqCst);
                    let index = rayon_core::current_thread_index();
                    let subtotal = (31..=40).sum::<usize>();
                    (subtotal, index, context.migrated())
                },
            )
        },
    );

    assert_eq!(pool.current_num_threads(), 2);

    assert_eq!(first_sum, 55);
    assert_eq!(second_sum, 155);
    assert_eq!(third_sum, 255);
    assert_eq!(fourth_sum, 355);
    assert_eq!(
        first_sum + second_sum + third_sum + fourth_sum,
        (1..=40).sum::<usize>()
    );

    assert_eq!(observed_workers.load(Ordering::SeqCst), 4);

    for (label, index) in [
        ("first", first_index),
        ("second", second_index),
        ("third", third_index),
        ("fourth", fourth_index),
    ] {
        let index = index.unwrap_or_else(|| {
            panic!("{label} join_context closure should run on a Rayon worker thread")
        });

        assert!(
            index < pool.current_num_threads(),
            "{label} worker index should be within the pool"
        );
    }

    let migrated_flags = [
        first_migrated,
        second_migrated,
        third_migrated,
        fourth_migrated,
    ];

    assert_eq!(migrated_flags.len(), 4);
}