#![deny(clippy::all)]
// #![deny(missing_docs)]
mod engines;
mod errors;
pub mod serde;
pub mod server;
pub mod shared;
pub mod thread_pool;

pub use engines::{KvStore, KvsEngine, SledKvsEngine};
pub use errors::{KvsError, Result};
