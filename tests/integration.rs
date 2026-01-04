use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use runtime::{Runtime, sleep};

const TEST_WORKER_COUNT: usize = 2;
const TASK_EXECUTION_WAIT: Duration = Duration::from_millis(50);
const MULTIPLE_TASKS_WAIT: Duration = Duration::from_millis(100);
const TIMER_DURATION: Duration = Duration::from_millis(100);
const TIMER_WAIT: Duration = Duration::from_millis(200);

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
    thread::sleep(MULTIPLE_TASKS_WAIT);

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
fn spawner_works_after_runtime_shutdown() {
    let runtime = Runtime::new();
    let spawner_before_shutdown = runtime.spawner();
    let spawner_for_test = runtime.spawner();

    spawner_before_shutdown.spawn(async {}).unwrap();

    let _handle = runtime.run(1);

    drop(spawner_before_shutdown);
    runtime.shutdown();

    let result = spawner_for_test.spawn(async {});
    assert!(result.is_ok());
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
    thread::sleep(MULTIPLE_TASKS_WAIT);

    assert_eq!(counter.load(Ordering::SeqCst), 2);
}
