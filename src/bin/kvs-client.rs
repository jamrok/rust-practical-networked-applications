use anyhow::Result;
use clap::{Args, Parser};
use derive_more::Constructor;
use kvs::{
    serde::BincodeSerde,
    shared::{Command, CommandResponse},
};
use std::{
    fmt::Debug,
    net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream},
    process::exit,
    time::Duration,
};
use tracing::debug;

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
    KvsClient::new(cli.options.addr).send_command(&cli.command)?;
    Ok(())
}

#[derive(Constructor, Clone, Debug)]
pub struct KvsClient {
    server_address: SocketAddr,
}

impl KvsClient {
    fn send_command(&self, command: &Command) -> Result<()> {
        let timeout = Duration::from_secs(10);
        let stream = TcpStream::connect_timeout(&self.server_address, timeout)?;
        stream.set_read_timeout(Some(timeout))?;
        command.serialize_into_stream(&stream)?;
        let response = CommandResponse::deserialize_from_stream(&stream)?;
        debug!("Got: {:?}", &response);
        if response.is_err() {
            eprint!("Error: {}", response);
            exit(1)
        } else {
            print!("{}", response);
            Ok(())
        }
    }
}
