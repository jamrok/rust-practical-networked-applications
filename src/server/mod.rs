use crate::{
    serde::BincodeSerde,
    shared::{initialize_log_directory, Command},
    KvsEngine,
    KvsError::WrongEngine,
    Result,
};
use derive_more::Constructor;
use std::{
    any::type_name,
    env::current_dir,
    fs,
    net::{SocketAddr, TcpListener, TcpStream},
};
use tracing::{debug, error, info};

#[derive(Constructor, Clone, Debug)]
pub struct KvsServer<Engine: KvsEngine> {
    address: SocketAddr,
    engine: Engine,
}

impl<Engine: KvsEngine> KvsServer<Engine> {
    pub fn start(&mut self) -> anyhow::Result<()> {
        let listener = TcpListener::bind(self.address)?;
        let engine = type_name::<Engine>();
        Self::check_or_save_engine(engine)?;
        info!("Now accepting connections");
        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    if let Err(e) = self.handle_request(stream) {
                        error!("{}", e)
                    }
                }
                Err(_e) => { /* connection failed */ }
            }
        }
        Ok(())
    }

    fn check_or_save_engine(engine: &str) -> Result<()> {
        let file = initialize_log_directory(current_dir()?)?.join("engine");
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

    pub fn handle_request(&mut self, stream: TcpStream) -> anyhow::Result<()> {
        let client = stream.peer_addr()?;
        debug!("Accepted request from {}", client);
        let command = Command::deserialize_from_stream(&stream)?;
        debug!("{:?} ", command);
        let response = command.process(&mut self.engine);
        response.serialize_into_stream(&stream)?;
        debug!("{:?} ", response);
        Ok(())
    }
}
