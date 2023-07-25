use crate::{
    serde::bincode::Serde,
    shared::{
        new_reader, new_writer, Command, Remove, Set, LOG_COMPACTION_MAX_KEY_DENSITY_PERCENT,
        LOG_ROTATION_MIN_SIZE_BYTES, LOG_ROTATION_MIN_SIZE_BYTES_DEFAULT,
    },
    KvsEngine,
    KvsError::{KeyNotFound, LogIndexIDError},
    Result,
};
use dashmap::DashMap;
use derive_more::{Constructor, From};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs::{self, File},
    io::{BufReader, BufWriter, Seek, SeekFrom},
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, RwLock},
};

#[derive(Constructor, Clone, Debug, Default, From, Deserialize, Serialize)]
pub struct CompactionList {
    ids: HashMap<LogId, CompactionAction>,
    migration_list: Vec<LogPointer>,
}

#[derive(Clone, Debug, From, Eq, PartialEq, Deserialize, Serialize)]
pub enum CompactionAction {
    Migrate,
    Remove,
}

#[derive(Constructor, Clone, Debug, Default, From, Deserialize, Serialize)]
pub struct LogPointer {
    id: LogId,
    offset: LogOffset,
}

type LogPointerIndex = DashMap<String, LogPointer>;
type LogId = u64;
type LogOffset = u64;
type LogSize = u64;

#[derive(Clone, Debug, Default, From, Eq, PartialEq, Deserialize, Serialize)]
pub enum LogIndexState {
    Compacting,
    #[default]
    Ready,
}

#[derive(Clone)]
pub struct LogMetadata {
    active_log_id: LogId,
    size: LogSize,
    path: PathBuf,
    ids: Vec<LogId>,
    eligible_for_compaction: CompactionList,
    state: LogIndexState,
}

#[derive(Clone)]
pub struct LogIndex {
    database: Arc<LogPointerIndex>,
    reader: Arc<DashMap<LogId, BufReader<File>>>,
    writer: Arc<RwLock<BufWriter<File>>>,
    metadata: Arc<RwLock<LogMetadata>>,
}

impl LogIndex {
    fn new(path: PathBuf) -> Result<LogIndex> {
        let ids = Self::get_file_log_ids(&path)?;
        let mut id = 0;
        let reader = Arc::new(DashMap::new());
        for log_id in &ids {
            let buf_reader = Self::log_reader(&path, *log_id)?;
            reader.insert(*log_id, buf_reader);
            id = *log_id;
        }
        let compaction_list = CompactionList::default();
        let writer = Arc::new(RwLock::new(Self::log_writer(&path, id)?));
        let size = 0;

        Ok(LogIndex {
            database: Arc::default(),
            reader,
            writer,
            metadata: Arc::new(RwLock::new(LogMetadata {
                active_log_id: id,
                size,
                path,
                ids,
                eligible_for_compaction: compaction_list,
                state: LogIndexState::default(),
            })),
        })
    }

    fn log_reader(path: &Path, id: LogId) -> Result<BufReader<File>> {
        let file = Self::get_log_file(path, id);
        if !file.exists() {
            File::create(&file)?;
        }
        new_reader(&file)
    }

    fn log_writer(path: &Path, id: LogId) -> Result<BufWriter<File>> {
        let file = Self::get_log_file(path, id);
        new_writer(file)
    }

    fn get_log_file(path: &Path, log_id: LogId) -> PathBuf {
        path.join(format!("{log_id}"))
    }

    fn replay_log(self) -> Result<Self> {
        let mut id = 0;
        let mut size = 0;
        let mut sorted_log_ids = self.reader.iter().map(|f| *f.key()).collect::<Vec<_>>();
        sorted_log_ids.sort_unstable();
        for log_id in sorted_log_ids {
            let mut record = self
                .reader
                .get_mut(&log_id)
                .expect("Unable to fetch reader by Id");
            let mut reader = &mut *record;
            let mut offset = 0;
            id = log_id;
            let file_info = reader.get_ref().metadata()?;
            while offset < file_info.size() {
                let command = Command::deserialize_from_reader(&mut reader)?;
                let log_pointer = LogPointer::new(id, offset);
                size = log_pointer.offset;
                Self::update_log_index(&self.database, command, log_pointer);
                offset = reader.stream_position()?;
                self.writer.write()?.seek(SeekFrom::Start(offset))?;
            }
        }
        (self.metadata.write()?).active_log_id = id;
        (self.metadata.write()?).size = size;
        Ok(self)
    }

    fn update_log_index(index: &LogPointerIndex, command: Command, log_pointer: LogPointer) {
        match command {
            Command::Set(cmd) => {
                index.insert(cmd.key, log_pointer);
            }
            Command::Rm(cmd) => {
                index.remove(&cmd.key);
            }
            Command::Get(_) => {}
        };
    }

    fn log_command(&self, command: Command) -> Result<()> {
        self.try_log_rotate()?;
        let mut writer = self.writer.write()?;
        let log_offset = command.serialize_into_writer(&mut *writer)?;
        self.metadata.write()?.size = writer.stream_position()?;
        drop(writer);
        let log_pointer = LogPointer::new(self.metadata.read()?.active_log_id, log_offset);
        Self::update_log_index(&self.database, command, log_pointer);
        Ok(())
    }

    fn try_log_rotate(&self) -> Result<bool> {
        if self.metadata.read()?.state == LogIndexState::Compacting {
            return Ok(false);
        }
        let mut rotated = false;
        let log_rotation_min_size =
            LOG_ROTATION_MIN_SIZE_BYTES.get_or_init(|| LOG_ROTATION_MIN_SIZE_BYTES_DEFAULT);
        let mut metadata = self.metadata.write()?;
        metadata.size = self.writer.write()?.stream_position()?;
        if metadata.size > *log_rotation_min_size {
            metadata.size = 0;
            metadata.active_log_id += 1;
            let log_id = metadata.active_log_id;
            metadata.ids.push(log_id);
            *self.writer.write()? = Self::log_writer(&metadata.path, metadata.active_log_id)?;

            self.reader.insert(
                metadata.active_log_id,
                Self::log_reader(&metadata.path, metadata.active_log_id)?,
            );
            drop(metadata);
            self.try_compacting_logs()?;
            rotated = true;
        }
        Ok(rotated)
    }

    fn get_value(&self, key: &str) -> Result<Option<String>> {
        let log_pointer = self.database.get(key);
        let mut value = None;
        if let Some(pointer) = &log_pointer {
            if let Some(command) = self.get_command(pointer)? {
                value = command.value().cloned();
            }
        }
        Ok(value)
    }

    fn get_command(&self, pointer: &LogPointer) -> Result<Option<Command>> {
        if let Some(reader) = self.reader.get_mut(&pointer.id).as_deref_mut() {
            reader.seek(SeekFrom::Start(pointer.offset))?;
            Ok(Some(Command::deserialize_from_reader(reader)?))
        } else {
            Ok(None)
        }
    }

    fn get_file_log_ids(path: &Path) -> Result<Vec<LogId>> {
        let path = path.join("[0-9]*");
        let path = path.to_str().ok_or_else(|| LogIndexIDError)?;
        let mut log_ids = glob::glob(path)?
            .map(|path| {
                let filename = path
                    .iter()
                    .map(|path| path.file_name().unwrap_or_default().to_owned())
                    .map(|s| s.to_str().unwrap_or_default().to_owned())
                    .collect::<String>();
                str::parse::<u64>(&filename).map_err(Into::into)
            })
            .collect::<Result<Vec<_>>>()?;
        if log_ids.is_empty() {
            log_ids = vec![0];
        } else {
            log_ids.sort_unstable();
        }
        Ok(log_ids)
    }

    fn try_compacting_logs(&self) -> Result<()> {
        self.metadata.write()?.state = LogIndexState::Compacting;
        self.identify_logs_that_can_be_compacted()?;
        self.try_migrating_infrequently_accessed_keys()?;
        self.try_removing_stale_logs()?;
        self.metadata.write()?.state = LogIndexState::Ready;
        Ok(())
    }

    fn identify_logs_that_can_be_compacted(&self) -> Result<()> {
        let mut total_records_per_log_id = HashMap::<LogId, Vec<LogPointer>>::new();
        for record in &*self.database {
            let log_pointer = record.value();
            total_records_per_log_id
                .entry(log_pointer.id)
                .and_modify(|log_pointers| log_pointers.push(log_pointer.clone()))
                .or_insert(vec![log_pointer.clone()]);
        }

        let max_records_in_any_log = total_records_per_log_id
            .values()
            .max_by(|x, y| x.len().cmp(&y.len()))
            .cloned()
            .unwrap_or(vec![LogPointer::default()]);
        let mut metadata = self.metadata.write()?;
        let mut migration_list = Vec::new();
        let mut eligible_ids = HashMap::new();
        for log_file_id in &metadata.ids {
            let active_id = total_records_per_log_id.get(log_file_id);
            if let Some(total_entries_in_this_log) = active_id {
                let log_id_percent =
                    (total_entries_in_this_log.len() * 100) / max_records_in_any_log.len();
                if log_id_percent as u64 <= LOG_COMPACTION_MAX_KEY_DENSITY_PERCENT {
                    // Mark this log as one that has entries that need migrating
                    eligible_ids.insert(*log_file_id, CompactionAction::Migrate);
                    // Save the list of log entries that need to be migrated
                    migration_list.extend(total_entries_in_this_log.clone());
                }
            } else if log_file_id != &metadata.active_log_id {
                // Mark this log as one that can be deleted
                eligible_ids.insert(*log_file_id, CompactionAction::Remove);
                continue;
            }
        }
        metadata.eligible_for_compaction.ids.extend(eligible_ids);
        metadata
            .eligible_for_compaction
            .migration_list
            .extend(migration_list);
        Ok(())
    }

    pub fn try_migrating_infrequently_accessed_keys(&self) -> Result<()> {
        let mut metadata = self.metadata.write()?;
        if metadata.eligible_for_compaction.migration_list.is_empty() {
            return Ok(());
        }
        while let Some(log_pointer) = metadata.eligible_for_compaction.migration_list.pop() {
            let command = self.get_command(&log_pointer.clone())?;
            if let Some(command) = command {
                self.log_command(command.clone())?;
            }
        }

        for (_, action) in metadata
            .eligible_for_compaction
            .ids
            .iter_mut()
            .filter(|(_, action)| **action == CompactionAction::Migrate)
        {
            *action = CompactionAction::Remove;
        }

        Ok(())
    }

    fn try_removing_stale_logs(&self) -> Result<()> {
        let metadata = self.metadata.write()?;
        for (log_id, _) in metadata
            .eligible_for_compaction
            .ids
            .iter()
            .filter(|(_, action)| **action == CompactionAction::Remove)
        {
            let file = Self::get_log_file(&metadata.path, *log_id);
            if file.exists() && file.is_file() {
                fs::remove_file(file)?;
            }
        }
        Ok(())
    }
}

/// Contains the in-memory index and
#[derive(Constructor, Clone)]
pub struct KvStore {
    index: Arc<LogIndex>,
    /// Use to selectively lock write operations, without locking the index
    write_lock: Arc<Mutex<()>>,
}

impl KvsEngine for KvStore {
    /// Open the `KvStore` at a given path and return the `KvStore`.
    ///
    /// # Errors
    ///
    /// If there was a problem opening the `KvStore`.
    fn open(path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();
        let index = Arc::new(LogIndex::new(path)?.replay_log()?);
        let write_lock = Arc::new(Mutex::new(()));
        Ok(KvStore::new(index, write_lock))
    }

    fn get(&self, key: String) -> Result<Option<String>> {
        self.index.get_value(&key)
    }

    fn remove(&self, key: String) -> Result<()> {
        if self.index.database.contains_key(&key) {
            let command = Command::from(Remove::new(key));
            let _write_lock = self.write_lock.lock();
            self.index.log_command(command)
        } else {
            Err(KeyNotFound)?
        }
    }

    fn set(&self, key: String, value: String) -> Result<()> {
        let command = Command::from(Set::new(key, value));
        let _write_lock = self.write_lock.lock();
        self.index.log_command(command)
    }
}
