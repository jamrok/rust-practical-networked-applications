mod spawned_listener;

use crate::{
    serde::bincode::Serde, server::spawned_listener::SpawnedListener, shared::Command,
    thread_pool::ThreadPool, KvsEngine, KvsError::WrongEngine, Result,
};
use crossbeam::{
    channel,
    channel::{Receiver, Sender},
};
use std::{
    any::type_name,
    fmt::Display,
    fs, io,
    net::{SocketAddr, TcpStream},
    path::{Path, PathBuf},
    str::FromStr,
    sync::{Arc, Once, RwLock},
};
use tracing::{debug, error, info, Level};
use tracing_subscriber::{
    fmt, fmt::writer::MakeWriterExt, layer::SubscriberExt, util::SubscriberInitExt, Registry,
};

static INIT_LOGGING: Once = Once::new();

pub enum Message {
    Stream(TcpStream),
    Ready,
    ShuttingDown,
    Shutdown,
}

impl From<TcpStream> for Message {
    fn from(stream: TcpStream) -> Self {
        Self::Stream(stream)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum State {
    Starting,
    Ready,
    Shutdown,
}

#[allow(clippy::module_name_repetitions)]
#[derive(Clone, Debug)]
pub struct KvsServer<Engine: KvsEngine, Pool: ThreadPool> {
    address: SocketAddr,
    engine: Engine,
    pool: Pool,
    path: PathBuf,
    state: Arc<RwLock<State>>,
    pub sender: Sender<Message>,
    pub receiver: Receiver<Message>,
}

impl<Engine: KvsEngine, Pool: ThreadPool> KvsServer<Engine, Pool> {
    pub fn new(address: SocketAddr, engine: Engine, pool: Pool, path: &Path) -> Self {
        let (sender, receiver) = channel::unbounded::<Message>();
        let state = Arc::new(RwLock::new(State::Starting));
        Self {
            address,
            engine,
            path: path.to_owned(),
            state,
            pool,
            sender,
            receiver,
        }
    }

    pub fn is_ready(&self) -> bool {
        self.state
            .read()
            .map(|state| *state == State::Ready)
            .unwrap_or_default()
    }

    pub fn is_shutdown(&self) -> bool {
        self.state
            .read()
            .map(|state| *state == State::Shutdown)
            .unwrap_or_default()
    }

    pub fn shutdown(&self) {
        self.sender
            .send(Message::ShuttingDown)
            .expect("Failed to initiate shutdown.");
    }

    pub fn set_state(&self, new_state: State) -> Result<()> {
        *self.state.write()? = new_state;
        Ok(())
    }

    pub fn start(&self, num_listeners: usize) -> anyhow::Result<()> {
        let engine: Vec<&str> = type_name::<Engine>().split("::").collect();
        let engine = engine.get(2).expect("Unable to parse engine.");

        startup_logging(self.address, engine);
        self.check_or_save_engine(engine)?;
        let (tx, rx) = (self.sender.clone(), self.receiver.clone());
        let spawned_listener =
            SpawnedListener::<Message>::new(num_listeners, self.address, tx.clone()).bind();
        tx.send(Message::Ready)
            .expect("Unable to switch to 'Ready' state.");

        self.process_messages(&rx, &spawned_listener);
        Ok(())
    }

    fn process_messages(
        &self,
        rx: &Receiver<Message>,
        spawned_listener: &Arc<SpawnedListener<Message>>,
    ) {
        loop {
            let next_message = rx.recv();
            match next_message {
                Ok(message) => match message {
                    Message::Stream(stream) => {
                        let engine = self.engine.clone();
                        self.pool.spawn(move || {
                            if let Err(e) = process_stream(&engine, &stream) {
                                error!("Error processing stream: {:?}", e);
                            }
                        });
                    }
                    Message::ShuttingDown => {
                        info!("Shutdown signal received: Shutting down server.");
                        spawned_listener.shutdown();
                        self.sender
                            .send(Message::Shutdown)
                            .expect("Unable to send 'Shutdown' message");
                    }
                    Message::Shutdown => {
                        info!("KVS Server Shutdown");
                        self.set_state(State::Shutdown)
                            .expect("Unable to switch to 'Shutdown' state");
                        break;
                    }
                    Message::Ready => {
                        self.set_state(State::Ready)
                            .expect("Unable to switch to 'Ready' state");
                    }
                },
                Err(err) => {
                    error!("Unexpected thread pool error: {}", err);
                }
            }
        }
    }

    fn check_or_save_engine(&self, engine: &str) -> Result<()> {
        let file = self.path.join("engine");
        if !file.exists() {
            fs::write(file.clone(), engine)?;
        }
        let detected_engine = fs::read_to_string(file)?;
        debug!("Detected Engine: {}", detected_engine);
        if engine != detected_engine {
            return Err(WrongEngine);
        }
        Ok(())
    }
}

pub fn process_stream<Engine: KvsEngine>(
    engine: &Engine,
    stream: &TcpStream,
) -> anyhow::Result<()> {
    let command = Command::deserialize_from_stream(stream)?;
    let response = command.process(engine);
    response.serialize_into_stream(stream)?;
    Ok(())
}

fn startup_logging(address: impl Display, engine: impl Display) {
    info!("Starting KVS Server Version {}.", env!("CARGO_PKG_VERSION"));
    info!("Using {} engine, listening on {}", engine, address);
    let _ = std::env::var("RUST_LOG").map(|log_level| debug!("Log level: {}", log_level));
}

pub fn initialize_event_logging() {
    let level = std::env::var("RUST_LOG").unwrap_or_default();
    let level = Level::from_str(&level).unwrap_or(Level::INFO);
    INIT_LOGGING.call_once(|| {
        Registry::default()
            .with(fmt::Layer::default().with_writer(io::stderr.with_max_level(level)))
            .init();
    });
}
