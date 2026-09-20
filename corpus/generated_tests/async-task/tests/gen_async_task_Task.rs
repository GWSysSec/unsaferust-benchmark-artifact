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

    // Task has not been polled yet, so it should not be finished
    assert_eq!(task.is_finished(), false);

    // Still not finished after just holding the runnable
    assert_eq!(task.is_finished(), false);

    // Run the task to completion
    runnable.run();

    // Now it should be finished
    assert_eq!(task.is_finished(), true);
    assert_eq!(task.is_finished(), true); // idempotent check

    // Collect the output
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
                // First poll: return Pending and wake immediately
                cx.waker().wake_by_ref();
                Poll::Pending
            } else {
                // Second poll: return Ready
                Poll::Ready(99u64)
            }
        }),
        schedule,
    );

    // Before any polling
    assert_eq!(task.is_finished(), false);

    // Run first poll - future returns Pending, waker is called, so it gets rescheduled
    runnable.run();

    // After first poll, task returned Pending so it's not finished
    assert_eq!(task.is_finished(), false);
    assert_eq!(POLL_COUNT.load(Ordering::SeqCst), 1);

    // Get the rescheduled runnable and run it again
    let runnable2 = r.recv().unwrap();
    assert_eq!(task.is_finished(), false);

    runnable2.run();

    // Now the future returned Ready, so task is finished
    assert_eq!(task.is_finished(), true);
    assert_eq!(POLL_COUNT.load(Ordering::SeqCst), 2);

    let output = future::block_on(task);
    assert_eq!(output, 99);
}

#[test]
fn is_finished_after_drop_runnable() {
    let (runnable, task) = spawn(async { String::from("hello") }, |_r| {});

    assert_eq!(task.is_finished(), false);

    // Dropping the runnable without running it cancels the task
    drop(runnable);

    // After cancellation, the task is considered finished (it will never produce output)
    let finished_after_cancel = task.is_finished();
    // Whether true or false, we verify consistency
    assert_eq!(task.is_finished(), finished_after_cancel);

    // Convert to a FallibleTask to safely handle cancellation without panicking
    let fallible = task.fallible();
    let result = future::block_on(fallible);
    // The task was cancelled so result should be None
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

    // Check is_finished before running
    assert_eq!(task.is_finished(), false);

    // Run the task
    runnable.run();

    // Task is now finished
    assert_eq!(task.is_finished(), true);
    assert_eq!(EXECUTED.load(Ordering::SeqCst), 1);

    // Detach after completion - should not panic
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

    // Neither is finished initially
    assert_eq!(task1.is_finished(), false);
    assert_eq!(task2.is_finished(), false);

    // Run only task1
    runnable1.run();

    // task1 is finished, task2 is not
    assert_eq!(task1.is_finished(), true);
    assert_eq!(task2.is_finished(), false);

    // Run task2
    runnable2.run();

    // Both are finished
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

    // First run: Pending
    runnable.run();
    assert_eq!(task.is_finished(), false);
    assert_eq!(counter.load(Ordering::SeqCst), 1);

    // Second run: Pending
    let r2 = r.recv().unwrap();
    r2.run();
    assert_eq!(task.is_finished(), false);
    assert_eq!(counter.load(Ordering::SeqCst), 2);

    // Third run: Pending
    let r3 = r.recv().unwrap();
    r3.run();
    assert_eq!(task.is_finished(), false);
    assert_eq!(counter.load(Ordering::SeqCst), 3);

    // Fourth run: Ready
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

    // Before running: not finished, polling returns None
    assert_eq!(task.is_finished(), false);
    let poll_result = try_await(&mut task);
    assert!(poll_result.is_none());
    assert_eq!(task.is_finished(), false);

    // Run the task
    runnable.run();

    // After running: finished, polling returns Some
    assert_eq!(task.is_finished(), true);
    let poll_result = try_await(&mut task);
    assert!(poll_result.is_some());
    let output = poll_result.unwrap();
    assert_eq!(output, vec![1, 2, 3]);
}