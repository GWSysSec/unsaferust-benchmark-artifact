use fragile::{SemiSticky, stack_token};
use std::sync::{Arc, Mutex};
use std::thread;

#[test]
fn test_semisticky_into_inner_workflow() {
    let s1 = SemiSticky::new(100u64);
    let v1 = s1.into_inner();
    assert_eq!(v1, 100u64);
    assert_ne!(v1, 0u64);

    let s2 = SemiSticky::new(String::from("hello world"));
    let v2 = s2.into_inner();
    assert_eq!(v2.len(), 11);
    assert_eq!(v2, "hello world");
    assert_eq!(v2.chars().next(), Some('h'));
    assert_ne!(v2, "goodbye");

    let s3 = SemiSticky::new(vec![1i32, 2, 3, 4, 5]);
    let v3 = s3.into_inner();
    assert_eq!(v3.len(), 5);
    assert_eq!(v3[0], 1);
    assert_eq!(v3[4], 5);
    assert_eq!(v3.iter().sum::<i32>(), 15);
    assert_ne!(v3[0], v3[1]);
}

#[test]
fn test_semisticky_into_inner_with_box() {
    let boxed = Box::new(42i64);
    let s = SemiSticky::new(boxed);
    let recovered = s.into_inner();
    assert_eq!(*recovered, 42i64);
    assert_ne!(*recovered, 0i64);

    let s2 = SemiSticky::new(Box::new(vec![10u8, 20, 30, 40]));
    let v = s2.into_inner();
    assert_eq!(v.len(), 4);
    assert_eq!(v[0], 10);
    assert_eq!(v[1], 20);
    assert_eq!(v[2], 30);
    assert_eq!(v[3], 40);
    assert_eq!(v.iter().map(|x| *x as u32).sum::<u32>(), 100);
}

#[test]
fn test_semisticky_try_into_inner_same_thread() {
    let s = SemiSticky::new(vec![10i32, 20, 30]);
    let res = s.try_into_inner();
    assert!(res.is_ok());
    let v = res.unwrap();
    assert_eq!(v.len(), 3);
    assert_eq!(v[0], 10);
    assert_eq!(v[1], 20);
    assert_eq!(v[2], 30);
    assert_eq!(v.iter().sum::<i32>(), 60);
    assert_ne!(v[0], v[2]);

    let s2 = SemiSticky::new(String::from("rustacean"));
    let res2 = s2.try_into_inner();
    assert!(res2.is_ok());
    let recovered = res2.unwrap();
    assert_eq!(recovered, "rustacean");
    assert_eq!(recovered.len(), 9);
}

#[test]
fn test_semisticky_try_into_inner_wrong_thread() {
    let holder: Arc<Mutex<Option<SemiSticky<Vec<i32>>>>> =
        Arc::new(Mutex::new(Some(SemiSticky::new(vec![7, 8, 9]))));
    let holder_clone = Arc::clone(&holder);

    let handle = thread::spawn(move || {
        let mut guard = holder_clone.try_lock().expect("lock failed");
        let taken = guard.take().expect("had value");
        let result = taken.try_into_inner();
        match result {
            Ok(_) => (false, None),
            Err(returned) => {
                *guard = Some(returned);
                (true, None::<()>)
            }
        }
    });

    let (was_err, _) = handle.join().expect("thread join");
    assert_eq!(was_err, true);
    assert_ne!(was_err, false);

    let mut guard = holder.try_lock().expect("re-lock");
    assert!(guard.is_some());
    let returned = guard.take().expect("returned value");
    let v = returned.into_inner();
    assert_eq!(v.len(), 3);
    assert_eq!(v[0], 7);
    assert_eq!(v[1], 8);
    assert_eq!(v[2], 9);
    assert_eq!(v.iter().sum::<i32>(), 24);
}

#[test]
fn test_semisticky_try_get_mut_same_thread() {
    let mut s = SemiSticky::new(vec![1i32, 2, 3]);

    {
        stack_token!(tok);
        let r = s.try_get_mut(&tok);
        assert!(r.is_ok());
        let v = r.unwrap();
        assert_eq!(v.len(), 3);
        assert_eq!(v[0], 1);
        v.push(4);
        v.push(5);
        assert_eq!(v.len(), 5);
        assert_eq!(v[3], 4);
        assert_eq!(v[4], 5);
    }

    {
        stack_token!(tok2);
        let r2 = s.try_get_mut(&tok2).expect("same thread access");
        assert_eq!(r2.len(), 5);
        assert_eq!(r2[0], 1);
        assert_eq!(r2[4], 5);
        r2[0] = 99;
        r2[4] = -1;
        assert_eq!(r2[0], 99);
        assert_eq!(r2[4], -1);
        assert_ne!(r2[0], 1);
    }

    let final_v = s.into_inner();
    assert_eq!(final_v.len(), 5);
    assert_eq!(final_v[0], 99);
    assert_eq!(final_v[4], -1);
    assert_eq!(final_v[1], 2);
    assert_eq!(final_v[2], 3);
}

#[test]
fn test_semisticky_try_get_mut_wrong_thread() {
    let holder: Arc<Mutex<SemiSticky<String>>> =
        Arc::new(Mutex::new(SemiSticky::new(String::from("origin"))));
    let holder_clone = Arc::clone(&holder);

    let handle = thread::spawn(move || {
        let mut guard = holder_clone.try_lock().expect("child lock");
        stack_token!(tok);
        let r = guard.try_get_mut(&tok);
        r.is_err()
    });

    let was_err = handle.join().expect("join");
    assert_eq!(was_err, true);
    assert_ne!(was_err, false);

    let mut guard = holder.try_lock().expect("parent lock");
    stack_token!(tok);
    let r = guard.try_get_mut(&tok);
    assert!(r.is_ok());
    let s = r.unwrap();
    assert_eq!(s, "origin");
    assert_eq!(s.len(), 6);
    s.push_str("-modified");
    assert_eq!(s, "origin-modified");
    assert_eq!(s.len(), 15);
    assert_ne!(s, "origin");
}

#[test]
fn test_semisticky_combined_workflow() {
    let mut s = SemiSticky::new(vec![0u32; 4]);

    {
        stack_token!(tok);
        let v = s.try_get_mut(&tok).expect("access");
        assert_eq!(v.len(), 4);
        for i in 0..4 {
            v[i] = (i as u32 + 1) * 10;
        }
        assert_eq!(v[0], 10);
        assert_eq!(v[1], 20);
        assert_eq!(v[2], 30);
        assert_eq!(v[3], 40);
    }

    let res = s.try_into_inner();
    assert!(res.is_ok());
    let v = res.unwrap();
    assert_eq!(v.len(), 4);
    assert_eq!(v.iter().sum::<u32>(), 100);
    assert_ne!(v[0], v[3]);
}