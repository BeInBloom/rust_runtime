use std::{
    collections::BTreeMap,
    mem,
    pin::Pin,
    sync::{Arc, OnceLock},
    task::{Context, Poll, Waker},
    thread,
    time::{Duration, Instant},
};

use parking_lot::{Condvar, Mutex, MutexGuard};

const REACTOR_THREAD_NAME: &str = "timer-reactor";

struct TimerRegistry {
    timers: BTreeMap<Instant, Vec<Waker>>,
}

impl Default for TimerRegistry {
    fn default() -> Self {
        Self {
            timers: BTreeMap::new(),
        }
    }
}

impl TimerRegistry {
    fn register(&mut self, deadline: Instant, waker: Waker) {
        self.timers.entry(deadline).or_default().push(waker);
    }

    fn next_deadline(&self) -> Option<Instant> {
        self.timers.keys().next().copied()
    }

    fn pop_ready_wakers(&mut self, now: Instant) -> Vec<Waker> {
        let pending = self.timers.split_off(&(now + Duration::from_nanos(1)));
        let ready = mem::replace(&mut self.timers, pending);
        ready.into_values().flatten().collect()
    }
}

pub struct Reactor {
    registry: Mutex<TimerRegistry>,
    condvar: Condvar,
}

impl Reactor {
    fn new() -> Arc<Self> {
        Arc::new(Reactor {
            registry: Mutex::new(TimerRegistry::default()),
            condvar: Condvar::new(),
        })
    }

    fn run(self: Arc<Self>) {
        let mut registry = self.registry.lock();

        loop {
            let now = Instant::now();

            match registry.next_deadline() {
                Some(deadline) if now >= deadline => {
                    registry = self.process_ready_timers(registry, now);
                }
                Some(deadline) => {
                    registry = self.park_until(registry, deadline);
                }
                None => {
                    registry = self.park_indefinitely(registry);
                }
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

        self.registry.lock()
    }

    fn park_until<'a>(
        &self,
        mut registry: MutexGuard<'a, TimerRegistry>,
        deadline: Instant,
    ) -> MutexGuard<'a, TimerRegistry> {
        let now = Instant::now();

        if deadline > now {
            self.condvar.wait_for(&mut registry, deadline - now);
        }

        registry
    }

    fn park_indefinitely<'a>(
        &self,
        mut registry: MutexGuard<'a, TimerRegistry>,
    ) -> MutexGuard<'a, TimerRegistry> {
        self.condvar.wait(&mut registry);
        registry
    }

    fn register_timer(&self, deadline: Instant, waker: Waker) {
        let mut registry = self.registry.lock();
        registry.register(deadline, waker);
        self.condvar.notify_one();
    }
}

static GLOBAL_REACTOR: OnceLock<Arc<Reactor>> = OnceLock::new();

fn get_reactor() -> &'static Arc<Reactor> {
    GLOBAL_REACTOR.get_or_init(initialize_reactor)
}

fn initialize_reactor() -> Arc<Reactor> {
    let reactor = Reactor::new();
    spawn_reactor_thread(reactor.clone());
    reactor
}

fn spawn_reactor_thread(reactor: Arc<Reactor>) {
    thread::Builder::new()
        .name(REACTOR_THREAD_NAME.to_string())
        .spawn(move || reactor.run())
        .expect("failed to spawn reactor thread");
}

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

pub fn sleep(duration: Duration) -> SleepFuture {
    SleepFuture {
        deadline: Instant::now() + duration,
        is_registered: false,
    }
}
