use std::{
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Context, Poll, Waker},
    thread,
    time::Duration,
};

pub struct TimerFuture {
    shared_state: Arc<Mutex<SharedState>>,
}

struct SharedState {
    completed: bool,
    waker: Option<Waker>,
}

impl Future for TimerFuture {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut share_state = self.shared_state.lock().unwrap();

        if share_state.completed {
            return Poll::Ready(());
        }

        share_state.waker = Some(cx.waker().clone());
        Poll::Pending
    }
}

pub fn sleep(duration: Duration) -> TimerFuture {
    let shared_state = Arc::new(Mutex::new(SharedState {
        completed: false,
        waker: None,
    }));

    let thread_shared_state = shared_state.clone();
    thread::spawn(move || {
        thread::sleep(duration);

        let mut shared_state = thread_shared_state.lock().unwrap();
        shared_state.completed = true;

        if let Some(waker) = shared_state.waker.take() {
            waker.wake();
        }
    });

    TimerFuture { shared_state }
}
