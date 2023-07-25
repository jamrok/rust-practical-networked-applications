use anyhow::Result;
use clap::{Args, Parser, Subcommand};
use kvs::{
    KvStore,
    KvsErrors::{EmptyResponse, KeyNotFound},
};
use std::env::current_dir;

#[derive(Parser)]
// Inherit cargo package defaults for author, version, etc
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Lists all available SubCommands
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]

enum Commands {
    /// Save the given string value to the given string key
    Set(SetArgs),
    /// Get the string value of a given string key
    Get(GetArgs),
    /// Remove the given string key
    Rm(RmArgs),
}

type Key = String;
type Value = String;

#[derive(Args)]
struct SetArgs {
    key: Key,
    value: Value,
}

#[derive(Args)]
struct GetArgs {
    key: Key,
}

#[derive(Args)]
struct RmArgs {
    key: Key,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let dir = current_dir()?;
    let mut kv = KvStore::open(dir)?;
    let result = match &cli.command {
        Commands::Get(GetArgs { key }) => kv
            .get(key.into())?
            .unwrap_or_else(|| KeyNotFound.to_string()),
        Commands::Set(SetArgs { key, value }) => kv
            .set(key.into(), value.into())
            .map(|_| EmptyResponse.to_string())?,
        Commands::Rm(RmArgs { key }) => kv.remove(key.into()).map(|_| EmptyResponse.to_string())?,
    };
    print!("{}", result);
    Ok(())
}
