use crate::{
    serde::bincode::Serde,
    shared::{Command, CommandResponse},
    KvsError::GeneralError,
    Result,
};
use std::{
    net::{SocketAddr, TcpStream},
    time::Duration,
};

#[allow(clippy::module_name_repetitions)]
#[derive(Clone, Debug)]
pub struct KvsClient {
    server_address: SocketAddr,
}

impl KvsClient {
    #[must_use]
    pub fn new(server_address: SocketAddr) -> KvsClient {
        Self { server_address }
    }

    pub fn send_command(&self, command: &Command) -> Result<String> {
        let timeout = Duration::from_millis(5_000);
        let stream = TcpStream::connect_timeout(&self.server_address, timeout)?;
        stream.set_read_timeout(Some(timeout))?;
        stream.set_write_timeout(Some(timeout))?;
        command.serialize_into_stream(&stream)?;
        let response = CommandResponse::deserialize_from_stream(&stream)?;
        if response.is_err() {
            Err(GeneralError(response.to_string()))
        } else {
            Ok(response.to_string())
        }
    }
}
