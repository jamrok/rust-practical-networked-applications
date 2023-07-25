use crate::{
    serde::BincodeSerde,
    KvsEngine,
    KvsError::{BufReaderError, KeyNotFound},
    Result,
};
use clap::{Args, Subcommand};
use derive_more::{Constructor, From};
use serde::{Deserialize, Serialize};
use std::{
    fmt::{Display, Formatter},
    fs,
    fs::{File, OpenOptions},
    io::{BufReader, BufWriter},
    path::PathBuf,
};

// TODO: move to config files
pub const LOG_DIRECTORY_PREFIX: &str = "log_index";
pub const LOG_ROTATION_MIN_SIZE_BYTES: u64 = 256 * 1024;
pub const LOG_COMPACTION_MAX_KEY_DENSITY_PERCENT: u64 = 30;

#[derive(Clone, Debug, From, Serialize, Deserialize)]
pub enum CommandResponse {
    ResultWithNoResponse(ResultWithNoResponse),
    ResultWithPossibleValue(ResultWithPossibleValue),
}

impl CommandResponse {
    pub fn is_err(&self) -> bool {
        matches!(
            *self,
            Self::ResultWithNoResponse(ResultWithNoResponse::Err(_))
                | Self::ResultWithPossibleValue(ResultWithPossibleValue::Err(_))
        )
    }
}

impl BincodeSerde for CommandResponse {}

impl Display for CommandResponse {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            CommandResponse::ResultWithNoResponse(e) => e.to_string(),
            CommandResponse::ResultWithPossibleValue(val) => val.to_string(),
        };
        write!(f, "{}", value)
    }
}

#[derive(Clone, Debug, From, Serialize, Deserialize)]
pub enum ResultWithPossibleValue {
    Ok(Option<String>),
    Err(String),
}

impl Display for ResultWithPossibleValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            ResultWithPossibleValue::Ok(ok) => ok.clone().unwrap_or_default(),
            ResultWithPossibleValue::Err(e) => e.into(),
        };
        if value.is_empty() {
            write!(f, "{}", value)
        } else {
            writeln!(f, "{}", value)
        }
    }
}

#[derive(Clone, Debug, From, Serialize, Deserialize)]
pub enum ResultWithNoResponse {
    Ok(()),
    Err(String),
}

impl Display for ResultWithNoResponse {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            ResultWithNoResponse::Ok(()) => "".to_string(),
            ResultWithNoResponse::Err(e) => format!("{}\n", e),
        };
        write!(f, "{}", value)
    }
}

impl From<Result<Option<String>>> for CommandResponse {
    fn from(value: Result<Option<String>>) -> Self {
        match value {
            Ok(val) => ResultWithPossibleValue::Ok(val).into(),
            Err(err) => ResultWithPossibleValue::Err(err.to_string()).into(),
        }
    }
}

impl From<Result<()>> for CommandResponse {
    fn from(value: Result<()>) -> Self {
        match value {
            Ok(_) => ResultWithNoResponse::Ok(()).into(),
            Err(err) => ResultWithNoResponse::Err(err.to_string()).into(),
        }
    }
}

pub fn initialize_log_directory(path: PathBuf) -> Result<PathBuf> {
    let path = path.join(LOG_DIRECTORY_PREFIX);
    fs::create_dir_all(&path)?;
    Ok(path)
}

pub fn new_reader(file: &PathBuf) -> Result<BufReader<File>> {
    Ok(BufReader::new(
        OpenOptions::new().read(true).open(file).map_err(|e| {
            BufReaderError(
                format!("Error opening file {}", file.display().to_string()),
                e,
            )
        })?,
    ))
}

pub fn new_writer(file: PathBuf) -> Result<BufWriter<File>> {
    Ok(BufWriter::new(
        OpenOptions::new().create(true).append(true).open(file)?,
    ))
}

#[derive(Subcommand, Clone, Debug, From, Deserialize, Serialize)]
pub enum Command {
    /// Save the given string value to the given string key
    Set(Set),
    /// Get the string value of a given string key
    Get(Get),
    /// Remove the given string key
    Rm(Remove),
}

type Key = String;
type Value = String;

#[derive(Args, Constructor, Clone, Debug, Default, From, Deserialize, Serialize)]
pub struct Set {
    pub key: Key,
    pub value: Value,
}

#[derive(Args, Clone, Debug, Serialize, Deserialize)]
pub struct Get {
    pub key: Key,
}

#[derive(Args, Constructor, Clone, Debug, Default, From, Deserialize, Serialize)]
pub struct Remove {
    pub key: Key,
}

impl Command {
    pub fn process<Engine: KvsEngine>(self, kv: &mut Engine) -> CommandResponse {
        match self {
            Command::Set(Set { key, value }) => kv.set(key, value).into(),
            Command::Get(Get { key }) => match kv.get(key) {
                Ok(data) => match data {
                    None => Err(KeyNotFound),
                    Some(e) => Ok(Some(e)),
                },
                Err(e) => Err(e),
            }
            .into(),
            Command::Rm(Remove { key }) => kv.remove(key).into(),
        }
    }

    pub fn value(&self) -> Option<&Value> {
        match self {
            Command::Set(cmd) => Some(&cmd.value),
            Command::Rm(_) | Command::Get(_) => None,
        }
    }
}

impl BincodeSerde for Command {}
