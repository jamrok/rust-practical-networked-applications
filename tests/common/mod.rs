use crossbeam_utils::Backoff;
use fake::Fake;
use kvs::{
    server::KvsServer,
    shared::{Command, Set},
    thread_pool::ThreadPool,
    KvsEngine,
};
use std::{collections::HashMap, net::SocketAddr, ops::Deref, sync::Arc, thread};
use tempfile::TempDir;
use tracing::debug;

pub fn generate_write_commands(
    quantity: usize,
    max_length: usize,
    word_length: WordLength,
) -> (SampleData, SampleWriteCommandsVec) {
    let mut list = SampleData::new();
    let mut list_vec = SampleWriteCommandsVec::new();

    let min_length = match word_length {
        WordLength::Fixed => max_length,
        WordLength::Random => 1,
    };

    for i in 1..=quantity {
        let key = format!("key{}_{}", i, (min_length..=max_length).fake::<String>());
        let value = format!("value{}_{}", i, (min_length..=max_length).fake::<String>());
        list.insert(key.clone(), value.clone());
        let command = Command::from(Set::new(key, value));
        list_vec.push(command);
    }
    (list, list_vec)
}

pub type SampleData = HashMap<String, String>;
pub type SampleWriteCommandsVec = Vec<Command>;

pub enum WordLength {
    Fixed,
    #[allow(dead_code)]
    Random,
}

pub struct TestKvsServer<Engine, Pool>
where
    Engine: KvsEngine,
    Pool: ThreadPool,
{
    #[allow(dead_code)]
    temp_dir: Arc<TempDir>,
    server: Arc<KvsServer<Engine, Pool>>,
}

impl<Engine, Pool> Deref for TestKvsServer<Engine, Pool>
where
    Engine: KvsEngine,
    Pool: ThreadPool,
{
    type Target = Arc<KvsServer<Engine, Pool>>;

    fn deref(&self) -> &Self::Target {
        &self.server
    }
}

impl<Engine, Pool> TestKvsServer<Engine, Pool>
where
    Engine: KvsEngine,
    Pool: ThreadPool,
{
    pub fn new(address: SocketAddr, cpus: Option<usize>) -> Self {
        let temp_dir = Arc::new(TempDir::new().unwrap());
        let engine = Engine::open(temp_dir.path()).unwrap();
        let cpus = cpus.unwrap_or(num_cpus::get());
        let pool = Pool::new(cpus as u32).unwrap();

        let server = Arc::new(KvsServer::new(address, engine, pool, temp_dir.path()));
        Self { server, temp_dir }
    }

    pub fn wait_until_ready(&self) {
        debug!("Waiting on server to be ready");
        let backoff = Backoff::new();
        while !self.server.is_ready() {
            backoff.snooze();
        }
        debug!("Server is ready");
    }

    pub fn wait_until_shutdown(&self) {
        debug!("Waiting on server to shutdown");
        let backoff = Backoff::new();
        while !self.server.is_shutdown() {
            backoff.snooze();
        }
        debug!("Server has shutdown");
    }

    pub fn spawn(self, num_listeners: usize) -> Arc<Self>
    where
        Engine: KvsEngine,
        Pool: ThreadPool,
    {
        let server = Arc::new(self);
        let spawned_server = server.clone();
        thread::spawn(move || {
            debug!("Spawning new server");
            let _ = spawned_server.start(num_listeners);
        });
        server
    }
}
