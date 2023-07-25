#![deny(clippy::all)]
#![deny(clippy::pedantic)]
#![deny(clippy::unwrap_used)]
// Yes, I know... I know... Ideally I should fix these 😉
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]
// #![deny(missing_docs)]
pub mod client;
mod engines;
mod errors;
pub mod serde;
pub mod server;
pub mod shared;
pub mod thread_pool;

pub use engines::{KvStore, KvsEngine, SledKvsEngine};
pub use errors::{KvsError, Result};
