pub mod kvs;
pub mod sled;

pub use self::{kvs::KvStore, sled::SledKvsEngine};
use crate::Result;
use std::path::PathBuf;

pub trait KvsEngine: Clone + Send + Sync + 'static {
    /// Open the `KvsEngine` at a given path and return it.
    ///
    /// # Errors
    ///
    /// If there was a problem opening the `KvsEngine`.
    fn open(path: impl Into<PathBuf>) -> Result<Self>;

    /// Returns the string value of the given string key.
    ///
    /// If the key does not exist, return `None`.
    ///
    /// `kvs` first searches the in-memory index and retrieves the corresponding log-pointer, which
    /// is then used to find the `get` command in the log (on-disk). The command is then evaluated
    /// and the result is returned.
    ///
    /// # Errors
    ///
    /// If the value is not read successfully.
    fn get(&self, key: String) -> Result<Option<String>>;

    /// Removes the given string key.
    ///
    /// `kvs` first writes the `rm` command to the sequential log on-disk and then removes the key
    /// from the in-memory index.
    ///
    /// # Errors
    ///
    /// - If the key does not exist.
    /// - If the key is not removed successfully.
    fn remove(&self, key: String) -> Result<()>;

    /// Saves the given string value to the given string key.
    ///
    /// `kvs` first writes the `set` command to disk in a sequential log, then stores the log
    /// pointer (file offset) of that command to the in-memory index (i.e. the value stored
    /// in-memory is the log pointer of the command).
    ///
    /// # Errors
    ///
    /// If the value is not written successfully.
    fn set(&self, key: String, value: String) -> Result<()>;
}
