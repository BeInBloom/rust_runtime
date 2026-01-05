use parking_lot::Mutex;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::task::{Context, Poll};

/// Token for cooperative task cancellation.
///
/// # Example
///
/// ```
/// use runtime::CancellationToken;
///
/// let token = CancellationToken::new();
/// let token_clone = token.clone();
///
/// // In a task: check if cancelled
/// if token_clone.is_cancelled() {
///     println!("Task was cancelled!");
/// }
///
/// // Cancel the token
/// token.cancel();
/// assert!(token_clone.is_cancelled());
/// ```
pub struct CancellationToken {
    inner: Arc<CancellationState>,
}

struct CancellationState {
    is_cancelled: AtomicBool,
    wakers: Mutex<Vec<std::task::Waker>>,
}

impl CancellationToken {
    pub fn new() -> Self {
        CancellationToken {
            inner: Arc::new(CancellationState {
                is_cancelled: AtomicBool::new(false),
                wakers: Mutex::new(Vec::new()),
            }),
        }
    }

    pub fn cancel(&self) {
        self.inner.is_cancelled.store(true, Ordering::SeqCst);
        let wakers = std::mem::take(&mut *self.inner.wakers.lock());
        for waker in wakers {
            waker.wake();
        }
    }

    pub fn is_cancelled(&self) -> bool {
        self.inner.is_cancelled.load(Ordering::SeqCst)
    }

    pub fn cancelled(&self) -> CancelledFuture {
        CancelledFuture {
            inner: self.inner.clone(),
        }
    }

    pub fn child_token(&self) -> CancellationToken {
        CancellationToken {
            inner: self.inner.clone(),
        }
    }
}

impl Default for CancellationToken {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for CancellationToken {
    fn clone(&self) -> Self {
        CancellationToken {
            inner: self.inner.clone(),
        }
    }
}

pub struct CancelledFuture {
    inner: Arc<CancellationState>,
}

impl Future for CancelledFuture {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.inner.is_cancelled.load(Ordering::SeqCst) {
            return Poll::Ready(());
        }

        self.inner.wakers.lock().push(cx.waker().clone());

        if self.inner.is_cancelled.load(Ordering::SeqCst) {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_starts_not_cancelled() {
        let token = CancellationToken::new();
        assert!(!token.is_cancelled());
    }

    #[test]
    fn token_cancel_sets_is_cancelled() {
        let token = CancellationToken::new();
        token.cancel();
        assert!(token.is_cancelled());
    }

    #[test]
    fn token_clone_shares_state() {
        let token1 = CancellationToken::new();
        let token2 = token1.clone();

        token1.cancel();

        assert!(token2.is_cancelled());
    }

    #[test]
    fn child_token_shares_state() {
        let parent = CancellationToken::new();
        let child = parent.child_token();

        parent.cancel();

        assert!(child.is_cancelled());
    }

    #[test]
    fn cancel_can_be_called_multiple_times() {
        let token = CancellationToken::new();

        token.cancel();
        token.cancel();
        token.cancel();

        assert!(token.is_cancelled());
    }

    #[test]
    fn default_creates_uncancelled_token() {
        let token = CancellationToken::default();
        assert!(!token.is_cancelled());
    }
}
