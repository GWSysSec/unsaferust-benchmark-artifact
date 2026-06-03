use std::cell::Cell;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::task::{Context, Poll};

use async_task::{spawn_local, Runnable, Task};
use smol::future;

fn try_await<T>(f: impl Future<Output = T>) -> Option<T> {
    future::block_on(future::poll_once(f))
}

#[test]
fn spawn_local_drop_and_detach() {
    static POLL: AtomicUsize = AtomicUsize::new(0);
    static DROP_F: AtomicUsize = AtomicUsize::new(0);
    static SCHEDULE: AtomicUsize = AtomicUsize::new(0);
    static DROP_S: AtomicUsize = AtomicUsize::new(0);

    struct Fut(Box<i32>);
    impl Future for Fut {
        type Output = Box<i32>;
        fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
            POLL.fetch_add(1, Ordering::SeqCst);
            Poll::Ready(Box::new(42))
        }
    }
    impl Drop for Fut {
        fn drop(&mut self) {
            DROP_F.fetch_add(1, Ordering::SeqCst);
        }
    }

    struct Guard(Box<i32>);
    impl Drop for Guard {
        fn drop(&mut self) {
            DROP_S.fetch_add(1, Ordering::SeqCst);
        }
    }

    let guard = Guard(Box::new(0));
    let schedule = move |_runnable: Runnable| {
        let _ = &guard;
        SCHEDULE.fetch_add(1, Ordering::SeqCst);
    };

    let (runnable, task) = spawn_local(Fut(Box::new(0)), schedule);

    assert_eq!(POLL.load(Ordering::SeqCst), 0);
    assert_eq!(SCHEDULE.load(Ordering::SeqCst), 0);
    assert_eq!(DROP_F.load(Ordering::SeqCst), 0);
    assert_eq!(DROP_S.load(Ordering::SeqCst), 0);

    drop(runnable);
    assert_eq!(POLL.load(Ordering::SeqCst), 0);
    assert_eq!(SCHEDULE.load(Ordering::SeqCst), 0);
    assert_eq!(DROP_F.load(Ordering::SeqCst), 1);
    assert_eq!(DROP_S.load(Ordering::SeqCst), 0);

    task.detach();
    assert_eq!(POLL.load(Ordering::SeqCst), 0);
    assert_eq!(SCHEDULE.load(Ordering::SeqCst), 0);
    assert_eq!(DROP_F.load(Ordering::SeqCst), 1);
    assert_eq!(DROP_S.load(Ordering::SeqCst), 1);
}

#[test]
fn spawn_local_detach_and_run() {
    static POLL: AtomicUsize = AtomicUsize::new(0);
    static DROP_F: AtomicUsize = AtomicUsize::new(0);
    static SCHEDULE: AtomicUsize = AtomicUsize::new(0);
    static DROP_S: AtomicUsize = AtomicUsize::new(0);

    struct Fut(Box<i32>);
    impl Future for Fut {
        type Output = Box<i32>;
        fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
            POLL.fetch_add(1, Ordering::SeqCst);
            Poll::Ready(Box::new(7))
        }
    }
    impl Drop for Fut {
        fn drop(&mut self) {
            DROP_F.fetch_add(1, Ordering::SeqCst);
        }
    }

    struct Guard(Box<i32>);
    impl Drop for Guard {
        fn drop(&mut self) {
            DROP_S.fetch_add(1, Ordering::SeqCst);
        }
    }

    let guard = Guard(Box::new(0));
    let schedule = move |_runnable: Runnable| {
        let _ = &guard;
        SCHEDULE.fetch_add(1, Ordering::SeqCst);
    };

    let (runnable, task) = spawn_local(Fut(Box::new(0)), schedule);

    task.detach();
    assert_eq!(POLL.load(Ordering::SeqCst), 0);
    assert_eq!(SCHEDULE.load(Ordering::SeqCst), 0);
    assert_eq!(DROP_F.load(Ordering::SeqCst), 0);
    assert_eq!(DROP_S.load(Ordering::SeqCst), 0);

    runnable.run();
    assert_eq!(POLL.load(Ordering::SeqCst), 1);
    assert_eq!(SCHEDULE.load(Ordering::SeqCst), 0);
    assert_eq!(DROP_F.load(Ordering::SeqCst), 1);
    assert_eq!(DROP_S.load(Ordering::SeqCst), 1);
}

#[test]
fn spawn_local_run_and_await() {
    static POLL: AtomicUsize = AtomicUsize::new(0);
    static DROP_F: AtomicUsize = AtomicUsize::new(0);
    static SCHEDULE: AtomicUsize = AtomicUsize::new(0);
    static DROP_S: AtomicUsize = AtomicUsize::new(0);

    struct Fut(i32);
    impl Future for Fut {
        type Output = i32;
        fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
            POLL.fetch_add(1, Ordering::SeqCst);
            Poll::Ready(self.0 * 2)
        }
    }
    impl Drop for Fut {
        fn drop(&mut self) {
            DROP_F.fetch_add(1, Ordering::SeqCst);
        }
    }

    struct Guard;
    impl Drop for Guard {
        fn drop(&mut self) {
            DROP_S.fetch_add(1, Ordering::SeqCst);
        }
    }

    let guard = Guard;
    let schedule = move |_runnable: Runnable| {
        let _ = &guard;
        SCHEDULE.fetch_add(1, Ordering::SeqCst);
    };

    let (runnable, mut task) = spawn_local(Fut(21), schedule);


    assert!(try_await(&mut task).is_none());
    assert_eq!(POLL.load(Ordering::SeqCst), 0);
    assert_eq!(DROP_F.load(Ordering::SeqCst), 0);

    runnable.run();
    assert_eq!(POLL.load(Ordering::SeqCst), 1);
    assert_eq!(DROP_F.load(Ordering::SeqCst), 1);
    assert_eq!(SCHEDULE.load(Ordering::SeqCst), 0);


    let result = try_await(&mut task);
    assert!(result.is_some());
    assert_eq!(result.unwrap(), 42);

    drop(task);
    assert_eq!(DROP_S.load(Ordering::SeqCst), 1);
}

#[test]
fn spawn_local_cancel_and_run() {
    static POLL: AtomicUsize = AtomicUsize::new(0);
    static DROP_F: AtomicUsize = AtomicUsize::new(0);
    static SCHEDULE: AtomicUsize = AtomicUsize::new(0);
    static DROP_S: AtomicUsize = AtomicUsize::new(0);

    struct Fut;
    impl Future for Fut {
        type Output = i32;
        fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
            POLL.fetch_add(1, Ordering::SeqCst);
            Poll::Ready(99)
        }
    }
    impl Drop for Fut {
        fn drop(&mut self) {
            DROP_F.fetch_add(1, Ordering::SeqCst);
        }
    }

    struct Guard;
    impl Drop for Guard {
        fn drop(&mut self) {
            DROP_S.fetch_add(1, Ordering::SeqCst);
        }
    }

    let guard = Guard;
    let schedule = move |_runnable: Runnable| {
        let _ = &guard;
        SCHEDULE.fetch_add(1, Ordering::SeqCst);
    };

    let (runnable, task) = spawn_local(Fut, schedule);


    drop(task);
    assert_eq!(POLL.load(Ordering::SeqCst), 0);
    assert_eq!(SCHEDULE.load(Ordering::SeqCst), 0);
    assert_eq!(DROP_F.load(Ordering::SeqCst), 0);
    assert_eq!(DROP_S.load(Ordering::SeqCst), 0);


    runnable.run();
    assert_eq!(POLL.load(Ordering::SeqCst), 0);
    assert_eq!(SCHEDULE.load(Ordering::SeqCst), 0);
    assert_eq!(DROP_F.load(Ordering::SeqCst), 1);
    assert_eq!(DROP_S.load(Ordering::SeqCst), 1);
}

#[test]
fn spawn_local_non_send_future() {

    use std::rc::Rc;

    static POLL_COUNT: AtomicUsize = AtomicUsize::new(0);
    static SCHED_COUNT: AtomicUsize = AtomicUsize::new(0);

    let shared = Rc::new(Cell::new(10));
    let shared_clone = shared.clone();

    let schedule = move |_runnable: Runnable| {
        SCHED_COUNT.fetch_add(1, Ordering::SeqCst);
    };

    let (runnable, mut task) = spawn_local(
        async move {
            POLL_COUNT.fetch_add(1, Ordering::SeqCst);
            let val = shared_clone.get();
            shared_clone.set(val + 5);
            shared_clone.get()
        },
        schedule,
    );

    assert_eq!(POLL_COUNT.load(Ordering::SeqCst), 0);
    assert_eq!(SCHED_COUNT.load(Ordering::SeqCst), 0);


    assert_eq!(shared.get(), 10);

    runnable.run();
    assert_eq!(POLL_COUNT.load(Ordering::SeqCst), 1);
    assert_eq!(SCHED_COUNT.load(Ordering::SeqCst), 0);

    let result = try_await(&mut task);
    assert!(result.is_some());
    assert_eq!(result.unwrap(), 15);


    assert_eq!(shared.get(), 15);
}

#[test]
fn spawn_local_schedule_reschedule() {
    static SCHED_COUNT: AtomicUsize = AtomicUsize::new(0);

    let (s, r) = flume::unbounded();
    let schedule = move |runnable: Runnable| {
        SCHED_COUNT.fetch_add(1, Ordering::SeqCst);
        s.send(runnable).unwrap();
    };

    let poll_count = Arc::new(AtomicUsize::new(0));
    let poll_count_clone = poll_count.clone();

    let (runnable, _task) = spawn_local(
        future::poll_fn(move |_cx| {
            let count = poll_count_clone.fetch_add(1, Ordering::SeqCst);
            if count < 2 {
                Poll::<i32>::Pending
            } else {
                Poll::Ready(100)
            }
        }),
        schedule,
    );

    assert_eq!(SCHED_COUNT.load(Ordering::SeqCst), 0);
    assert!(r.is_empty());


    runnable.schedule();
    assert_eq!(SCHED_COUNT.load(Ordering::SeqCst), 1);


    let r1 = r.recv().unwrap();
    r1.run();
    assert_eq!(poll_count.load(Ordering::SeqCst), 1);








    assert_eq!(SCHED_COUNT.load(Ordering::SeqCst), 1);
}

#[test]
fn spawn_local_waker_wakes_into_schedule() {
    static SCHED_COUNT: AtomicUsize = AtomicUsize::new(0);

    let (s, r) = flume::unbounded();
    let schedule = move |runnable: Runnable| {
        SCHED_COUNT.fetch_add(1, Ordering::SeqCst);
        s.send(runnable).unwrap();
    };

    let (runnable, _task) = spawn_local(
        future::poll_fn(|_cx| Poll::<()>::Pending),
        schedule,
    );

    assert!(r.is_empty());
    assert_eq!(SCHED_COUNT.load(Ordering::SeqCst), 0);

    let waker = runnable.waker();


    runnable.run();
    assert_eq!(SCHED_COUNT.load(Ordering::SeqCst), 0);
    assert!(r.is_empty());


    waker.wake_by_ref();
    assert_eq!(SCHED_COUNT.load(Ordering::SeqCst), 1);
    assert!(!r.is_empty());

    let runnable2 = r.recv().unwrap();
    runnable2.run();
    assert_eq!(SCHED_COUNT.load(Ordering::SeqCst), 1);


    waker.wake();
    assert_eq!(SCHED_COUNT.load(Ordering::SeqCst), 2);
    assert!(!r.is_empty());
}

#[test]
fn spawn_local_non_send_output() {

    use std::rc::Rc;

    static POLL_COUNT: AtomicUsize = AtomicUsize::new(0);

    let schedule = move |_runnable: Runnable| {};

    let (runnable, mut task) = spawn_local(
        async {
            POLL_COUNT.fetch_add(1, Ordering::SeqCst);
            Rc::new(vec![1, 2, 3, 4, 5])
        },
        schedule,
    );

    assert_eq!(POLL_COUNT.load(Ordering::SeqCst), 0);
    assert!(try_await(&mut task).is_none());

    runnable.run();
    assert_eq!(POLL_COUNT.load(Ordering::SeqCst), 1);

    let result = try_await(&mut task);
    assert!(result.is_some());
    let rc_val = result.unwrap();
    assert_eq!(rc_val.len(), 5);
    assert_eq!(rc_val[0], 1);
    assert_eq!(rc_val[4], 5);
    assert_eq!(Rc::strong_count(&rc_val), 1);
}

#[test]
fn spawn_local_multiple_tasks_same_schedule() {
    static SCHED_COUNT: AtomicUsize = AtomicUsize::new(0);
    static DROP_COUNT: AtomicUsize = AtomicUsize::new(0);

    let (s, r) = flume::unbounded();
    let schedule = move |runnable: Runnable| {
        SCHED_COUNT.fetch_add(1, Ordering::SeqCst);
        s.send(runnable).unwrap();
    };

    let (r1, mut t1) = spawn_local(async { 10i32 }, schedule.clone());
    let (r2, mut t2) = spawn_local(async { 20i32 }, schedule.clone());
    let (r3, mut t3) = spawn_local(
        async {
            DROP_COUNT.fetch_add(1, Ordering::SeqCst);
            30i32
        },
        schedule.clone(),
    );


    assert!(try_await(&mut t1).is_none());
    assert!(try_await(&mut t2).is_none());
    assert!(try_await(&mut t3).is_none());


    r1.run();
    r2.run();
    r3.run();

    let v1 = try_await(&mut t1);
    let v2 = try_await(&mut t2);
    let v3 = try_await(&mut t3);

    assert_eq!(v1.unwrap(), 10);
    assert_eq!(v2.unwrap(), 20);
    assert_eq!(v3.unwrap(), 30);
    assert_eq!(DROP_COUNT.load(Ordering::SeqCst), 1);
    assert_eq!(SCHED_COUNT.load(Ordering::SeqCst), 0);
}