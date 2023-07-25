use serde::{de::DeserializeOwned, Serialize};
use std::{
    io::{BufRead, Seek, Write},
    net::TcpStream,
};

pub trait BincodeSerde {
    fn serialize_into_stream(&self, writer: &TcpStream) -> crate::Result<()>
    where
        Self: Serialize,
    {
        Ok(bincode::serialize_into(writer, self)?)
    }

    fn deserialize_from_stream(reader: &TcpStream) -> crate::Result<Self>
    where
        Self: DeserializeOwned,
    {
        Ok(bincode::deserialize_from::<_, Self>(reader)?)
    }

    fn serialize_into_writer<T: Write + Seek>(&self, mut writer: T) -> crate::Result<u64>
    where
        Self: Serialize,
    {
        let log_position = writer.stream_position()?;
        bincode::serialize_into(&mut writer, self)?;
        writer.flush()?;
        Ok(log_position)
    }

    fn deserialize_from_reader<T: BufRead>(reader: T) -> crate::Result<Self>
    where
        Self: DeserializeOwned,
    {
        Ok(bincode::deserialize_from::<_, Self>(reader)?)
    }
}
