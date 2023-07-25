use crate::{KvsEngine, KvsError::KeyNotFound, Result};
use sled::Db;
use std::path::PathBuf;

pub struct SledKvsEngine {
    index: Db,
}

impl SledKvsEngine {
    pub fn new(path: PathBuf) -> Result<Self> {
        let index = sled::open(path)?;
        Ok(Self { index })
    }
}

impl KvsEngine for SledKvsEngine {
    fn get(&mut self, key: String) -> Result<Option<String>> {
        match self.index.get(key)? {
            None => Ok(None),
            Some(val) => Ok(Some(String::from_utf8(val.to_vec())?)),
        }
    }

    fn remove(&mut self, key: String) -> Result<()> {
        if self.index.remove(key)?.is_none() {
            Err(KeyNotFound)
        } else {
            Ok(())
        }
    }

    fn set(&mut self, key: String, value: String) -> Result<()> {
        self.index.insert(key, value.as_str())?;
        Ok(())
    }
}
