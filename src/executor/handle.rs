use std::thread;

pub struct RuntimeHandle {
    worker_handles: Vec<thread::JoinHandle<()>>,
}

impl RuntimeHandle {
    pub(super) fn new(worker_handles: Vec<thread::JoinHandle<()>>) -> Self {
        RuntimeHandle { worker_handles }
    }

    pub fn wait(self) {
        for handle in self.worker_handles {
            let _ = handle.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_handle_new_creates_with_handles() {
        let handles = vec![];
        let runtime_handle = RuntimeHandle::new(handles);
        runtime_handle.wait();
    }

    #[test]
    fn runtime_handle_wait_joins_all_threads() {
        let handles: Vec<thread::JoinHandle<()>> = (0..3).map(|_| thread::spawn(|| {})).collect();

        let runtime_handle = RuntimeHandle::new(handles);
        runtime_handle.wait();
    }
}
