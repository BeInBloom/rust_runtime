use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::{Duration, Instant};

use super::reactor::get_reactor;

pub struct SleepFuture {
    deadline: Instant,
    is_registered: bool,
}

impl SleepFuture {
    fn is_ready(&self) -> bool {
        Instant::now() >= self.deadline
    }

    fn ensure_registered(&mut self, cx: &mut Context<'_>) {
        if !self.is_registered {
            get_reactor().register_timer(self.deadline, cx.waker().clone());
            self.is_registered = true;
        }
    }
}

impl Future for SleepFuture {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.is_ready() {
            return Poll::Ready(());
        }

        self.ensure_registered(cx);
        Poll::Pending
    }
}

/// Suspends the current task for the specified duration.
///
/// # Example
///
/// ```no_run
/// use runtime::sleep;
/// use std::time::Duration;
///
/// async fn example() {
///     sleep(Duration::from_secs(1)).await;
///     println!("One second has passed!");
/// }
/// ```
pub fn sleep(duration: Duration) -> SleepFuture {
    SleepFuture {
        deadline: Instant::now() + duration,
        is_registered: false,
    }
}
