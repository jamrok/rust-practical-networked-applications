use anyhow::Result;
use clap::{Args, Parser, ValueEnum};
use kvs::{server::KvsServer, shared::LOG_DIRECTORY_PREFIX, KvStore, KvsEngine, SledKvsEngine};
use std::{
    env::current_dir,
    fmt::Display,
    io,
    net::{IpAddr, Ipv4Addr, SocketAddr},
};
use strum::Display;
use tracing::{debug, info};
use tracing_subscriber::{fmt, prelude::*, Registry};

const DEFAULT_SERVER_IP: IpAddr = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
const DEFAULT_SERVER_PORT: u16 = 4000;

#[derive(Parser, Clone, Debug)]
// Inherit cargo package defaults for author, version, etc
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(flatten)]
    options: CommandOptions,
}

#[derive(ValueEnum, Clone, Debug, Default, Display)]
#[strum(serialize_all = "lowercase")]
pub enum Engine {
    #[default]
    Kvs,
    Sled,
}

#[derive(Args, Clone, Debug)]
struct CommandOptions {
    #[arg(
        default_value_t = CommandOptions::default().address,
        global = true,
        help = "Sets the IP and PORT to listen on.",
        long = "addr",
        name = "IP:PORT",
    )]
    address: SocketAddr,

    #[arg(
        value_enum,
        long,
        default_value_t = CommandOptions::default().engine,
        help = "Sets the Engine to be used."
    )]
    engine: Engine,
}

impl Default for CommandOptions {
    fn default() -> Self {
        Self {
            address: SocketAddr::new(DEFAULT_SERVER_IP, DEFAULT_SERVER_PORT),
            engine: Engine::default(),
        }
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    Registry::default()
        .with(fmt::Layer::default().with_writer(io::stderr))
        .init();

    startup_logging(cli.options.address, &cli.options.engine);
    match cli.options.engine {
        Engine::Kvs => {
            let kv = KvStore::open(current_dir()?)?;
            // let kv = KvStore::open(current_dir()?)?;
            start_kvs_server(cli.options.address, kv)
        }
        Engine::Sled => {
            let kv = SledKvsEngine::new(current_dir()?.join(LOG_DIRECTORY_PREFIX))?;
            start_kvs_server(cli.options.address, kv)
        }
    }?;
    Ok(())
}

pub fn start_kvs_server<Engine: KvsEngine>(addr: SocketAddr, engine: Engine) -> Result<()> {
    let mut server = KvsServer::new(addr, engine);
    server.start()?;
    Ok(())
}

fn startup_logging(address: impl Display, engine: impl Display) {
    info!("Starting KVS Server Version {}.", env!("CARGO_PKG_VERSION"));
    info!("Using {} engine, listening on {}", engine, address);
    debug!(
        "Log level: {}",
        std::env::var("RUST_LOG").unwrap_or_default()
    )
}
