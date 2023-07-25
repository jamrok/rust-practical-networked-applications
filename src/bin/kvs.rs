use clap::{Args, Parser, Subcommand};
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

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Set(_) => panic!("unimplemented"),
        Commands::Get(_) => panic!("unimplemented"),
        Commands::Rm(_) => panic!("unimplemented"),
    }
}
