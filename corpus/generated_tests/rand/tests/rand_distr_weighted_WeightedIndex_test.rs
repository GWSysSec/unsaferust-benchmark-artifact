use rand::distr::weighted::WeightedIndex;

#[test]
fn weighted_index_total_weight_tracks_initial_and_updated_weights() {
    let initial_weights = vec![1_u32, 2, 3, 4];
    let mut distribution =
        WeightedIndex::new(initial_weights).expect("positive weights should be valid");

    assert_eq!(distribution.total_weight(), 10);
    assert_eq!(distribution.weight(0), Some(1));
    assert_eq!(distribution.weight(3), Some(4));
    assert_eq!(distribution.weight(4), None);

    let collected_weights: Vec<u32> = distribution.weights().collect();
    assert_eq!(collected_weights, vec![1, 2, 3, 4]);

    let five = 5_u32;
    let zero = 0_u32;
    distribution
        .update_weights(&[(1, &five), (3, &zero)])
        .expect("updating existing weights with a positive total should succeed");

    assert_eq!(distribution.total_weight(), 9);
    assert_eq!(distribution.weight(0), Some(1));
    assert_eq!(distribution.weight(1), Some(5));
    assert_eq!(distribution.weight(2), Some(3));
    assert_eq!(distribution.weight(3), Some(0));

    let updated_weights: Vec<u32> = distribution.weights().collect();
    assert_eq!(updated_weights, vec![1, 5, 3, 0]);
}

#[test]
fn weighted_index_total_weight_handles_zero_weight_entries_and_rejects_empty_totals() {
    let distribution = WeightedIndex::new([0_u32, 7, 0]).expect("one positive weight is enough");

    assert_eq!(distribution.total_weight(), 7);
    assert_eq!(distribution.weight(0), Some(0));
    assert_eq!(distribution.weight(1), Some(7));
    assert_eq!(distribution.weight(2), Some(0));
    assert_eq!(distribution.weights().collect::<Vec<u32>>(), vec![0, 7, 0]);

    assert!(WeightedIndex::new([0_u32, 0, 0]).is_err());

    let empty_weights: [u32; 0] = [];
    assert!(WeightedIndex::new(empty_weights).is_err());
}