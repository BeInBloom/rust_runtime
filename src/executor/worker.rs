use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::task::{Context, Poll};
use std::thread;

use crossbeam_deque::Injector;
use futures::task::waker_ref;

use super::task::Task;

pub fn run_worker_loop(
    worker_id: usize,
    global_queue: Arc<Injector<Arc<Task>>>,
    is_shutdown: Arc<AtomicBool>,
) {
    loop {
        match global_queue.steal() {
            crossbeam_deque::Steal::Success(task) => {
                execute_task(&task);
            }
            crossbeam_deque::Steal::Empty => {
                if is_shutdown.load(Ordering::SeqCst) {
                    break;
                }
                thread::yield_now();
            }
            crossbeam_deque::Steal::Retry => continue,
        }
    }
    println!("worker {} ended work", worker_id);
}

fn execute_task(task: &Arc<Task>) {
    let waker = waker_ref(task);
    let mut context = Context::from_waker(&waker);
    let mut future_slot = task.future_slot().lock();

    let Some(future) = future_slot.as_mut() else {
        return;
    };

    let poll_result = catch_unwind(AssertUnwindSafe(|| future.as_mut().poll(&mut context)));

    match poll_result {
        Ok(Poll::Pending) => {
            // Task will be re-added to queue by ArcWake::wake_by_ref when ready
        }
        Ok(Poll::Ready(())) => {
            *future_slot = None;
        }
        Err(_) => {
            *future_slot = None;
            eprintln!("task panicked!");
        }
    }
}
