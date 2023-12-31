mod naive;
mod rayon;
mod shared_queue;

pub use self::rayon::*;
pub use naive::*;
pub use shared_queue::*;

use crate::Result;

pub trait ThreadPool: Send + Sync + 'static {
    /// Creates a new thread pool, immediately spawning the specified number of threads.
    ///
    /// Returns an error if any thread fails to spawn. All previously-spawned threads are terminated.
    fn new(threads: u32) -> Result<Self>
    where
        Self: Sized;

    /// Spawn a function into the thread pool.
    ///
    /// Spawning always succeeds, but if the function panics the thread pool continues to operate
    /// with the same number of threads. i.e. The thread count is not reduced nor is the thread pool
    /// destroyed, corrupted or invalidated.
    fn spawn<F>(&self, job: F)
    where
        F: FnOnce() + Send + 'static;
}
