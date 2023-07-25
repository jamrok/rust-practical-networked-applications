use crate::{
    KvsEngine,
    KvsErrors::{BufReaderError, KeyNotFound, LogIndexIDError},
    Result,
};
use derive_more::{Constructor, From};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashMap},
    fs::{self, File, OpenOptions},
    io::{BufRead, BufReader, BufWriter, Seek, SeekFrom, Write},
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
};

// TODO: move to config files
pub const LOG_DIRECTORY_PREFIX: &str = "log_index";
pub const LOG_ROTATION_MIN_SIZE_BYTES: u64 = 256 * 1024;
pub const LOG_COMPACTION_MAX_KEY_DENSITY_PERCENT: u64 = 30;

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

type Key = String;
type Value = String;
type LogPointerIndex = HashMap<String, LogPointer>;
type LogId = u64;
type LogOffset = u64;
type LogSize = u64;

#[derive(Clone, Debug, Default, From, Eq, PartialEq, Deserialize, Serialize)]
pub enum LogIndexState {
    Compacting,
    #[default]
    Ready,
}

pub struct LogMetadata {
    active_log_id: LogId,
    size: LogSize,
    path: PathBuf,
    ids: Vec<LogId>,
    eligible_for_compaction: CompactionList,
    state: LogIndexState,
}

pub struct LogIndex {
    database: LogPointerIndex,
    reader: BTreeMap<LogId, BufReader<File>>,
    writer: BufWriter<File>,
    metadata: LogMetadata,
}

impl LogIndex {
    fn new(path: PathBuf) -> Result<LogIndex> {
        let path = Self::initialize_log_directory(path)?;
        let ids = Self::get_file_log_ids(&path)?;
        let mut id = 0;
        let mut reader = BTreeMap::new();
        for log_id in &ids {
            let buf_reader = Self::reader(&path, log_id)?;
            reader.insert(*log_id, buf_reader);
            id = *log_id;
        }
        let compaction_list = CompactionList::default();
        let writer = Self::writer(&path, &id)?;
        let size = 0;

        // dbg!(&path, &reader, &writer);
        Ok(LogIndex {
            database: Default::default(),
            reader,
            writer,
            metadata: LogMetadata {
                active_log_id: id,
                size,
                path,
                ids,
                eligible_for_compaction: compaction_list,
                state: Default::default(),
            },
        })
    }

    fn initialize_log_directory(path: PathBuf) -> Result<PathBuf> {
        let path = path.join(LOG_DIRECTORY_PREFIX);
        fs::create_dir_all(&path)?;
        Ok(path)
    }

    fn reader(path: &Path, id: &LogId) -> Result<BufReader<File>> {
        let file = Self::get_log_file(path, id)?;
        if !file.exists() {
            File::create(&file)?;
        }
        Ok(BufReader::new(
            OpenOptions::new().read(true).open(&file).map_err(|e| {
                BufReaderError(
                    format!("Error opening file {}", file.display().to_string()),
                    e,
                )
            })?,
        ))
    }

    fn writer(path: &Path, id: &LogId) -> Result<BufWriter<File>> {
        let file = Self::get_log_file(path, id)?;
        Ok(BufWriter::new(
            OpenOptions::new().create(true).append(true).open(file)?,
        ))
    }

    fn get_log_file(path: &Path, log_id: &LogId) -> Result<PathBuf> {
        Ok(path.join(format!("{}", log_id)))
    }

    fn replay_log(mut self) -> Result<Self> {
        let mut id = 0;
        let mut size = 0;
        for (log_id, mut reader) in self.reader.iter_mut() {
            let mut offset = 0;
            id = *log_id;
            let file_info = reader.get_ref().metadata()?;
            while offset < file_info.size() {
                let command = Command::deserialize_from(&mut reader)?;
                let log_pointer = LogPointer::new(id, offset);
                size = log_pointer.offset;
                // eprintln!(
                //     "{} | {} | {:?} | {:?}",
                //     &offset, &size, &log_pointer, &command
                // );
                Self::update_log_index(&mut self.database, command, log_pointer)?;
                offset = reader.stream_position()?;
                self.writer.seek(SeekFrom::Start(offset))?;
            }
        }
        self.metadata.active_log_id = id;
        self.metadata.size = size;
        Ok(self)
    }

    fn update_log_index(
        index: &mut LogPointerIndex,
        command: Command,
        log_pointer: LogPointer,
    ) -> Result<()> {
        match command {
            Command::Set(cmd) => {
                index.insert(cmd.key, log_pointer);
            }
            Command::Remove(cmd) => {
                index.remove(&cmd.key);
            }
        };
        Ok(())
    }

    fn log_command(&mut self, command: Command) -> Result<()> {
        self.try_log_rotate()?;
        let log_offset = &command.serialize_into(&mut self.writer)?;
        let log_pointer = LogPointer::new(self.metadata.active_log_id, *log_offset);
        self.metadata.size = self.writer.stream_position()?;
        // dbg!(&self.size);
        // eprintln!(
        //     "{} | {:?} | {:?}",
        //     &self.metadata.size, &log_pointer, &command
        // );

        Self::update_log_index(&mut self.database, command, log_pointer)?;
        Ok(())
    }

    fn try_log_rotate(&mut self) -> Result<bool> {
        // eprintln!("{} ", &self.metadata.size,);
        self.metadata.size = self.writer.stream_position()?;
        if self.metadata.state == LogIndexState::Compacting {
            return Ok(false);
        }
        let mut rotated = false;
        // dbg!(
        //     "log_rotate",
        //     self.metadata.size,
        //     LOG_ROTATION_MIN_SIZE_BYTES,
        //     &self.writer
        // );
        if self.metadata.size > LOG_ROTATION_MIN_SIZE_BYTES {
            self.metadata.size = 0;
            self.metadata.active_log_id += 1;
            self.metadata.ids.push(self.metadata.active_log_id);
            self.writer = Self::writer(&self.metadata.path, &self.metadata.active_log_id)?;
            self.reader.insert(
                self.metadata.active_log_id,
                Self::reader(&self.metadata.path, &self.metadata.active_log_id)?,
            );
            self.try_compacting_logs()?;
            rotated = true;
        }
        Ok(rotated)
    }

    fn get_value(&mut self, key: String) -> Result<Option<String>> {
        let log_pointer = self.database.get(&key).cloned();
        // dbg!("get", &key, &self.in_memory, &self.reader);
        let mut value = None;
        if let Some(pointer) = &log_pointer {
            if let Some(command) = self.get_command(pointer)? {
                value = command.value().cloned();
            }
        }
        Ok(value)
    }

    fn get_command(&mut self, pointer: &LogPointer) -> Result<Option<Command>> {
        if let Some(reader) = self.reader.get_mut(&pointer.id) {
            // dbg!(&reader.stream_position());
            reader.seek(SeekFrom::Start(pointer.offset))?;
            // dbg!(&reader.stream_position());
            Ok(Some(Command::deserialize_from(reader)?))
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
                str::parse::<u64>(&filename).map_err(|e| e.into())
            })
            .collect::<Result<Vec<_>>>()?;
        if log_ids.is_empty() {
            log_ids = vec![0];
        } else {
            log_ids.sort();
        }
        // dbg!(&log_ids);
        Ok(log_ids)
    }

    fn try_compacting_logs(&mut self) -> Result<()> {
        self.metadata.state = LogIndexState::Compacting;

        self.identify_logs_that_can_be_compacted();
        // dbg!(&self.metadata.compaction_list);
        // dbg!(&self.database);
        self.try_migrating_infrequently_accessed_keys()?;
        self.try_removing_stale_logs()?;
        // dbg!(&self.metadata.compaction_list);
        self.metadata.state = LogIndexState::Ready;
        Ok(())
    }

    fn identify_logs_that_can_be_compacted(&mut self) {
        let mut total_records_per_log_id = HashMap::<LogId, Vec<LogPointer>>::new();
        for log_pointer in self.database.values() {
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
        for log_file_id in &self.metadata.ids {
            let active_id = total_records_per_log_id.get_mut(log_file_id);
            if let Some(total_entries_in_this_log) = active_id {
                let log_id_percent =
                    (total_entries_in_this_log.len() * 100) / max_records_in_any_log.len();
                if log_id_percent as u64 <= LOG_COMPACTION_MAX_KEY_DENSITY_PERCENT {
                    // dbg!(
                    //     &log_file_id,
                    //     // &total_entries_in_this_log,
                    //     // &max_records_in_any_log,
                    //     &log_id_percent,
                    // );
                    // Mark this log as one that has entries that need migrating
                    self.metadata
                        .eligible_for_compaction
                        .ids
                        .insert(*log_file_id, CompactionAction::Migrate);
                    // Save the list of log entries that need to be migrated
                    self.metadata
                        .eligible_for_compaction
                        .migration_list
                        .extend(total_entries_in_this_log.clone())
                }
            } else if log_file_id != &self.metadata.active_log_id {
                // Mark this log as one that can be deleted
                self.metadata
                    .eligible_for_compaction
                    .ids
                    .insert(*log_file_id, CompactionAction::Remove);
                continue;
            }
        }
    }

    pub fn try_migrating_infrequently_accessed_keys(&mut self) -> Result<()> {
        if self
            .metadata
            .eligible_for_compaction
            .migration_list
            .is_empty()
        {
            return Ok(());
        }
        while let Some(log_pointer) = self.metadata.eligible_for_compaction.migration_list.pop() {
            let command = self.get_command(&log_pointer.clone())?;
            if let Some(command) = command {
                self.log_command(command.clone())?;
            }
        }

        for (_, action) in self
            .metadata
            .eligible_for_compaction
            .ids
            .iter_mut()
            .filter(|(_, action)| **action == CompactionAction::Migrate)
        {
            *action = CompactionAction::Remove;
        }

        Ok(())
    }

    fn try_removing_stale_logs(&mut self) -> Result<()> {
        for (log_id, _) in self
            .metadata
            .eligible_for_compaction
            .ids
            .iter()
            .filter(|(_, action)| **action == CompactionAction::Remove)
        {
            let file = Self::get_log_file(&self.metadata.path, log_id)?;
            if file.exists() && file.is_file() {
                // dbg!(&file);
                fs::remove_file(file)?;
            }
        }
        Ok(())
    }
}

/// Contains the in-memory index and
#[derive(Constructor)]
pub struct KvStore {
    index: LogIndex,
}

#[derive(Clone, Debug, From, Deserialize, Serialize)]
enum Command {
    /// Save the given string value to the given string key
    Set(Set),
    /// Remove the given string key
    Remove(Remove),
}

#[derive(Constructor, Clone, Debug, Default, From, Deserialize, Serialize)]
struct Set {
    key: Key,
    value: Value,
}

#[derive(Constructor, Clone, Debug, Default, From, Deserialize, Serialize)]
struct Remove {
    key: Key,
}

impl KvsEngine for KvStore {
    fn get(&mut self, key: String) -> Result<Option<String>> {
        self.index.get_value(key)
    }

    fn remove(&mut self, key: String) -> Result<()> {
        if self.index.database.contains_key(&key) {
            let command = Command::from(Remove::new(key));
            self.index.log_command(command)
        } else {
            Err(KeyNotFound)?
        }
    }

    fn set(&mut self, key: String, value: String) -> Result<()> {
        let command = Command::from(Set::new(key, value));
        self.index.log_command(command)
    }
}

impl KvStore {
    /// Open the KvStore at a given path and return the KvStore.
    ///
    /// # Errors
    ///
    /// If there was a problem opening the KvStore.
    pub fn open(path: impl Into<PathBuf>) -> Result<KvStore> {
        let path = path.into();
        let index = LogIndex::new(path)?.replay_log()?;
        Ok(KvStore::new(index))
    }
}

impl Command {
    fn value(&self) -> Option<&Value> {
        match self {
            Command::Set(cmd) => Some(&cmd.value),
            Command::Remove(_) => None,
        }
    }

    fn serialize_into<T: Write + Seek>(&self, mut writer: T) -> Result<u64> {
        let log_position = writer.stream_position()?;
        bincode::serialize_into(&mut writer, self)?;
        writer.flush()?;
        Ok(log_position)
    }

    fn deserialize_from<T: BufRead>(reader: T) -> Result<Self> {
        Ok(bincode::deserialize_from::<_, Self>(reader)?)
    }
}
