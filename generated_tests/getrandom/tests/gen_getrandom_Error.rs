use getrandom::{fill, fill_uninit, Error};
use core::mem::MaybeUninit;

#[test]
fn test_new_custom_basic() {
    let e1 = Error::new_custom(1);
    let e2 = Error::new_custom(42);
    let e3 = Error::new_custom(65535);
    let e4 = Error::new_custom(1);


    assert_eq!(e1.raw_os_error(), None);
    assert_eq!(e2.raw_os_error(), None);
    assert_eq!(e3.raw_os_error(), None);
    assert_eq!(e4.raw_os_error(), None);


    assert_eq!(e1, e4);
    assert_ne!(e1, e2);
    assert_ne!(e2, e3);
    assert_ne!(e1, e3);
}

#[test]
fn test_new_custom_many_codes() {
    let codes: [u16; 8] = [1, 7, 100, 255, 256, 1000, 32000, 65000];
    let mut errors = Vec::with_capacity(codes.len());
    for &c in codes.iter() {
        let e = Error::new_custom(c);
        assert_eq!(e.raw_os_error(), None);
        errors.push(e);
    }
    assert_eq!(errors.len(), 8);


    for i in 0..errors.len() {
        for j in (i + 1)..errors.len() {
            assert_ne!(errors[i], errors[j]);
        }
    }


    let again = Error::new_custom(100);
    assert_eq!(again, errors[2]);
    assert_ne!(again, errors[0]);
    assert_eq!(again.raw_os_error(), None);
}

#[test]
fn test_new_custom_debug_and_display() {
    let e = Error::new_custom(12345);
    assert_eq!(e.raw_os_error(), None);

    let dbg = format!("{:?}", e);
    let disp = format!("{}", e);
    assert!(!dbg.is_empty());
    assert!(!disp.is_empty());
    assert_ne!(dbg.len(), 0);
    assert_ne!(disp.len(), 0);

    let e2 = Error::new_custom(12345);
    assert_eq!(e, e2);
    assert_eq!(format!("{:?}", e2), dbg);
    assert_eq!(e2.raw_os_error(), None);
}

#[test]
fn test_successful_calls_no_error_path() {


    let mut buf = [0u8; 32];
    let r = fill(&mut buf);
    assert!(r.is_ok());

    let mut ubuf: [MaybeUninit<u8>; 32] = [MaybeUninit::uninit(); 32];
    let r2 = fill_uninit(&mut ubuf);
    assert!(r2.is_ok());
    let slice = r2.unwrap();
    assert_eq!(slice.len(), 32);

    let a = getrandom::u32();
    let b = getrandom::u64();
    assert!(a.is_ok());
    assert!(b.is_ok());


    let ce = Error::new_custom(7);
    assert_eq!(ce.raw_os_error(), None);
    let ce2 = Error::new_custom(8);
    assert_ne!(ce, ce2);
}

#[test]
fn test_raw_os_error_none_for_customs_only() {


    for code in [0u16, 1, 2, 3, 100, 1000, 65535].iter().copied() {
        let e = Error::new_custom(code);
        let r = e.raw_os_error();
        assert!(r.is_none());
        assert_eq!(r, None);
    }


    let e1 = Error::new_custom(500);
    let e2 = Error::new_custom(500);
    let e3 = Error::new_custom(501);
    assert_eq!(e1, e2);
    assert_ne!(e1, e3);
    assert_eq!(e1.raw_os_error(), e2.raw_os_error());
    assert_eq!(e1.raw_os_error(), None);
    assert_eq!(e3.raw_os_error(), None);
}