use std::future::Future;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::task::Poll;

use async_task::spawn;
use smol::future;

fn try_await<T>(f: impl Future<Output = T>) -> Option<T> {
    future::block_on(future::poll_once(f))
}

#[test]
fn is_finished_false_before_run() {
    let (runnable, task) = spawn(async { 42i32 }, |_r| {});


    assert_eq!(task.is_finished(), false);


    assert_eq!(task.is_finished(), false);


    runnable.run();


    assert_eq!(task.is_finished(), true);
    assert_eq!(task.is_finished(), true);


    let output = future::block_on(task);
    assert_eq!(output, 42);
}

#[test]
fn is_finished_with_pending_then_ready() {
    static POLL_COUNT: AtomicUsize = AtomicUsize::new(0);

    let (s, r) = flume::unbounded();
    let schedule = move |runnable| s.send(runnable).unwrap();

    let (runnable, task) = spawn(
        future::poll_fn(|cx| {
            let count = POLL_COUNT.fetch_add(1, Ordering::SeqCst);
            if count == 0 {

                cx.waker().wake_by_ref();
                Poll::Pending
            } else {

                Poll::Ready(99u64)
            }
        }),
        schedule,
    );


    assert_eq!(task.is_finished(), false);


    runnable.run();


    assert_eq!(task.is_finished(), false);
    assert_eq!(POLL_COUNT.load(Ordering::SeqCst), 1);


    let runnable2 = r.recv().unwrap();
    assert_eq!(task.is_finished(), false);

    runnable2.run();


    assert_eq!(task.is_finished(), true);
    assert_eq!(POLL_COUNT.load(Ordering::SeqCst), 2);

    let output = future::block_on(task);
    assert_eq!(output, 99);
}

#[test]
fn is_finished_after_drop_runnable() {
    let (runnable, task) = spawn(async { String::from("hello") }, |_r| {});

    assert_eq!(task.is_finished(), false);


    drop(runnable);


    let finished_after_cancel = task.is_finished();

    assert_eq!(task.is_finished(), finished_after_cancel);


    let fallible = task.fallible();
    let result = future::block_on(fallible);

    assert!(result.is_none());
}

#[test]
fn is_finished_detached_task_after_run() {
    static EXECUTED: AtomicUsize = AtomicUsize::new(0);

    let (runnable, task) = spawn(
        async {
            EXECUTED.fetch_add(1, Ordering::SeqCst);
            123i32
        },
        |_r| {},
    );

    assert_eq!(task.is_finished(), false);
    assert_eq!(EXECUTED.load(Ordering::SeqCst), 0);


    assert_eq!(task.is_finished(), false);


    runnable.run();


    assert_eq!(task.is_finished(), true);
    assert_eq!(EXECUTED.load(Ordering::SeqCst), 1);


    task.detach();
}

#[test]
fn is_finished_multiple_tasks_independent() {
    let (s1, _r1) = flume::unbounded();
    let schedule1 = move |runnable| s1.send(runnable).unwrap();

    let (s2, _r2) = flume::unbounded();
    let schedule2 = move |runnable| s2.send(runnable).unwrap();

    let (runnable1, task1) = spawn(async { 1u32 }, schedule1);
    let (runnable2, task2) = spawn(async { 2u32 }, schedule2);


    assert_eq!(task1.is_finished(), false);
    assert_eq!(task2.is_finished(), false);


    runnable1.run();


    assert_eq!(task1.is_finished(), true);
    assert_eq!(task2.is_finished(), false);


    runnable2.run();


    assert_eq!(task1.is_finished(), true);
    assert_eq!(task2.is_finished(), true);

    let out1 = future::block_on(task1);
    let out2 = future::block_on(task2);
    assert_eq!(out1, 1);
    assert_eq!(out2, 2);
}

#[test]
fn is_finished_with_spawn_unchecked() {
    let (s, _r) = flume::unbounded();
    let schedule = move |runnable| s.send(runnable).unwrap();

    let (runnable, task) = unsafe { async_task::spawn_unchecked(async { 77u8 }, schedule) };

    assert_eq!(task.is_finished(), false);
    assert_eq!(task.is_finished(), false);

    runnable.run();

    assert_eq!(task.is_finished(), true);
    assert_eq!(task.is_finished(), true);

    let output = future::block_on(task);
    assert_eq!(output, 77u8);
}

#[test]
fn is_finished_task_with_waker_reschedule() {
    let (s, r) = flume::unbounded();
    let schedule = move |runnable| s.send(runnable).unwrap();

    let counter = Arc::new(AtomicUsize::new(0));
    let counter_clone = counter.clone();

    let (runnable, task) = spawn(
        future::poll_fn(move |cx| {
            let val = counter_clone.fetch_add(1, Ordering::SeqCst);
            if val < 3 {
                cx.waker().wake_by_ref();
                Poll::Pending
            } else {
                Poll::Ready(val)
            }
        }),
        schedule,
    );

    assert_eq!(task.is_finished(), false);


    runnable.run();
    assert_eq!(task.is_finished(), false);
    assert_eq!(counter.load(Ordering::SeqCst), 1);


    let r2 = r.recv().unwrap();
    r2.run();
    assert_eq!(task.is_finished(), false);
    assert_eq!(counter.load(Ordering::SeqCst), 2);


    let r3 = r.recv().unwrap();
    r3.run();
    assert_eq!(task.is_finished(), false);
    assert_eq!(counter.load(Ordering::SeqCst), 3);


    let r4 = r.recv().unwrap();
    r4.run();
    assert_eq!(task.is_finished(), true);
    assert_eq!(counter.load(Ordering::SeqCst), 4);

    let output = future::block_on(task);
    assert_eq!(output, 3);
}

#[test]
fn is_finished_consistency_across_polls() {
    let (runnable, mut task) = spawn(async { vec![1, 2, 3] }, |_r| {});


    assert_eq!(task.is_finished(), false);
    let poll_result = try_await(&mut task);
    assert!(poll_result.is_none());
    assert_eq!(task.is_finished(), false);


    runnable.run();


    assert_eq!(task.is_finished(), true);
    let poll_result = try_await(&mut task);
    assert!(poll_result.is_some());
    let output = poll_result.unwrap();
    assert_eq!(output, vec![1, 2, 3]);
}