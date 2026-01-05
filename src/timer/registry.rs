use std::collections::BTreeMap;
use std::mem;
use std::task::Waker;
use std::time::{Duration, Instant};

#[derive(Default)]
pub(super) struct TimerRegistry {
    timers: BTreeMap<Instant, Vec<Waker>>,
}

impl TimerRegistry {
    pub fn register(&mut self, deadline: Instant, waker: Waker) {
        self.timers.entry(deadline).or_default().push(waker);
    }

    pub fn next_deadline(&self) -> Option<Instant> {
        self.timers.keys().next().copied()
    }

    pub fn pop_ready_wakers(&mut self, now: Instant) -> Vec<Waker> {
        let pending = self.timers.split_off(&(now + Duration::from_nanos(1)));
        let ready = mem::replace(&mut self.timers, pending);
        ready.into_values().flatten().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::task::{Wake, Waker};

    struct TestWaker {
        wake_count: Arc<AtomicUsize>,
    }

    impl Wake for TestWaker {
        fn wake(self: Arc<Self>) {
            self.wake_count.fetch_add(1, Ordering::SeqCst);
        }
    }

    fn create_test_waker() -> (Waker, Arc<AtomicUsize>) {
        let count = Arc::new(AtomicUsize::new(0));
        let waker = Arc::new(TestWaker {
            wake_count: count.clone(),
        });
        (Waker::from(waker), count)
    }

    #[test]
    fn registry_starts_empty() {
        let registry = TimerRegistry::default();
        assert!(registry.next_deadline().is_none());
    }

    #[test]
    fn registry_registers_timer() {
        let mut registry = TimerRegistry::default();
        let (waker, _) = create_test_waker();
        let deadline = Instant::now() + Duration::from_secs(1);

        registry.register(deadline, waker);

        assert_eq!(registry.next_deadline(), Some(deadline));
    }

    #[test]
    fn registry_returns_earliest_deadline() {
        let mut registry = TimerRegistry::default();
        let (waker1, _) = create_test_waker();
        let (waker2, _) = create_test_waker();

        let early = Instant::now() + Duration::from_millis(100);
        let late = Instant::now() + Duration::from_secs(1);

        registry.register(late, waker1);
        registry.register(early, waker2);

        assert_eq!(registry.next_deadline(), Some(early));
    }

    #[test]
    fn registry_pops_ready_wakers() {
        let mut registry = TimerRegistry::default();
        let (waker, count) = create_test_waker();

        let past = Instant::now() - Duration::from_millis(100);
        registry.register(past, waker);

        let wakers = registry.pop_ready_wakers(Instant::now());

        assert_eq!(wakers.len(), 1);
        wakers[0].wake_by_ref();
        assert_eq!(count.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn registry_keeps_future_timers() {
        let mut registry = TimerRegistry::default();
        let (waker1, _) = create_test_waker();
        let (waker2, _) = create_test_waker();

        let past = Instant::now() - Duration::from_millis(100);
        let future = Instant::now() + Duration::from_secs(10);

        registry.register(past, waker1);
        registry.register(future, waker2);

        let _ = registry.pop_ready_wakers(Instant::now());

        assert_eq!(registry.next_deadline(), Some(future));
    }
}
