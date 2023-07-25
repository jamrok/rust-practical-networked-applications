#![deny(clippy::all)]
// #![deny(missing_docs)]

use std::collections::HashMap;

#[derive(Default)]
pub struct KvStore {
    data: HashMap<String, String>,
}

impl KvStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&self, key: String) -> Option<String> {
        self.data.get(&key).cloned()
    }

    pub fn remove(&mut self, key: String) {
        self.data.remove(&key);
    }

    pub fn set(&mut self, key: String, value: String) {
        self.data.insert(key, value);
    }
}
