use std::{
    panic::{AssertUnwindSafe, catch_unwind},
    sync::Arc,
    task::{Context, Poll},
    thread,
};

use crossbeam::channel::{Receiver, Sender, unbounded};
use futures::task::waker_ref;
use parking_lot::Mutex;

use crate::task::Task;

const RUNTIME_STOPPED_MESSAGE: &str = "runtime has been stopped";

pub struct Runtime {
    task_receiver: Receiver<Arc<Task>>,
    task_sender: Sender<Arc<Task>>,
}

#[derive(Clone)]
pub struct Spawner {
    task_sender: Sender<Arc<Task>>,
}

impl Spawner {
    pub fn spawn<F>(&self, future: F) -> Result<(), SpawnError>
    where
        F: Future<Output = ()> + Send + 'static,
    {
        let task = Task {
            future: Mutex::new(Some(Box::pin(future))),
            task_sender: self.task_sender.clone(),
        };

        self.task_sender
            .send(Arc::new(task))
            .map_err(|_| SpawnError::RuntimeStopped)
    }
}

#[derive(Debug)]
pub enum SpawnError {
    RuntimeStopped,
}

impl std::fmt::Display for SpawnError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SpawnError::RuntimeStopped => write!(f, "{}", RUNTIME_STOPPED_MESSAGE),
        }
    }
}

impl std::error::Error for SpawnError {}

impl Default for Runtime {
    fn default() -> Self {
        Self::new()
    }
}

impl Runtime {
    pub fn new() -> Self {
        let (task_sender, task_receiver) = unbounded();
        Runtime {
            task_receiver,
            task_sender,
        }
    }

    pub fn spawner(&self) -> Spawner {
        Spawner {
            task_sender: self.task_sender.clone(),
        }
    }

    pub fn run(&self, num_workers: usize) -> RuntimeHandle {
        RuntimeHandle {
            worker_handles: self.spawn_workers(num_workers),
        }
    }

    pub fn run_blocking(&self, num_workers: usize) {
        self.run(num_workers).wait();
    }

    pub fn shutdown(self) {
        drop(self.task_sender);
    }

    fn spawn_workers(&self, num_workers: usize) -> Vec<thread::JoinHandle<()>> {
        (0..num_workers)
            .map(|worker_id| self.spawn_worker(worker_id))
            .collect()
    }

    fn spawn_worker(&self, worker_id: usize) -> thread::JoinHandle<()> {
        let receiver = self.task_receiver.clone();
        thread::spawn(move || run_worker_loop(worker_id, receiver))
    }
}

pub struct RuntimeHandle {
    worker_handles: Vec<thread::JoinHandle<()>>,
}

impl RuntimeHandle {
    pub fn wait(self) {
        for handle in self.worker_handles {
            handle.join().expect("worker thread panicked");
        }
    }
}

fn run_worker_loop(worker_id: usize, task_receiver: Receiver<Arc<Task>>) {
    while let Ok(task) = task_receiver.recv() {
        execute_task(&task);
    }
    println!("worker {} ended work", worker_id);
}

fn execute_task(task: &Arc<Task>) {
    let waker = waker_ref(task);
    let mut context = Context::from_waker(&waker);
    let mut future_slot = task.future.lock();

    let Some(future) = future_slot.as_mut() else {
        return;
    };

    let poll_result = catch_unwind(AssertUnwindSafe(|| future.as_mut().poll(&mut context)));

    match poll_result {
        Ok(Poll::Pending) => {}
        Ok(Poll::Ready(())) => {
            *future_slot = None;
        }
        Err(_) => {
            *future_slot = None;
            eprintln!("task panicked!");
        }
    }
}
