use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use runtime::{CancellationToken, Runtime, sleep};

const TEST_WORKER_COUNT: usize = 2;
const TASK_EXECUTION_WAIT: Duration = Duration::from_millis(100);
const TIMER_DURATION: Duration = Duration::from_millis(100);
const TIMER_WAIT: Duration = Duration::from_millis(250);

#[test]
fn single_task_executes() {
    let counter = Arc::new(AtomicUsize::new(0));
    let counter_clone = counter.clone();

    let runtime = Runtime::new();
    let spawner = runtime.spawner();

    spawner
        .spawn(async move {
            counter_clone.fetch_add(1, Ordering::SeqCst);
        })
        .unwrap();

    let _handle = runtime.run(1);
    thread::sleep(TASK_EXECUTION_WAIT);

    assert_eq!(counter.load(Ordering::SeqCst), 1);
}

#[test]
fn multiple_tasks_execute_in_parallel() {
    let task_count = 10;
    let counter = Arc::new(AtomicUsize::new(0));

    let runtime = Runtime::new();
    let spawner = runtime.spawner();

    for _ in 0..task_count {
        let counter_clone = counter.clone();
        spawner
            .spawn(async move {
                counter_clone.fetch_add(1, Ordering::SeqCst);
            })
            .unwrap();
    }

    let _handle = runtime.run(TEST_WORKER_COUNT);
    thread::sleep(TASK_EXECUTION_WAIT);

    assert_eq!(counter.load(Ordering::SeqCst), task_count);
}

#[test]
fn sleep_timer_waits_correct_duration() {
    let runtime = Runtime::new();
    let spawner = runtime.spawner();

    let completed = Arc::new(AtomicUsize::new(0));
    let completed_clone = completed.clone();

    let start = Instant::now();

    spawner
        .spawn(async move {
            sleep(TIMER_DURATION).await;
            completed_clone.fetch_add(1, Ordering::SeqCst);
        })
        .unwrap();

    let _handle = runtime.run(1);
    thread::sleep(TIMER_WAIT);

    let elapsed = start.elapsed();

    assert_eq!(completed.load(Ordering::SeqCst), 1);
    assert!(elapsed >= TIMER_DURATION);
}

#[test]
fn spawn_returns_join_handle_with_result() {
    let runtime = Runtime::new();
    let spawner = runtime.spawner();

    let result = Arc::new(AtomicUsize::new(0));
    let result_clone = result.clone();

    let handle = spawner.spawn(async { 42_usize }).unwrap();

    spawner
        .spawn(async move {
            if let Ok(value) = handle.await {
                result_clone.store(value, Ordering::SeqCst);
            }
        })
        .unwrap();

    let _handle = runtime.run(TEST_WORKER_COUNT);
    thread::sleep(TASK_EXECUTION_WAIT);

    assert_eq!(result.load(Ordering::SeqCst), 42);
}

#[test]
fn cancellation_token_cancels_task() {
    let runtime = Runtime::new();
    let spawner = runtime.spawner();

    let was_cancelled = Arc::new(AtomicUsize::new(0));
    let was_cancelled_clone = was_cancelled.clone();

    let token = CancellationToken::new();
    let token_clone = token.clone();

    spawner
        .spawn(async move {
            while !token_clone.is_cancelled() {
                sleep(Duration::from_millis(10)).await;
            }
            was_cancelled_clone.store(1, Ordering::SeqCst);
        })
        .unwrap();

    spawner
        .spawn(async move {
            sleep(Duration::from_millis(50)).await;
            token.cancel();
        })
        .unwrap();

    let _handle = runtime.run(TEST_WORKER_COUNT);
    thread::sleep(Duration::from_millis(200));

    assert_eq!(was_cancelled.load(Ordering::SeqCst), 1);
}

#[test]
fn nested_spawn_executes_both_tasks() {
    let runtime = Runtime::new();
    let spawner = runtime.spawner();

    let counter = Arc::new(AtomicUsize::new(0));
    let counter_clone = counter.clone();
    let spawner_clone = spawner.clone();

    spawner
        .spawn(async move {
            counter_clone.fetch_add(1, Ordering::SeqCst);

            let counter_inner = counter_clone.clone();
            let _ = spawner_clone.spawn(async move {
                counter_inner.fetch_add(1, Ordering::SeqCst);
            });
        })
        .unwrap();

    let _handle = runtime.run(TEST_WORKER_COUNT);
    thread::sleep(TASK_EXECUTION_WAIT);

    assert_eq!(counter.load(Ordering::SeqCst), 2);
}
