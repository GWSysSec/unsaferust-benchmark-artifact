extern crate arraydeque;
use arraydeque::ArrayDeque;

#[test]
fn test_front_back_inspection() {
    let mut deque: ArrayDeque<[i32; 9]> = ArrayDeque::new();

    // Pre-state: empty deque returns None
    assert_eq!(deque.front(), None);
    assert_eq!(deque.back(), None);
    assert_eq!(deque.len(), 0);

    // Single element: front and back point to same item
    deque.push_back(100);
    assert_eq!(deque.front(), Some(&100));
    assert_eq!(deque.back(), Some(&100));
    assert_eq!(deque.len(), 1);

    // Multiple back pushes
    deque.push_back(200);
    deque.push_back(300);
    assert_eq!(deque.front(), Some(&100));
    assert_eq!(deque.back(), Some(&300));

    // Front push shifts the front
    deque.push_front(50);
    assert_eq!(deque.front(), Some(&50));
    assert_eq!(deque.back(), Some(&300));
    assert_eq!(deque.len(), 4);

    // Pop and re-check
    let popped_front = deque.pop_front();
    assert_eq!(popped_front, Some(50));
    assert_eq!(deque.front(), Some(&100));
    assert_eq!(deque.back(), Some(&300));

    let popped_back = deque.pop_back();
    assert_eq!(popped_back, Some(300));
    assert_eq!(deque.front(), Some(&100));
    assert_eq!(deque.back(), Some(&200));
    assert_eq!(deque.len(), 2);
}

#[test]
fn test_front_mut_and_back_mut_modification() {
    let mut deque: ArrayDeque<[i64; 9]> = ArrayDeque::new();

    // Pre-state: mutable accessors return None on empty
    assert!(deque.front_mut().is_none());
    assert!(deque.back_mut().is_none());

    deque.push_back(1);
    deque.push_back(2);
    deque.push_back(3);
    deque.push_back(4);

    // Pre-mutation snapshot
    assert_eq!(deque.front(), Some(&1));
    assert_eq!(deque.back(), Some(&4));
    assert_eq!(deque.len(), 4);

    // Mutate via front_mut and back_mut
    {
        let f = deque.front_mut().expect("front exists");
        *f = 1000;
    }
    {
        let b = deque.back_mut().expect("back exists");
        *b = 4000;
    }

    // Post-mutation: only ends changed
    assert_eq!(deque.front(), Some(&1000));
    assert_eq!(deque.back(), Some(&4000));
    assert_eq!(deque.len(), 4);

    // Middle elements remain untouched (verified via get_mut)
    assert_eq!(deque.get_mut(1).map(|x| *x), Some(2));
    assert_eq!(deque.get_mut(2).map(|x| *x), Some(3));

    // Increment using back_mut several times
    for _ in 0..3 {
        if let Some(b) = deque.back_mut() {
            *b += 1;
        }
    }
    assert_eq!(deque.back(), Some(&4003));
    assert_eq!(deque.front(), Some(&1000));
}

#[test]
fn test_get_mut_with_strings() {
    let mut deque: ArrayDeque<[String; 7]> = ArrayDeque::new();

    // Empty deque: out-of-range indices return None
    assert!(deque.get_mut(0).is_none());
    assert!(deque.get_mut(1).is_none());

    deque.push_back(String::from("alpha"));
    deque.push_back(String::from("beta"));
    deque.push_back(String::from("gamma"));

    // Pre-mutation reads
    assert_eq!(deque.get_mut(0).map(|s| s.clone()), Some(String::from("alpha")));
    assert_eq!(deque.get_mut(1).map(|s| s.clone()), Some(String::from("beta")));
    assert_eq!(deque.get_mut(2).map(|s| s.clone()), Some(String::from("gamma")));

    // Mutate the middle element through get_mut
    {
        let s = deque.get_mut(1).expect("index 1 valid");
        s.push_str("_modified");
    }

    // Post-mutation verification
    assert_eq!(deque.get_mut(0).map(|s| s.clone()), Some(String::from("alpha")));
    assert_eq!(deque.get_mut(1).map(|s| s.clone()), Some(String::from("beta_modified")));
    assert_eq!(deque.get_mut(2).map(|s| s.clone()), Some(String::from("gamma")));

    // Out-of-range
    assert!(deque.get_mut(3).is_none());
    assert!(deque.get_mut(100).is_none());
    assert!(deque.get_mut(usize::MAX).is_none());

    // front/back still observe the unchanged ends
    assert_eq!(deque.front(), Some(&String::from("alpha")));
    assert_eq!(deque.back(), Some(&String::from("gamma")));
}

#[test]
fn test_contains_workflow() {
    let mut deque: ArrayDeque<[i32; 11]> = ArrayDeque::new();

    // Empty: nothing is contained
    assert_eq!(deque.contains(&0), false);
    assert_eq!(deque.contains(&5), false);

    for i in 1..=5 {
        deque.push_back(i * 10);
    }
    // Now: [10, 20, 30, 40, 50]

    // Existing values report true
    assert_eq!(deque.contains(&10), true);
    assert_eq!(deque.contains(&30), true);
    assert_eq!(deque.contains(&50), true);

    // Non-existing values report false
    assert_eq!(deque.contains(&15), false);
    assert_eq!(deque.contains(&0), false);
    assert_eq!(deque.contains(&-10), false);
    assert_eq!(deque.contains(&999), false);

    // After popping front, that value disappears
    let removed = deque.pop_front();
    assert_eq!(removed, Some(10));
    assert_eq!(deque.contains(&10), false);
    assert_eq!(deque.contains(&20), true);
    assert_eq!(deque.contains(&50), true);

    // After mutating via front_mut, contains reflects the new value
    if let Some(f) = deque.front_mut() {
        *f = 777;
    }
    assert_eq!(deque.contains(&20), false);
    assert_eq!(deque.contains(&777), true);
    assert_eq!(deque.front(), Some(&777));
}

#[test]
fn test_wraparound_ring_buffer_semantics() {
    let mut deque: ArrayDeque<[i32; 5]> = ArrayDeque::new();

    // Fill the buffer to capacity
    deque.push_back(1);
    deque.push_back(2);
    deque.push_back(3);
    deque.push_back(4);
    assert_eq!(deque.len(), 4);
    assert_eq!(deque.front(), Some(&1));
    assert_eq!(deque.back(), Some(&4));

    // Pop two from front and push two to the back to force ring-buffer wrap
    assert_eq!(deque.pop_front(), Some(1));
    assert_eq!(deque.pop_front(), Some(2));
    deque.push_back(5);
    deque.push_back(6);

    // Logical order is now [3, 4, 5, 6] regardless of internal layout
    assert_eq!(deque.len(), 4);
    assert_eq!(deque.front(), Some(&3));
    assert_eq!(deque.back(), Some(&6));

    // get_mut indexes the logical (not physical) order
    assert_eq!(deque.get_mut(0).map(|x| *x), Some(3));
    assert_eq!(deque.get_mut(1).map(|x| *x), Some(4));
    assert_eq!(deque.get_mut(2).map(|x| *x), Some(5));
    assert_eq!(deque.get_mut(3).map(|x| *x), Some(6));
    assert!(deque.get_mut(4).is_none());

    // Mutate a wrapped element and verify via contains
    *deque.get_mut(2).expect("wrapped index 2 valid") = 555;
    assert_eq!(deque.contains(&555), true);
    assert_eq!(deque.contains(&5), false);
    assert_eq!(deque.contains(&3), true);
    assert_eq!(deque.contains(&6), true);

    // Ends are not disturbed by inner mutation
    assert_eq!(deque.front(), Some(&3));
    assert_eq!(deque.back(), Some(&6));
}

#[test]
fn test_front_back_after_mixed_pushes_and_pops() {
    let mut deque: ArrayDeque<[u32; 7]> = ArrayDeque::new();

    // Build up using interleaved push_front and push_back
    deque.push_back(10);   // [10]
    deque.push_front(5);   // [5, 10]
    deque.push_back(20);   // [5, 10, 20]
    deque.push_front(1);   // [1, 5, 10, 20]

    assert_eq!(deque.len(), 4);
    assert_eq!(deque.front(), Some(&1));
    assert_eq!(deque.back(), Some(&20));

    // contains over interleaved-built deque
    assert_eq!(deque.contains(&1), true);
    assert_eq!(deque.contains(&5), true);
    assert_eq!(deque.contains(&10), true);
    assert_eq!(deque.contains(&20), true);
    assert_eq!(deque.contains(&999), false);

    // Mutate every element using get_mut, doubling each
    let len = deque.len();
    for i in 0..len {
        let v = deque.get_mut(i).expect("valid index");
        *v *= 2;
    }

    // Post-state: doubled values, ends doubled too
    assert_eq!(deque.front(), Some(&2));
    assert_eq!(deque.back(), Some(&40));
    assert_eq!(deque.get_mut(1).map(|x| *x), Some(10));
    assert_eq!(deque.get_mut(2).map(|x| *x), Some(20));

    // Drain ends and confirm front/back update accordingly
    assert_eq!(deque.pop_front(), Some(2));
    assert_eq!(deque.pop_back(), Some(40));
    assert_eq!(deque.front(), Some(&10));
    assert_eq!(deque.back(), Some(&20));
    assert_eq!(deque.len(), 2);

    // Empty it out and reconfirm None on front/back/get_mut
    assert_eq!(deque.pop_front(), Some(10));
    assert_eq!(deque.pop_back(), Some(20));
    assert_eq!(deque.front(), None);
    assert_eq!(deque.back(), None);
    assert!(deque.front_mut().is_none());
    assert!(deque.back_mut().is_none());
    assert!(deque.get_mut(0).is_none());
    assert_eq!(deque.contains(&10), false);
}