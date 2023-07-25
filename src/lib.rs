#![deny(clippy::all)]
// #![deny(missing_docs)]

mod engines;
mod errors;
pub use engines::{KvStore, KvsEngine};
pub use errors::{KvsErrors, Result};
