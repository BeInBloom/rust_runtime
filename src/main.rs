use std::time::Duration;

use runtime::{Runtime, sleep};

fn main() {
    let runtime = Runtime::new();
    let spawner = runtime.spawner();

    spawner.spawn(async {
        println!("task 1 starting waiting...");
        sleep(Duration::from_secs(2)).await;
        foo().await;
        println!("task 1 wakeup");
    });

    let spawner_clone = spawner.clone();
    spawner.spawn(async move {
        println!("Задача 2 стартовала.");
        sleep(Duration::from_secs(1)).await;
        println!("Задача 2 порождает подзадачу...");

        spawner_clone.spawn(async {
            println!("Подзадача (от Задачи 2) выполняется.");
        });
    });

    println!("Запуск 4 воркеров...");
    runtime.run(4);
}

async fn foo() {
    println!("hello from foo!");
}
