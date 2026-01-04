use std::{
    sync::{Arc, Mutex},
    task::Context,
    thread,
};

use crossbeam::channel::{Receiver, Sender, unbounded};
use futures::task::waker_ref;

use crate::task::Task;

pub struct Runtime {
    scheduled_task_receiver: Receiver<Arc<Task>>,
    scheduled_task_sender: Sender<Arc<Task>>,
}

#[derive(Clone)]
pub struct Spawner {
    task_sender: Sender<Arc<Task>>,
}

impl Spawner {
    pub fn spawn<F>(&self, future: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        let future = Box::pin(future);
        let task = Arc::new(Task {
            future: Mutex::new(Some(future)),
            task_sender: self.task_sender.clone(),
        });

        self.task_sender.send(task).expect("query closed");
    }
}

impl Default for Runtime {
    fn default() -> Self {
        Self::new()
    }
}

impl Runtime {
    pub fn new() -> Self {
        let (sender, receiver) = unbounded();

        Runtime {
            scheduled_task_receiver: receiver,
            scheduled_task_sender: sender,
        }
    }

    pub fn spawner(&self) -> Spawner {
        Spawner {
            task_sender: self.scheduled_task_sender.clone(),
        }
    }

    pub fn run(&self, num_workers: usize) {
        let mut handles = Vec::with_capacity(num_workers);

        for id in 0..num_workers {
            let receiver = self.scheduled_task_receiver.clone();

            handles.push(thread::spawn(move || {
                while let Ok(task) = receiver.recv() {
                    let waker = waker_ref(&task);
                    let mut context = Context::from_waker(&waker);
                    let mut future_slot = task.future.lock().unwrap();

                    if let Some(mut future) = future_slot.as_mut() {
                        if future.as_mut().poll(&mut context).is_pending() {
                            //
                        }
                    }
                }
            }));

            println!("worker {} ended work", id);
        }

        for h in handles {
            h.join().unwrap();
        }
    }
}
