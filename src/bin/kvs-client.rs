use anyhow::Result;
use clap::{Args, Parser};
use kvs::{client::KvsClient, shared::Command};
use std::{
    fmt::Debug,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    process::exit,
};

const DEFAULT_SERVER_IP: IpAddr = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
const DEFAULT_SERVER_PORT: u16 = 4000;

#[derive(Parser)]
// Inherit cargo package defaults for author, version, etc
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Lists all available SubCommands
    #[command(subcommand)]
    command: Command,
    #[command(flatten)]
    options: CommandOptions,
}

#[derive(Args, Clone, Debug)]
struct CommandOptions {
    #[arg(
        default_value_t = CommandOptions::default().addr,
        global = true,
        help = "Sets the IP and PORT to connect to.",
        long,
        name = "IP:PORT",
    )]
    addr: SocketAddr,
}

impl Default for CommandOptions {
    fn default() -> Self {
        Self {
            addr: SocketAddr::new(DEFAULT_SERVER_IP, DEFAULT_SERVER_PORT),
        }
    }
}

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let cli = Cli::parse();
    match KvsClient::new(cli.options.addr).send_command(&cli.command) {
        Ok(response) => {
            print!("{}", response);
        }
        Err(error) => {
            eprint!("{}", error);
            exit(1)
        }
    };
    Ok(())
}
