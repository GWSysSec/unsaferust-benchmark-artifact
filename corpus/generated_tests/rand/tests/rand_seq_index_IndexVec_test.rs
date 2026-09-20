use std::collections::HashSet;

use rand::rngs::mock::StepRng;
use rand::seq::index;

#[test]
fn indexvec_index_matches_iteration_and_into_vec_after_sampling() {
    let mut rng = StepRng::new(0, 1);

    let sampled = index::sample(&mut rng, 32, 8);

    assert_eq!(sampled.len(), 8);
    assert!(!sampled.is_empty());

    let values_from_index: Vec<usize> = (0..sampled.len()).map(|position| sampled.index(position)).collect();
    let values_from_iter: Vec<usize> = sampled.iter().collect();

    assert_eq!(values_from_index, values_from_iter);
    assert!(values_from_index.iter().all(|&value| value < 32));

    let unique_values: HashSet<usize> = values_from_index.iter().copied().collect();
    assert_eq!(unique_values.len(), values_from_index.len());

    let values_from_into_vec = sampled.into_vec();
    assert_eq!(values_from_index, values_from_into_vec);
}

#[test]
fn indexvec_index_reads_positions_in_weighted_sampling_results() {
    let mut rng = StepRng::new(5, 3);
    let weights = [0_u32, 4, 0, 7, 1, 0, 9];

    let sampled = index::sample_weighted(&mut rng, weights.len(), |i| weights[i], 3)
        .expect("non-negative weights with a positive total should be valid");

    assert_eq!(sampled.len(), 3);
    assert!(!sampled.is_empty());

    let first = sampled.index(0);
    assert!(first < weights.len());
    assert!(weights[first] > 0);

    let indexed_values: Vec<usize> = (0..sampled.len()).map(|position| sampled.index(position)).collect();
    assert!(indexed_values.iter().all(|&value| value < weights.len()));
    assert!(indexed_values.iter().all(|&value| weights[value] > 0));

    let unique_values: HashSet<usize> = indexed_values.iter().copied().collect();
    assert_eq!(unique_values.len(), indexed_values.len());

    let iter_values: Vec<usize> = sampled.iter().collect();
    assert_eq!(indexed_values, iter_values);
}

#[test]
fn indexvec_empty_sample_has_no_indexable_positions() {
    let mut rng = StepRng::new(10, 1);

    let sampled = index::sample(&mut rng, 16, 0);

    assert_eq!(sampled.len(), 0);
    assert!(sampled.is_empty());
    assert_eq!(sampled.iter().count(), 0);

    let as_vec = sampled.into_vec();
    assert!(as_vec.is_empty());
}