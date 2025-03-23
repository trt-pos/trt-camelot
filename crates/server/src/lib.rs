mod error;

use std::io::ErrorKind;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use trtcp::{Head, Response, Status, StatusType};

pub use error::Error;

pub struct Client {
    name: String,
    stream: Arc<Mutex<TcpStream>>,
}

impl Client {
    pub async fn read_and_wait<'r, R: TryFrom<&'r [u8], Error = trtcp::Error>>(&self, buf: &'r mut Vec<u8>) -> Result<R, Error> {
        self.read(buf, true).await
    }

    pub async fn read<'r, R: TryFrom<&'r [u8], Error = trtcp::Error>>(&self, buf: &'r mut Vec<u8>, blocking: bool) -> Result<R, Error> {
        buf.clear();

        {
            let mut stream_guard = self.stream.lock().await;
            Self::read_stream(&mut stream_guard, buf, blocking).await?;
        }

        let result: Result<R, trtcp::Error> = buf.as_slice().try_into();
        match result {
            Ok(result) => Ok(result),
            Err(e) => Err(error::Error::from(e)),
        }
    }
    
    pub async fn write<W: Into<Vec<u8>>>(&self, message: W) -> Result<(), Error> {
        let response_bytes: Vec<u8>= message.into();

        self.write_slice(response_bytes.as_slice()).await
    }

    pub async fn write_slice(&self, message: &[u8]) -> Result<(), Error> {
        {
            let mut stream = self.stream.lock().await;
            Self::write_stream(&mut stream, message).await?;
        }

        Ok(())
    }
    
    pub async fn shutdown(&self) -> Result<(), Error> {
        let mut stream = self.stream.lock().await;
        stream.shutdown().await?;

        Ok(())
    }
    
    async fn write_stream(writer: &mut TcpStream, bytes: &[u8]) -> Result<(), Error> {
        writer.write_all(bytes).await?;
        writer.flush().await?;
        Ok(())
    }

    async fn read_stream(reader: &mut TcpStream, buf: &mut Vec<u8>, blocking: bool) -> Result<(), Error> {
        loop {
            let mut tmp_buf = [0; 1024];
            let size = if blocking {
                reader.read(&mut tmp_buf).await
            } else {
                reader.try_read(&mut tmp_buf)
            }.map_err(|e| {
                if e.kind() == ErrorKind::WouldBlock {
                    Error::NoData
                } else {
                    Error::ReadingError
                }
            })?;
            
            if size == 0 { return Err(Error::ConexionClosed) }
            
            buf.extend_from_slice(&tmp_buf[..size]);

            if size < 1024 {
                break;
            }
        }
        Ok(())
    }

}

impl Client {
    pub fn name(&self) -> &str {
        &self.name
    }
    
    pub fn set_name(&mut self, name: String) {
        self.name = name;
    }
}

impl From<TcpStream> for Client {
    fn from(stream: TcpStream) -> Self {
        Client {
            name: "".to_string(),
            stream: Arc::new(Mutex::new(stream))
        }
    }
}

pub fn new_head(caller: &str) -> Head {
    Head::new(trtcp::Version::actual(), caller)
}

pub fn ok_response(caller: &str) -> Response {
    Response::new(new_head(caller), Status::new(StatusType::OK), "".as_bytes())
}

pub fn unexpected_error_response<'a>(caller: &'a str, error_msg: &'a str) -> Response<'a> {
    Response::new(
        new_head(caller),
        Status::new(StatusType::InternalServerError),
        error_msg.as_bytes(),
    )
}