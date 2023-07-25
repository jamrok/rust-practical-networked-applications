use anyhow::Result;
use clap::{Args, Parser, ValueEnum};
use kvs::{
    server,
    server::KvsServer,
    shared::initialize_log_directory,
    thread_pool::{SharedQueueThreadPool, ThreadPool},
    KvStore, KvsEngine, SledKvsEngine,
};
use std::{
    env::current_dir,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::Path,
};
use strum::Display;

const DEFAULT_SERVER_IP: IpAddr = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
const DEFAULT_SERVER_PORT: u16 = 4000;

#[derive(Parser)]
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

    server::initialize_event_logging();

    let path = initialize_log_directory(&current_dir()?)?;
    match cli.options.engine {
        Engine::Kvs => {
            let kv = KvStore::open(&path)?;
            start_kvs_server(cli.options.address, kv, &path)
        }
        Engine::Sled => {
            let kv = SledKvsEngine::open(&path)?;
            start_kvs_server(cli.options.address, kv, &path)
        }
    }?;
    Ok(())
}

pub fn start_kvs_server<Engine: KvsEngine>(
    address: SocketAddr,
    engine: Engine,
    path: &Path,
) -> anyhow::Result<()> {
    let cpus = num_cpus::get();
    let pool = SharedQueueThreadPool::new(cpus as u32)?;
    let server = KvsServer::new(address, engine, pool, path);
    server.start(1)?;
    Ok(())
}
