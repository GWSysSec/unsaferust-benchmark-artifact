use std::collections::HashSet;

use rand::seq::index;

#[test]
fn random_iter_supports_long_lived_iterator_workflows() {
    let mut iter = rand::random_iter::<u8>();

    let samples: Vec<usize> = iter.by_ref().take(128).map(|byte| (byte % 4) as usize).collect();
    let next_value = iter.next();

    assert_eq!(samples.len(), 128);
    assert!(next_value.is_some(), "random_iter should be an unbounded iterator");
    assert!(samples.iter().all(|&bucket| bucket < 4));

    let mut bucket_counts = [0usize; 4];
    for bucket in samples {
        bucket_counts[bucket] += 1;
    }

    assert_eq!(bucket_counts.iter().sum::<usize>(), 128);
    assert_eq!(bucket_counts.len(), 4);
}

#[test]
#[allow(deprecated)]
fn thread_rng_can_drive_sampling_workflows() {
    let mut rng = rand::thread_rng();

    let selected = index::sample(&mut rng, 50, 10);
    assert_eq!(selected.len(), 10);
    assert!(!selected.is_empty());

    let selected_values: Vec<usize> = (0..selected.len()).map(|i| selected.index(i)).collect();

    assert!(selected_values.iter().all(|&value| value < 50));

    let unique_values: HashSet<usize> = selected_values.iter().copied().collect();
    assert_eq!(unique_values.len(), selected_values.len());

    let empty = index::sample(&mut rng, 50, 0);
    assert_eq!(empty.len(), 0);
    assert!(empty.is_empty());
}