use crate::{thread_pool::ThreadPool, KvsError::ThreadError, Result};
use rayon::ThreadPoolBuilder;

#[allow(clippy::module_name_repetitions)]
pub struct RayonThreadPool(rayon::ThreadPool);

impl ThreadPool for RayonThreadPool {
    fn new(threads: u32) -> Result<Self>
    where
        Self: Sized,
    {
        let pool = ThreadPoolBuilder::new()
            .num_threads(threads as usize)
            .build()
            .map_err(|e| ThreadError(e.to_string()))?;
        Ok(RayonThreadPool(pool))
    }

    fn spawn<F>(&self, job: F)
    where
        F: FnOnce() + Send + 'static,
    {
        self.0.spawn(job);
    }
}
