use crate::{thread_pool::ThreadPool, Result};
use crossbeam::{
    channel,
    channel::{Receiver, Sender},
};
use std::thread;
use tracing::{debug, error};

pub type Job = Box<dyn FnOnce() + Send + 'static>;

#[derive(Clone)]
struct ReceiverManager(Receiver<Job>);

#[allow(clippy::module_name_repetitions)]
#[derive(Clone)]
pub struct SharedQueueThreadPool {
    tx: Sender<Job>,
}

impl ThreadPool for SharedQueueThreadPool {
    fn new(threads: u32) -> Result<Self>
    where
        Self: Sized,
    {
        let (tx, rx) = channel::unbounded::<Job>();
        for _ in 0..threads {
            spawn_receiver(ReceiverManager(rx.clone()));
        }
        Ok(SharedQueueThreadPool { tx })
    }

    fn spawn<F>(&self, job: F)
    where
        F: FnOnce() + Send + 'static,
    {
        if let Err(err) = self.tx.send(Box::new(job)) {
            debug!("Unexpected thread pool spawn error: {}", err);
        };
    }
}

fn spawn_receiver(rx: ReceiverManager) {
    let result = thread::Builder::new().spawn(move || loop {
        if let Ok(job) = rx.0.recv() {
            job();
        };
    });
    if let Err(e) = result {
        error!("Failed to spawn a new thread: {}", e);
    };
}

impl Drop for ReceiverManager {
    fn drop(&mut self) {
        if thread::panicking() {
            spawn_receiver(self.clone());
        }
    }
}
