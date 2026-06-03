use async_std::prelude::*;
use async_std::stream;
use async_std::task::block_on;

#[test]
fn test_from_iter_collects_in_order() {
    block_on(async {
        let mut s = stream::from_iter(vec![10u32, 20, 30, 40]);
        let mut collected = Vec::new();
        while let Some(v) = s.next().await {
            collected.push(v);
        }
        assert_eq!(collected.len(), 4);
        assert_eq!(collected[0], 10);
        assert_eq!(collected[1], 20);
        assert_eq!(collected[2], 30);
        assert_eq!(collected[3], 40);
        assert_eq!(collected.iter().sum::<u32>(), 100);
        assert_ne!(collected, vec![40, 30, 20, 10]);
    });
}

#[test]
fn test_empty_stream_yields_nothing() {
    block_on(async {
        let mut s: stream::Empty<i32> = stream::empty();
        let first = s.next().await;
        assert!(first.is_none());
        assert_eq!(first, None);
        let second = s.next().await;
        assert!(second.is_none());
        assert_eq!(second, None);


        let mut count = 0usize;
        let mut s2: stream::Empty<String> = stream::empty();
        while let Some(_) = s2.next().await {
            count += 1;
        }
        assert_eq!(count, 0);
        assert_ne!(count, 1);
    });
}

#[test]
fn test_once_yields_single_item() {
    block_on(async {
        let mut s = stream::once(42i64);
        let first = s.next().await;
        assert_eq!(first, Some(42));
        let second = s.next().await;
        assert_eq!(second, None);
        assert!(second.is_none());

        let mut s2 = stream::once(String::from("hello"));
        let v = s2.next().await.unwrap();
        assert_eq!(v.len(), 5);
        assert_eq!(v, "hello");
        assert!(s2.next().await.is_none());
    });
}

#[test]
fn test_repeat_take_some() {
    block_on(async {
        let s = stream::repeat(7u8);
        let mut taken = s.take(5);
        let mut buf = Vec::new();
        while let Some(v) = taken.next().await {
            buf.push(v);
        }
        assert_eq!(buf.len(), 5);
        assert_eq!(buf, vec![7, 7, 7, 7, 7]);
        assert_eq!(buf.iter().map(|x| *x as u32).sum::<u32>(), 35);
        assert_ne!(buf, vec![7, 7, 7, 7]);
        assert!(buf.iter().all(|&v| v == 7));
    });
}

#[test]
fn test_repeat_with_closure() {
    block_on(async {
        let mut i = 0i32;
        let s = stream::repeat_with(move || {
            i += 1;
            i
        });
        let mut taken = s.take(4);
        let mut collected = Vec::new();
        while let Some(v) = taken.next().await {
            collected.push(v);
        }
        assert_eq!(collected.len(), 4);
        assert_eq!(collected, vec![1, 2, 3, 4]);
        assert_eq!(collected[0], 1);
        assert_eq!(collected[3], 4);
        assert_ne!(collected[0], collected[1]);
        assert_eq!(collected.iter().sum::<i32>(), 10);
    });
}

#[test]
fn test_from_fn_finite_stream() {
    block_on(async {
        let mut counter = 0u32;
        let s = stream::from_fn(move || {
            counter += 1;
            if counter <= 3 {
                Some(counter * 10)
            } else {
                None
            }
        });
        let mut pinned = Box::pin(s);
        let mut out = Vec::new();
        while let Some(v) = pinned.next().await {
            out.push(v);
        }
        assert_eq!(out.len(), 3);
        assert_eq!(out, vec![10, 20, 30]);
        assert_eq!(out[0], 10);
        assert_eq!(out[2], 30);
        assert_ne!(out, vec![10, 20]);
        assert!(pinned.next().await.is_none());
        assert_eq!(out.iter().sum::<u32>(), 60);
    });
}