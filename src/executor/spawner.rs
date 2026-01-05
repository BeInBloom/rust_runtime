use crossbeam_deque::Injector;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use super::task::Task;
#[allow(unused_imports)]
use crate::join_handle::{JoinHandle, JoinNotifier};

const RUNTIME_STOPPED_MESSAGE: &str = "runtime has been stopped";

#[derive(Clone)]
pub struct Spawner {
    global_queue: Arc<Injector<Arc<Task>>>,
    is_shutdown: Arc<AtomicBool>,
}

impl Spawner {
    pub(super) fn new(
        global_queue: Arc<Injector<Arc<Task>>>,
        is_shutdown: Arc<AtomicBool>,
    ) -> Self {
        Spawner {
            global_queue,
            is_shutdown,
        }
    }

    pub fn spawn<F, T>(&self, future: F) -> Result<JoinHandle<T>, SpawnError>
    where
        F: Future<Output = T> + Send + 'static,
        T: Send + 'static,
    {
        if self.is_shutdown.load(Ordering::SeqCst) {
            return Err(SpawnError::RuntimeStopped);
        }

        let (handle, notifier) = JoinHandle::new();
        let queue = self.global_queue.clone();

        let wrapped_future = Box::pin(async move {
            let result = future.await;
            notifier.complete(Ok(result));
        });

        let task = Arc::new(Task::new(wrapped_future, queue));
        self.global_queue.push(task);

        Ok(handle)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spawn_error_display() {
        let error = SpawnError::RuntimeStopped;
        assert!(format!("{}", error).contains("stopped"));
    }

    #[test]
    fn spawn_error_is_debug() {
        let error = SpawnError::RuntimeStopped;
        let _ = format!("{:?}", error);
    }
}
