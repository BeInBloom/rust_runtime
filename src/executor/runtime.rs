use crossbeam_deque::Injector;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;

use super::handle::RuntimeHandle;
use super::spawner::Spawner;
use super::task::Task;
use super::worker::run_worker_loop;

/// Async runtime with work-stealing executor.
///
/// # Example
///
/// ```
/// use runtime::Runtime;
/// use std::time::Duration;
///
/// let runtime = Runtime::new();
/// let spawner = runtime.spawner();
///
/// spawner.spawn(async {
///     println!("Hello from async task!");
/// }).unwrap();
///
/// let handle = runtime.run(2);
/// std::thread::sleep(Duration::from_millis(50));
/// runtime.shutdown();
/// handle.wait();
/// ```
pub struct Runtime {
    global_queue: Arc<Injector<Arc<Task>>>,
    is_shutdown: Arc<AtomicBool>,
}

impl Default for Runtime {
    fn default() -> Self {
        Self::new()
    }
}

impl Runtime {
    pub fn new() -> Self {
        Runtime {
            global_queue: Arc::new(Injector::new()),
            is_shutdown: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn spawner(&self) -> Spawner {
        Spawner::new(self.global_queue.clone(), self.is_shutdown.clone())
    }

    pub fn run(&self, num_workers: usize) -> RuntimeHandle {
        let global_queue = self.global_queue.clone();
        let is_shutdown = self.is_shutdown.clone();

        let worker_handles: Vec<thread::JoinHandle<()>> = (0..num_workers)
            .map(|worker_id| {
                let queue = global_queue.clone();
                let shutdown = is_shutdown.clone();
                thread::spawn(move || run_worker_loop(worker_id, queue, shutdown))
            })
            .collect();

        RuntimeHandle::new(worker_handles)
    }

    pub fn run_blocking(&self, num_workers: usize) {
        self.run(num_workers).wait();
    }

    pub fn shutdown(self) {
        self.is_shutdown.store(true, Ordering::SeqCst);
    }
}
