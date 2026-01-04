use std::{
    collections::BTreeMap,
    mem,
    pin::Pin,
    sync::{Arc, Condvar, Mutex, MutexGuard, OnceLock},
    task::{Context, Poll, Waker},
    thread::{self, spawn},
    time::{Duration, Instant},
};

#[derive(Default)]
struct TimerRegistry {
    timers: BTreeMap<Instant, Vec<Waker>>,
}

impl TimerRegistry {
    fn new() -> Self {
        Self::default()
    }

    fn register(&mut self, deadline: Instant, waker: Waker) {
        self.timers.entry(deadline).or_default().push(waker);
    }

    fn next_deadline(&self) -> Option<Instant> {
        self.timers.keys().next().copied()
    }

    fn pop_ready_wakers(&mut self, now: Instant) -> Vec<Waker> {
        let pending = self.timers.split_off(&(now + Duration::from_nanos(1)));
        let ready_timers = mem::replace(&mut self.timers, pending);
        ready_timers.into_values().flatten().collect()
    }
}

pub struct Reactor {
    registry: Mutex<TimerRegistry>,
    condvar: Condvar,
}

impl Reactor {
    fn new() -> Arc<Self> {
        Arc::new(Reactor {
            registry: Mutex::new(TimerRegistry::new()),
            condvar: Condvar::new(),
        })
    }

    fn run(self: Arc<Self>) {
        let mut registry = self.registry.lock().unwrap();

        loop {
            let now = Instant::now();

            if let Some(deadline) = registry.next_deadline() {
                if now >= deadline {
                    registry = self.process_ready_timers(registry, now);
                } else {
                    registry = self.park_thread_until(registry, deadline);
                }
            } else {
                registry = self.park_thread_indefinitely(registry);
            }
        }
    }

    fn process_ready_timers(
        &self,
        mut registry: MutexGuard<TimerRegistry>,
        now: Instant,
    ) -> MutexGuard<'_, TimerRegistry> {
        let wakers = registry.pop_ready_wakers(now);

        drop(registry);

        for waker in wakers {
            waker.wake();
        }

        self.registry.lock().unwrap()
    }

    #[inline]
    fn park_thread_until<'a>(
        &self,
        registry: MutexGuard<'a, TimerRegistry>,
        deadline: Instant,
    ) -> MutexGuard<'a, TimerRegistry> {
        let now = Instant::now();

        if deadline > now {
            let duration = deadline - now;
            self.condvar.wait_timeout(registry, duration).unwrap().0
        } else {
            registry
        }
    }

    #[inline]
    fn park_thread_indefinitely<'a>(
        &self,
        registry: MutexGuard<'a, TimerRegistry>,
    ) -> MutexGuard<'a, TimerRegistry> {
        self.condvar.wait(registry).unwrap()
    }

    #[inline]
    fn registry(&self, deadline: Instant, waker: Waker) {
        let mut registry = self.registry.lock().unwrap();
        registry.register(deadline, waker);
        self.condvar.notify_one();
    }
}

static REACTOR: OnceLock<Arc<Reactor>> = OnceLock::new();

fn get_reactor() -> &'static Arc<Reactor> {
    REACTOR.get_or_init(|| {
        let reactor = Reactor::new();
        let reactor_clone = reactor.clone();

        thread::Builder::new()
            .name("timer-reactor".to_string())
            .spawn(move || reactor_clone.run())
            .expect("cant run reactor of thread");

        reactor
    })
}

pub struct SleepFuture {
    deadline: Instant,
    registered: bool,
}

impl Future for SleepFuture {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let now = Instant::now();

        if now > self.deadline {
            return Poll::Ready(());
        }

        if !self.registered {
            get_reactor().registry(self.deadline, cx.waker().clone());
            self.registered = true;
        }

        Poll::Pending
    }
}

pub fn sleep(duration: Duration) -> SleepFuture {
    SleepFuture {
        deadline: Instant::now() + duration,
        registered: false,
    }
}
