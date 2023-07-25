use crossbeam::channel::{unbounded, Receiver, Sender};
use crossbeam_utils::Backoff;
use std::{
    net::{SocketAddr, TcpListener, TcpStream},
    sync::Arc,
};
use tracing::{debug, error, info};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ListenerState {
    Initialized,
    Shutdown,
}

/// TCP Listener that can be spawned and shutdown gracefully from the parent thread
#[derive(Clone, Debug)]
pub struct SpawnedListener<T>
where
    T: Send + From<TcpStream> + 'static,
{
    address: SocketAddr,
    cpus: usize,
    stream_sender: Sender<T>,
    state_sender: Sender<ListenerState>,
    state_receiver: Receiver<ListenerState>,
}

impl<T> SpawnedListener<T>
where
    T: Send + From<TcpStream> + 'static,
{
    pub fn new(cpus: usize, address: SocketAddr, stream_sender: Sender<T>) -> Self {
        let (state_sender, state_receiver) = unbounded();
        // At least 1 listener must be spawned
        let cpus = cpus.max(1);
        Self {
            address,
            cpus,
            stream_sender,
            state_sender,
            state_receiver,
        }
    }

    pub fn bind(self) -> Arc<SpawnedListener<T>> {
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(self.cpus)
            .build()
            .expect("Unable to create thread pool for TCP listener");

        self.state_sender
            .send(ListenerState::Initialized)
            .expect("Unable to send 'Initialized' message");
        let spawned_listener = Arc::new(self);
        let tcp_listener = TcpListener::bind(spawned_listener.address).unwrap_or_else(|err| {
            panic!(
                "Unable to bind to address '{}': {}",
                spawned_listener.address, err
            )
        });
        for id in 1..=spawned_listener.cpus {
            let spawned = spawned_listener.clone();
            debug!("Starting TCP listener #{}", id);
            let tcp_listener = tcp_listener.try_clone().expect("Unable to clone listener");
            pool.spawn(move || {
                for stream in tcp_listener.incoming() {
                    if let Ok(state) = spawned.state_receiver.try_recv() {
                        match state {
                            ListenerState::Initialized => continue,
                            ListenerState::Shutdown => {
                                debug!("Shutting down TCP listener #{:?}", id);
                                break;
                            }
                        }
                    }
                    match stream {
                        Ok(stream) => {
                            if let Err(err) = spawned.stream_sender.send(stream.into()) {
                                error!("Stream sender error: {}", err);
                            }
                        }
                        Err(err) => {
                            error!("Stream error: {}", err);
                        }
                    }
                }
            });
        }
        // Wait on TCP listener to be available
        let backoff = Backoff::new();
        while let Err(_error) = TcpStream::connect(spawned_listener.address) {
            backoff.snooze();
        }

        info!("Now accepting connections.");
        spawned_listener
    }

    pub fn shutdown(&self) {
        for _ in 0..self.cpus {
            self.state_sender
                .send(ListenerState::Shutdown)
                .expect("Unable to send TCP shutdown message");
        }
        // Wait on TCP listener to shutdown
        let backoff = Backoff::new();
        while let Ok(_stream) = TcpStream::connect(self.address) {
            backoff.snooze();
        }
        info!("TCP listener has been shutdown");
    }
}
