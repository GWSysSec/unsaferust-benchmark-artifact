use rand::rngs::StdRng;
use rand::seq::index::sample_weighted;
use std::collections::HashSet;

fn make_rng() -> StdRng {
    <StdRng as rand::SeedableRng>::seed_from_u64(0xDEADBEEF)
}

#[test]
fn sample_weighted_basic_distinct_indices() {
    let mut rng = make_rng();
    let length = 10usize;
    let amount = 5usize;
    let result = sample_weighted(&mut rng, length, |i| (i + 1) as f64, amount);
    assert!(result.is_ok(), "expected Ok, got {:?}", result.err());
    let idx = result.unwrap();
    assert_eq!(idx.len(), amount);

    let collected: Vec<usize> = idx.into_iter().collect();
    assert_eq!(collected.len(), amount);
    let set: HashSet<usize> = collected.iter().copied().collect();
    assert_eq!(set.len(), amount, "indices must be distinct");
    for &i in &collected {
        assert!(i < length, "index {} out of bounds", i);
    }

    let sum: usize = collected.iter().sum();
    assert!(sum <= 35);
    assert!(sum >= 0 + 1 + 2 + 3 + 4);
}

#[test]
fn sample_weighted_amount_equals_length() {
    let mut rng = make_rng();
    let length = 6usize;
    let result = sample_weighted(&mut rng, length, |i| (i as f64) + 1.0, length);
    assert!(result.is_ok());
    let idx = result.unwrap();
    assert_eq!(idx.len(), length);
    let collected: Vec<usize> = idx.into_iter().collect();
    assert_eq!(collected.len(), length);
    let set: HashSet<usize> = collected.iter().copied().collect();
    assert_eq!(set.len(), length);
    let expected: HashSet<usize> = (0..length).collect();
    assert_eq!(set, expected);
    let total: usize = collected.iter().sum();
    assert_eq!(total, (0..length).sum::<usize>());
}

#[test]
fn sample_weighted_zero_amount() {
    let mut rng = make_rng();
    let result = sample_weighted(&mut rng, 10, |_| 1.0_f64, 0);
    assert!(result.is_ok());
    let idx = result.unwrap();
    assert_eq!(idx.len(), 0);
    let v: Vec<usize> = idx.into_iter().collect();
    assert_eq!(v.len(), 0);
    assert!(v.is_empty());

    let result2 = sample_weighted(&mut rng, 10, |_| 1.0_f64, 0);
    assert!(result2.is_ok());
    assert_eq!(result2.unwrap().len(), 0);
}

#[test]
fn sample_weighted_amount_exceeds_length_errors() {
    let mut rng = make_rng();
    let result = sample_weighted(&mut rng, 3, |i| (i + 1) as f64, 5);


    assert!(result.is_ok(), "expected Ok when amount > length (clamped)");
    let idx = result.unwrap();
    assert_eq!(idx.len(), 3, "should return all available indices when amount > length");
    let collected: Vec<usize> = idx.into_iter().collect();
    assert_eq!(collected.len(), 3);
    let set: HashSet<usize> = collected.iter().copied().collect();
    assert_eq!(set.len(), 3, "indices must be distinct");
    let expected: HashSet<usize> = (0..3).collect();
    assert_eq!(set, expected, "all indices 0..3 must be present");

    let ok = sample_weighted(&mut rng, 5, |i| (i + 1) as f64, 3);
    assert!(ok.is_ok());
    assert_eq!(ok.as_ref().unwrap().len(), 3);
    let v: Vec<usize> = ok.unwrap().into_iter().collect();
    assert_eq!(v.len(), 3);
    let set: HashSet<usize> = v.iter().copied().collect();
    assert_eq!(set.len(), 3);
    for &i in &v {
        assert!(i < 5);
    }
}

#[test]
fn sample_weighted_negative_weight_errors() {
    let mut rng = make_rng();
    let result = sample_weighted(&mut rng, 5, |i| if i == 2 { -1.0 } else { 1.0 }, 2);
    assert!(result.is_err(), "negative weight should error");
    let result_nan = sample_weighted(&mut rng, 5, |i| if i == 0 { f64::NAN } else { 1.0 }, 2);
    assert!(result_nan.is_err(), "NaN weight should error");

    let ok = sample_weighted(&mut rng, 5, |_| 2.0_f64, 2);
    assert!(ok.is_ok());
    assert_eq!(ok.as_ref().unwrap().len(), 2);
    let v: Vec<usize> = ok.unwrap().into_iter().collect();
    assert_eq!(v.len(), 2);
    assert_ne!(v[0], v[1]);
}

#[test]
fn sample_weighted_skewed_weights_bias() {
    let mut rng = make_rng();

    let mut count_zero = 0usize;
    let trials = 200;
    for _ in 0..trials {
        let res = sample_weighted(&mut rng, 5, |i| if i == 0 { 1000.0 } else { 0.001 }, 1);
        assert!(res.is_ok());
        let idx = res.unwrap();
        assert_eq!(idx.len(), 1);
        let v: Vec<usize> = idx.into_iter().collect();
        assert_eq!(v.len(), 1);
        assert!(v[0] < 5);
        if v[0] == 0 {
            count_zero += 1;
        }
    }
    assert!(count_zero > trials * 9 / 10, "expected heavy bias to index 0, got {}/{}", count_zero, trials);
}