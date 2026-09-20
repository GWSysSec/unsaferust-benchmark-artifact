use std::collections::HashSet;

use rand::rngs::mock::StepRng;
use rand::seq::index;

#[test]
fn weighted_sampling_returns_distinct_indices_with_positive_weights_only() {
    let mut rng = StepRng::new(0, 1);
    let weights = [0.0_f64, 1.0, 0.0, 2.0, 0.0, 3.0];

    let sampled = index::sample_weighted(&mut rng, weights.len(), |i| weights[i], 10)
        .expect("valid non-negative finite weights should sample successfully");

    assert_eq!(
        sampled.len(),
        3,
        "requesting more indices than positive weights should return only selectable indices"
    );
    assert!(!sampled.is_empty());

    let sampled_by_index_method: Vec<usize> = (0..sampled.len()).map(|i| sampled.index(i)).collect();
    let sampled_by_iter: Vec<usize> = sampled.iter().collect();

    assert_eq!(
        sampled_by_index_method, sampled_by_iter,
        "IndexVec::index should expose the same values as iteration"
    );
    assert!(sampled_by_index_method.iter().all(|&i| i < weights.len()));
    assert!(sampled_by_index_method.iter().all(|&i| weights[i] > 0.0));

    let unique: HashSet<usize> = sampled_by_index_method.iter().copied().collect();
    assert_eq!(
        unique.len(),
        sampled_by_index_method.len(),
        "sample_weighted should return distinct indices"
    );
    assert_eq!(unique, HashSet::from([1_usize, 3, 5]));
}

#[test]
fn weighted_sampling_handles_empty_requests_and_invalid_weights() {
    let mut rng = StepRng::new(123, 17);
    let weights = [4_u32, 0, 9, 2];

    let empty = index::sample_weighted(&mut rng, weights.len(), |i| weights[i], 0)
        .expect("requesting zero samples from valid weights should succeed");

    assert_eq!(empty.len(), 0);
    assert!(empty.is_empty());
    assert_eq!(empty.into_vec(), Vec::<usize>::new());

    let mut rng = StepRng::new(456, 31);
    let invalid_negative =
        index::sample_weighted(&mut rng, 3, |i| if i == 1 { -1.0_f64 } else { 1.0 }, 1);
    assert!(
        invalid_negative.is_err(),
        "negative weights must be rejected"
    );

    let mut rng = StepRng::new(789, 43);
    let invalid_nan =
        index::sample_weighted(&mut rng, 3, |i| if i == 2 { f64::NAN } else { 1.0 }, 1);
    assert!(invalid_nan.is_err(), "NaN weights must be rejected");
}

#[test]
fn index_method_supports_multi_step_sampling_workflow() {
    let mut rng = StepRng::new(10, 3);

    let sampled = index::sample(&mut rng, 32, 8);
    assert_eq!(sampled.len(), 8);
    assert!(!sampled.is_empty());

    let mut values = Vec::new();
    for position in 0..sampled.len() {
        let value = sampled.index(position);
        assert!(value < 32);
        values.push(value);
    }

    let iter_values: Vec<usize> = sampled.iter().collect();
    assert_eq!(values, iter_values);

    let unique: HashSet<usize> = values.iter().copied().collect();
    assert_eq!(unique.len(), values.len());

    let owned = sampled.into_vec();
    assert_eq!(owned.len(), 8);
    assert!(owned.iter().all(|&value| value < 32));
}