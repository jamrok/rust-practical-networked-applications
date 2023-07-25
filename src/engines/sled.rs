use crate::{KvsEngine, KvsError::KeyNotFound, Result};
use sled::Db;
use std::path::PathBuf;

#[allow(clippy::module_name_repetitions)]
#[derive(Clone)]
pub struct SledKvsEngine {
    index: Db,
}

impl KvsEngine for SledKvsEngine {
    fn open(path: impl Into<PathBuf>) -> Result<Self> {
        let index = sled::open(path.into())?;
        Ok(Self { index })
    }

    fn get(&self, key: String) -> Result<Option<String>> {
        match self.index.get(key)? {
            None => Ok(None),
            Some(val) => Ok(Some(String::from_utf8(val.to_vec())?)),
        }
    }

    fn remove(&self, key: String) -> Result<()> {
        if self.index.remove(key)?.is_none() {
            Err(KeyNotFound)
        } else {
            Ok(())
        }
    }

    fn set(&self, key: String, value: String) -> Result<()> {
        self.index.insert(key, value.as_str())?;
        Ok(())
    }
}
