mod error;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::TcpStream;

pub use error::Error;

pub struct WriteHalfClient {
    name: String,
    stream: OwnedWriteHalf,
}

impl WriteHalfClient {
    pub async fn write<W: Into<Vec<u8>>>(&mut self, message: W) -> Result<(), Error> {
        let response_bytes: Vec<u8> = message.into();

        self.write_slice(response_bytes.as_slice()).await
    }

    pub async fn write_slice(&mut self, message: &[u8]) -> Result<(), Error> {
        write_stream(&mut self.stream, message).await?;

        Ok(())
    }

    pub async fn shutdown(&mut self) -> Result<(), Error> {
        self.stream.shutdown().await?;

        Ok(())
    }

    pub async fn is_open(&self) -> bool {
        self.stream.writable().await.is_ok()
    }
    
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn set_name(&mut self, name: String) {
        self.name = name;
    }
}

pub struct ReadHalfClient {
    name: String,
    stream: OwnedReadHalf,
}

impl ReadHalfClient {
    pub async fn read<'r, R: TryFrom<&'r [u8], Error = trtcp::Error>>(
        &mut self,
        buf: &'r mut Vec<u8>,
    ) -> Result<R, Error> {
        buf.clear();

        read_stream(&mut self.stream, buf).await?;

        let result: Result<R, trtcp::Error> = buf.as_slice().try_into();
        match result {
            Ok(result) => Ok(result),
            Err(e) => Err(error::Error::from(e)),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn set_name(&mut self, name: String) {
        self.name = name;
    }
}

pub async fn split(stream: TcpStream, name: &str) -> (ReadHalfClient, WriteHalfClient) {
    let (read_half, write_half) = stream.into_split();
    (
        ReadHalfClient {
            name: name.to_string(),
            stream: read_half,
        },
        WriteHalfClient {
            name: name.to_string(),
            stream: write_half,
        },
    )
}

async fn write_stream(writer: &mut OwnedWriteHalf, bytes: &[u8]) -> Result<(), Error> {
    writer.write_all(bytes).await?;
    writer.flush().await?;
    Ok(())
}

async fn read_stream(reader: &mut OwnedReadHalf, buf: &mut Vec<u8>) -> Result<(), Error> {
    loop {
        let mut tmp_buf = [0; 1024];
        let size = reader.read(&mut tmp_buf).await?;

        if size == 0 {
            return Err(Error::ConexionClosed);
        }

        buf.extend_from_slice(&tmp_buf[..size]);

        if size < 1024 {
            break;
        }
    }
    Ok(())
}
