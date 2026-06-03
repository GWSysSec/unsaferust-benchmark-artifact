use rand::distr::slice::Choose;
use rand::distr::Distribution;
use rand::rngs::mock::StepRng;

#[test]
fn test_choose_num_choices_basic() {
    let data = ["a", "b", "c", "d", "e"];
    let choose = Choose::new(&data).expect("non-empty");
    assert_eq!(choose.num_choices().get(), 5);

    let single = [42i32];
    let c1 = Choose::new(&single).expect("non-empty");
    assert_eq!(c1.num_choices().get(), 1);

    let pair = [1u8, 2u8];
    let c2 = Choose::new(&pair).expect("non-empty");
    assert_eq!(c2.num_choices().get(), 2);

    let empty: [i32; 0] = [];
    assert!(Choose::new(&empty).is_err());


    let big: Vec<u32> = (0..100).collect();
    let cb = Choose::new(&big[..]).expect("non-empty");
    assert_eq!(cb.num_choices().get(), 100);
    assert_ne!(cb.num_choices().get(), 99);
}

#[test]
fn test_choose_sampling_and_num_choices() {
    let data = [10u32, 20, 30, 40];
    let choose = Choose::new(&data).expect("non-empty");
    let n = choose.num_choices();
    assert_eq!(n.get(), 4);

    let mut rng = StepRng::new(0, 1 << 32);
    let mut seen = [false; 4];
    for _ in 0..200 {
        let &v = choose.sample(&mut rng);
        let idx = match v {
            10 => 0,
            20 => 1,
            30 => 2,
            40 => 3,
            _ => panic!("unexpected value {}", v),
        };
        seen[idx] = true;
    }
    let count_seen = seen.iter().filter(|x| **x).count();
    assert!(count_seen >= 1);
    assert_eq!(seen.len(), choose.num_choices().get());


    let cloned = choose.clone();
    assert_eq!(cloned.num_choices().get(), choose.num_choices().get());
    assert_eq!(cloned.num_choices().get(), 4);
}

#[test]
fn test_choose_num_choices_various_types() {
    let strings = vec![String::from("x"), String::from("y"), String::from("z")];
    let c = Choose::new(&strings).expect("non-empty");
    assert_eq!(c.num_choices().get(), 3);

    let chars = ['a', 'b'];
    let cc = Choose::new(&chars).expect("non-empty");
    assert_eq!(cc.num_choices().get(), 2);
    assert!(cc.num_choices().get() > 0);

    let floats = [1.0f64, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0];
    let cf = Choose::new(&floats).expect("non-empty");
    assert_eq!(cf.num_choices().get(), 7);
    assert_ne!(cf.num_choices().get(), 0);

    let mut rng = StepRng::new(5, 7);
    let s = cf.sample(&mut rng);
    assert!(*s >= 1.0 && *s <= 7.0);
}