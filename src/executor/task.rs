use futures::task::ArcWake;
use parking_lot::Mutex;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use crossbeam_deque::Injector;

pub(crate) type BoxFuture = Pin<Box<dyn Future<Output = ()> + Send + 'static>>;

pub(crate) struct Task {
    future: Mutex<Option<BoxFuture>>,
    global_queue: Arc<Injector<Arc<Task>>>,
}

impl Task {
    pub(crate) fn new(future: BoxFuture, global_queue: Arc<Injector<Arc<Task>>>) -> Self {
        Task {
            future: Mutex::new(Some(future)),
            global_queue,
        }
    }

    pub(crate) fn future_slot(&self) -> &Mutex<Option<BoxFuture>> {
        &self.future
    }
}

impl ArcWake for Task {
    fn wake_by_ref(arc_self: &Arc<Self>) {
        arc_self.global_queue.push(arc_self.clone());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn task_new_creates_with_future() {
        let queue = Arc::new(Injector::new());
        let future = Box::pin(async {});
        let task = Task::new(future, queue);

        assert!(task.future_slot().lock().is_some());
    }

    #[test]
    fn task_future_can_be_taken() {
        let queue = Arc::new(Injector::new());
        let future = Box::pin(async {});
        let task = Task::new(future, queue);

        let taken = task.future_slot().lock().take();
        assert!(taken.is_some());
        assert!(task.future_slot().lock().is_none());
    }

    #[test]
    fn task_wake_adds_to_queue() {
        let queue = Arc::new(Injector::new());
        let future = Box::pin(async {});
        let task = Arc::new(Task::new(future, queue.clone()));

        ArcWake::wake_by_ref(&task);

        assert!(!queue.is_empty());
    }
}
