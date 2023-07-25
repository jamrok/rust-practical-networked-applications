#![deny(clippy::all)]
// #![deny(missing_docs)]

pub use std::io::Result;
use std::{collections::HashMap, path::PathBuf};

#[derive(Default)]
pub struct KvStore {
    data: HashMap<String, String>,
}

impl KvStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the string value of the given string key.
    ///
    /// If the key does not exist, return None.
    ///
    /// `kvs` first searches the in-memory index and retrieves the corresponding log-pointer, which
    /// is then used to find the `get` command in the log (on-disk). The command is then evaluated
    /// and the result is returned.
    ///
    /// # Errors
    ///
    /// If the value is not read successfully.
    pub fn get(&self, key: String) -> Result<Option<String>> {
        Ok(self.data.get(&key).cloned())
    }

    /// Removes the given string key.
    ///
    /// `kvs` first writes the `rm` command to the sequential log on-disk and then removes the key
    /// from the in-memory index.
    ///
    /// # Errors
    ///
    /// - If the key does not exist.
    /// - If the key is not removed successfully.
    pub fn remove(&mut self, key: String) -> Result<()> {
        self.data.remove(&key);
        Ok(())
    }

    /// Saves the given string value to the given string key.
    ///
    /// `kvs` first writes the `set` command to disk in a sequential log, then stores the log
    /// pointer (file offset) of that command to the in-memory index (i.e. the value stored
    /// in-memory is the log pointer of the command).
    ///
    /// # Errors
    ///
    /// If the value is not written successfully.
    pub fn set(&mut self, key: String, value: String) -> Result<()> {
        self.data.insert(key, value);
        Ok(())
    }

    /// Open the KvStore at a given path and return the KvStore.
    ///
    /// # Errors
    ///
    /// If there was a problem opening the KvStore.
    pub fn open(_path: impl Into<PathBuf>) -> Result<KvStore> {
        Ok(KvStore::new())
    }
}
