use parking_lot::Mutex;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll, Waker};

pub struct JoinHandle<T> {
    state: Arc<JoinState<T>>,
}

pub(crate) struct JoinState<T> {
    result: Mutex<Option<Result<T, JoinError>>>,
    waker: Mutex<Option<Waker>>,
}

impl<T> JoinHandle<T> {
    pub(crate) fn new() -> (Self, JoinNotifier<T>) {
        let state = Arc::new(JoinState {
            result: Mutex::new(None),
            waker: Mutex::new(None),
        });

        let handle = JoinHandle {
            state: state.clone(),
        };
        let notifier = JoinNotifier { state };

        (handle, notifier)
    }

    pub fn is_finished(&self) -> bool {
        self.state.result.lock().is_some()
    }
}

pub(crate) struct JoinNotifier<T> {
    state: Arc<JoinState<T>>,
}

impl<T> JoinNotifier<T> {
    pub fn complete(self, result: Result<T, JoinError>) {
        *self.state.result.lock() = Some(result);
        if let Some(waker) = self.state.waker.lock().take() {
            waker.wake();
        }
    }
}

impl<T> Future for JoinHandle<T> {
    type Output = Result<T, JoinError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut result_guard = self.state.result.lock();

        if let Some(result) = result_guard.take() {
            return Poll::Ready(result);
        }

        *self.state.waker.lock() = Some(cx.waker().clone());
        Poll::Pending
    }
}

#[derive(Debug)]
pub enum JoinError {
    Cancelled,
    Panicked,
}

impl std::fmt::Display for JoinError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JoinError::Cancelled => write!(f, "task was cancelled"),
            JoinError::Panicked => write!(f, "task panicked"),
        }
    }
}

impl std::error::Error for JoinError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn join_handle_starts_not_finished() {
        let (handle, _notifier): (JoinHandle<i32>, _) = JoinHandle::new();
        assert!(!handle.is_finished());
    }

    #[test]
    fn join_handle_is_finished_after_complete() {
        let (handle, notifier): (JoinHandle<i32>, _) = JoinHandle::new();
        notifier.complete(Ok(42));
        assert!(handle.is_finished());
    }

    #[test]
    fn join_error_display_cancelled() {
        let error = JoinError::Cancelled;
        assert_eq!(format!("{}", error), "task was cancelled");
    }

    #[test]
    fn join_error_display_panicked() {
        let error = JoinError::Panicked;
        assert_eq!(format!("{}", error), "task panicked");
    }
}
