use async_lock::OnceCell;
use futures_lite::future;

#[test]
fn once_cell_basic_set_get() {
    let cell: OnceCell<u32> = OnceCell::new();
    assert!(!cell.is_initialized());
    assert!(cell.get().is_none());

    let r = cell.set_blocking(42);
    assert!(r.is_ok());
    assert_eq!(*r.unwrap(), 42);
    assert!(cell.is_initialized());
    assert_eq!(cell.get().copied(), Some(42));


    let r2 = cell.set_blocking(100);
    assert!(r2.is_err());
    assert_eq!(r2.unwrap_err(), 100);
    assert_eq!(cell.get().copied(), Some(42));

    unsafe {
        assert_eq!(*cell.get_unchecked(), 42);
    }
}

#[test]
fn once_cell_set_async() {
    future::block_on(async {
        let cell: OnceCell<String> = OnceCell::new();
        assert!(!cell.is_initialized());
        assert!(cell.get().is_none());

        let res = cell.set("hello".to_string()).await;
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), "hello");
        assert!(cell.is_initialized());
        assert_eq!(cell.get().map(|s| s.as_str()), Some("hello"));

        let res2 = cell.set("world".to_string()).await;
        assert!(res2.is_err());
        assert_eq!(res2.unwrap_err(), "world".to_string());
    });
}

#[test]
fn once_cell_take() {
    let mut cell: OnceCell<i64> = OnceCell::new();
    assert_eq!(cell.take(), None);
    assert!(!cell.is_initialized());

    cell.set_blocking(7).unwrap();
    assert!(cell.is_initialized());

    let taken = cell.take();
    assert_eq!(taken, Some(7));
    assert!(!cell.is_initialized());
    assert!(cell.get().is_none());


    cell.set_blocking(99).unwrap();
    assert_eq!(cell.get().copied(), Some(99));
}

#[test]
fn once_cell_get_or_init_blocking() {
    let cell: OnceCell<u32> = OnceCell::new();
    let mut call_count = 0;

    let v = cell.get_or_init_blocking(|| {
        call_count += 1;
        123
    });
    assert_eq!(*v, 123);
    assert_eq!(call_count, 1);
    assert!(cell.is_initialized());


    let v2 = cell.get_or_init_blocking(|| {
        call_count += 1;
        999
    });
    assert_eq!(*v2, 123);
    assert_eq!(call_count, 1);
}

#[test]
fn once_cell_get_or_init_async() {
    future::block_on(async {
        let cell: OnceCell<u32> = OnceCell::new();
        let v = cell.get_or_init(|| async { 55u32 }).await;
        assert_eq!(*v, 55);
        assert!(cell.is_initialized());

        let v2 = cell.get_or_init(|| async { 999u32 }).await;
        assert_eq!(*v2, 55);
        assert_eq!(cell.get().copied(), Some(55));
    });
}

#[test]
fn once_cell_get_or_try_init_blocking_err_then_ok() {
    let cell: OnceCell<u32> = OnceCell::new();

    let r1: Result<&u32, &'static str> = cell.get_or_try_init_blocking(|| Err("fail"));
    assert!(r1.is_err());
    assert_eq!(r1.unwrap_err(), "fail");
    assert!(!cell.is_initialized());
    assert!(cell.get().is_none());

    let r2: Result<&u32, &'static str> = cell.get_or_try_init_blocking(|| Ok(77));
    assert!(r2.is_ok());
    assert_eq!(*r2.unwrap(), 77);
    assert!(cell.is_initialized());


    let r3: Result<&u32, &'static str> = cell.get_or_try_init_blocking(|| Err("ignored"));
    assert!(r3.is_ok());
    assert_eq!(*r3.unwrap(), 77);
}

#[test]
fn once_cell_get_or_try_init_async() {
    future::block_on(async {
        let cell: OnceCell<i32> = OnceCell::new();

        let r1: Result<&i32, &'static str> =
            cell.get_or_try_init(|| async { Err("nope") }).await;
        assert!(r1.is_err());
        assert!(!cell.is_initialized());

        let r2: Result<&i32, &'static str> =
            cell.get_or_try_init(|| async { Ok(-5) }).await;
        assert!(r2.is_ok());
        assert_eq!(*r2.unwrap(), -5);
        assert!(cell.is_initialized());
        assert_eq!(cell.get().copied(), Some(-5));

        let r3: Result<&i32, &'static str> =
            cell.get_or_try_init(|| async { Err("still ignored") }).await;
        assert!(r3.is_ok());
        assert_eq!(*r3.unwrap(), -5);
    });
}