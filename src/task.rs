use crossbeam::channel::Sender;
use futures::task::ArcWake;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

pub type BoxFuture = Pin<Box<dyn Future<Output = ()> + Send + 'static>>;

pub struct Task {
    pub future: Mutex<Option<BoxFuture>>,
    pub task_sender: Sender<Arc<Task>>,
}

impl ArcWake for Task {
    fn wake_by_ref(arc_self: &Arc<Self>) {
        let cloned = arc_self.clone();
        arc_self
            .task_sender
            .send(cloned)
            .expect("Очередь задач закрыта");
    }
}
