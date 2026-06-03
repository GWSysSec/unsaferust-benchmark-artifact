use std::collections::HashSet;
use std::panic::{catch_unwind, AssertUnwindSafe};

use rand::rngs::mock::StepRng;
use rand::seq::index;

#[test]
fn sample_weighted_selects_only_positive_weighted_distinct_indices() {
    let mut rng = StepRng::new(0, 1);
    let weights = [0.0_f64, 1.5, 0.0, 2.5, 0.0, 4.0];

    let sampled = index::sample_weighted(&mut rng, weights.len(), |i| weights[i], weights.len())
        .expect("finite non-negative weights should be accepted");

    assert_eq!(
        sampled.len(),
        3,
        "only indices with positive weights are selectable"
    );
    assert!(!sampled.is_empty());

    let via_index: Vec<usize> = (0..sampled.len()).map(|i| sampled.index(i)).collect();
    let via_iter: Vec<usize> = sampled.iter().collect();

    assert_eq!(
        via_index, via_iter,
        "IndexVec::index and IndexVec::iter should expose the same sampled indices"
    );
    assert!(via_index.iter().all(|&i| i < weights.len()));
    assert!(via_index.iter().all(|&i| weights[i] > 0.0));

    let unique: HashSet<usize> = via_index.iter().copied().collect();
    assert_eq!(
        unique.len(),
        via_index.len(),
        "weighted sampling without replacement should not duplicate indices"
    );
    assert_eq!(unique, HashSet::from([1_usize, 3, 5]));
}

#[test]
fn sample_weighted_respects_requested_amount_when_enough_positive_weights_exist() {
    let mut rng = StepRng::new(7, 11);
    let weights = [3_u32, 1, 4, 1, 5, 9, 2, 6];

    let sampled = index::sample_weighted(&mut rng, weights.len(), |i| weights[i], 4)
        .expect("positive integer weights should sample successfully");

    assert_eq!(sampled.len(), 4);
    assert!(!sampled.is_empty());

    let selected = sampled.into_vec();
    assert_eq!(selected.len(), 4);
    assert!(selected.iter().all(|&i| i < weights.len()));
    assert!(selected.iter().all(|&i| weights[i] > 0));

    let unique: HashSet<usize> = selected.iter().copied().collect();
    assert_eq!(
        unique.len(),
        selected.len(),
        "sample_weighted should return unique indices"
    );
}

#[test]
fn sample_weighted_handles_zero_amount_and_unselectable_weights() {
    let mut rng = StepRng::new(123, 456);
    let weights = [10.0_f64, 20.0, 30.0];

    let none_requested = index::sample_weighted(&mut rng, weights.len(), |i| weights[i], 0)
        .expect("requesting zero samples should be valid");

    assert_eq!(none_requested.len(), 0);
    assert!(none_requested.is_empty());
    assert_eq!(none_requested.into_vec(), Vec::<usize>::new());

    let all_zero = [0.0_f64, 0.0, 0.0, 0.0];
    let no_selectable = index::sample_weighted(&mut rng, all_zero.len(), |i| all_zero[i], 3)
        .expect("all-zero finite non-negative weights should produce no selected indices");

    assert_eq!(no_selectable.len(), 0);
    assert!(no_selectable.is_empty());
}

#[test]
fn sample_weighted_rejects_invalid_weights() {
    let negative_weights = [1.0_f64, -1.0, 2.0];
    let mut rng = StepRng::new(0, 1);
    let negative_result =
        index::sample_weighted(&mut rng, negative_weights.len(), |i| negative_weights[i], 2);
    assert!(
        negative_result.is_err(),
        "negative weights should be rejected"
    );

    let nan_weights = [1.0_f64, f64::NAN, 2.0];
    let nan_result = catch_unwind(AssertUnwindSafe(|| {
        let mut rng = StepRng::new(0, 1);
        index::sample_weighted(&mut rng, nan_weights.len(), |i| nan_weights[i], 2)
    }));
    assert!(
        !matches!(nan_result, Ok(Ok(_))),
        "NaN weights must not be accepted as a successful weighted sample"
    );

    let negative_infinite_weights = [1.0_f64, f64::NEG_INFINITY, 2.0];
    let mut rng = StepRng::new(0, 1);
    let negative_infinite_result = index::sample_weighted(
        &mut rng,
        negative_infinite_weights.len(),
        |i| negative_infinite_weights[i],
        2,
    );
    assert!(
        negative_infinite_result.is_err(),
        "negative infinite weights should be rejected"
    );
}