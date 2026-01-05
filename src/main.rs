use std::time::Duration;

use runtime::{CancellationToken, Runtime, sleep};

const NUM_WORKERS: usize = 4;
const TASK_WAIT_DURATION: Duration = Duration::from_secs(3);

fn main() {
    let runtime = Runtime::new();
    let spawner = runtime.spawner();

    spawn_with_result(&spawner);
    spawn_delayed_task(&spawner);
    spawn_task_with_subtask(&spawner);
    spawn_counter_tasks(&spawner, 5);
    spawn_cancellable_task(&spawner);

    println!("Starting {} workers...", NUM_WORKERS);

    let handle = runtime.run(NUM_WORKERS);

    std::thread::sleep(TASK_WAIT_DURATION);

    println!("Stopping runtime...");
    runtime.shutdown();

    handle.wait();
    println!("Runtime stopped gracefully!");
}

fn spawn_with_result(spawner: &runtime::Spawner) {
    let handle = spawner
        .spawn(async {
            sleep(Duration::from_millis(100)).await;
            42
        })
        .expect("spawn failed");

    spawner
        .spawn(async move {
            match handle.await {
                Ok(result) => println!("[result] got value: {}", result),
                Err(e) => println!("[result] error: {}", e),
            }
        })
        .expect("spawn failed");
}

fn spawn_delayed_task(spawner: &runtime::Spawner) {
    spawner
        .spawn(async {
            println!("[delayed] starting...");
            sleep(Duration::from_secs(2)).await;
            println!("[delayed] woke up after 2 seconds");
        })
        .expect("spawn failed");
}

fn spawn_task_with_subtask(spawner: &runtime::Spawner) {
    let spawner_clone = spawner.clone();

    spawner
        .spawn(async move {
            println!("[parent] started");
            sleep(Duration::from_secs(1)).await;
            println!("[parent] spawning subtask...");

            spawner_clone
                .spawn(async {
                    println!("[subtask] executing");
                })
                .expect("spawn failed");
        })
        .expect("spawn failed");
}

fn spawn_counter_tasks(spawner: &runtime::Spawner, count: usize) {
    for i in 0..count {
        let task_id = i;
        spawner
            .spawn(async move {
                println!("[counter:{}] executed", task_id);
            })
            .expect("spawn failed");
    }
}

fn spawn_cancellable_task(spawner: &runtime::Spawner) {
    let token = CancellationToken::new();
    let token_clone = token.clone();

    spawner
        .spawn(async move {
            println!("[cancellable] started");

            loop {
                if token_clone.is_cancelled() {
                    println!("[cancellable] was cancelled!");
                    break;
                }
                sleep(Duration::from_millis(100)).await;
            }
        })
        .expect("spawn failed");

    spawner
        .spawn(async move {
            sleep(Duration::from_millis(500)).await;
            println!("[canceller] cancelling the cancellable task");
            token.cancel();
        })
        .expect("spawn failed");
}
