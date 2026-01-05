use std::sync::{Arc, OnceLock};
use std::task::Waker;
use std::thread;
use std::time::Instant;

use parking_lot::{Condvar, Mutex, MutexGuard};

use super::registry::TimerRegistry;

const REACTOR_THREAD_NAME: &str = "timer-reactor";

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

    pub fn register_timer(&self, deadline: Instant, waker: Waker) {
        let mut registry = self.registry.lock();
        registry.register(deadline, waker);
        self.condvar.notify_one();
    }
}

static GLOBAL_REACTOR: OnceLock<Arc<Reactor>> = OnceLock::new();

pub(super) fn get_reactor() -> &'static Arc<Reactor> {
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
