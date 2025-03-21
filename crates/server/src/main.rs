extern crate core;

mod error;
mod handlers;

pub use error::Error;

// TODO: Hacer un ping para mantener la conexión y cerrarla si no hablo en los ultimos 5 mins

use std::collections::HashMap;
use std::io::ErrorKind;
use std::sync::{Arc, LazyLock};
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufWriter};
use tokio::net::TcpStream;
use tokio::sync::{Mutex, RwLock};
use trtcp::{ActionType, Head, Request, Response, Status, StatusType};

static CLIENTS: LazyLock<Arc<RwLock<HashMap<String, Client>>>> =
    LazyLock::new(|| Arc::new(RwLock::new(HashMap::new())));

struct Client {
    name: String,
    stream: Arc<Mutex<TcpStream>>,
}

impl Client {
    async fn read_and_wait<'r, R: TryFrom<&'r [u8], Error = trtcp::Error>>(&self, buf: &'r mut Vec<u8>) -> Result<R, Error> {
        self.read(buf, true).await
    }

    async fn read<'r, R: TryFrom<&'r [u8], Error = trtcp::Error>>(&self, buf: &'r mut Vec<u8>, blocking: bool) -> Result<R, Error> {
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
    
    async fn write<W: Into<Vec<u8>>>(&self, message: W) -> Result<(), Error> {
        let response_bytes: Vec<u8>= message.into();

        self.write_slice(response_bytes.as_slice()).await
    }

    async fn write_slice(&self, message: &[u8]) -> Result<(), Error> {
        {
            let mut stream = self.stream.lock().await;
            Self::write_stream(&mut stream, message).await?;
        }

        Ok(())
    }
    
    async fn shutdown(&self) -> Result<(), Error> {
        let mut stream = self.stream.lock().await;
        stream.shutdown().await?;

        Ok(())
    }


    async fn write_stream(writer: &mut TcpStream, bytes: &[u8]) -> Result<(), Error> {
        let mut writer = BufWriter::new(writer);
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

impl From<TcpStream> for Client {
    fn from(stream: TcpStream) -> Self {
        Client {
            name: "".to_string(),
            stream: Arc::new(Mutex::new(stream))
        }
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let args: Vec<String> = std::env::args().collect();

    let port = if args.len() == 2 {
        args[2].parse().expect("Invalid port")
    } else {
        1237
    };

    start_server(port).await;
}

async fn start_server(port: u16) {
    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{}", port))
        .await
        .expect("Could not bind");

    loop {
        match listener.accept().await {
            Ok((socket, _)) => {
                tokio::spawn(async move {
                    handle_client(socket).await;
                });
            }
            Err(e) => println!("couldn't get client: {:?}", e),
        }
    }
}

async fn handle_client(socket: TcpStream) {
    let client_name = match handle_first_connection(socket).await {
        Ok(client) => {
            let client = match client {
                Some(client) => client,
                None => return,
            };

            let client_name = client.name.clone();

            {
                let mut clients = CLIENTS.write().await;
                clients.insert(client.name.clone(), client);
            }

            client_name
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            return;
        }
    };

    let mut buffer = vec![];

    loop {
        let request: Result<Request, Error> = {
            let guard = CLIENTS.read().await;
            let client = guard.get(&client_name).expect("Client not found");
            client.read(&mut buffer, false).await
        };
        
        let request = match request {
            Ok(request) => request,
            Err(e) => {
                match e {
                    Error::NoData => continue,
                    _ => {
                        let guard = CLIENTS.write().await;
                        let client = guard.get(&client_name).expect("Client not found");
                        let _ = client.shutdown().await;
                        let client_name = client.name.clone();
                        drop(guard);
                        CLIENTS.write().await.remove(&client_name);
                        return;
                    }
                }
            }
        };
        
        // Creating a response
        let response = handlers::handle_request(&request).await;

        {
            let guard = CLIENTS.read().await;
            let client = guard.get(&client_name).expect("Client not found");

            if client.write(response).await.is_err() {
                client.shutdown().await.expect("Could not shutdown client");
                CLIENTS.write().await.remove(&client_name);
                break;
            }
        }
    }
}

async fn handle_first_connection(socket: TcpStream) -> Result<Option<Client>, Error> {
    let mut client = Client::from(socket);

    let mut buff = Vec::new();
    let request: Request = client.read_and_wait(&mut buff).await?;

    let client_name = request.head().caller();

    if *request.action().r#type() != ActionType::Connect {
        let response = Response::new(
            new_head(client_name),
            Status::new(StatusType::NeedConnection),
            "".as_ref(),
        );

        client.write(response).await?;
        client.shutdown().await?;
        Ok(None)
    } else {
        let response = ok_response(client_name);

        client.name = client_name.to_string();
        client.write(response).await?;
        Ok(Some(client))
    }
}

fn new_head(caller: &str) -> Head {
    Head::new(trtcp::Version::actual(), caller)
}

fn ok_response(caller: &str) -> Response {
    Response::new(new_head(caller), Status::new(StatusType::OK), "".as_bytes())
}

fn unexpected_error_response<'a>(caller: &'a str, error_msg: &'a str) -> Response<'a> {
    Response::new(
        new_head(caller),
        Status::new(StatusType::InternalServerError),
        error_msg.as_bytes(),
    )
}

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    async fn test_handle_clients() {
        let server = tokio::spawn(async move {
            start_server(1237).await;
        });

        for i in 0..10 {
            let client = Client::from(
                TcpStream::connect("localhost:1237")
                    .await
                    .expect("Could not connect"),
            );

            let client_name = format!("test{}", i);

            let request = Request::new(
                Head::new(trtcp::Version::actual(), &client_name),
                trtcp::Action::new(ActionType::Connect, "", ""),
                "".as_bytes(),
            );

            client.write(request).await.expect("Could not write");

            let mut buf = vec![0u8; 1024];
            let response: Response = client.read_and_wait(&mut buf).await.expect("Could not read");
            
            assert_eq!(response.head().caller(), client_name);
            assert_eq!(*response.status().r#type(), StatusType::OK);
            assert!(CLIENTS.read().await.contains_key(&client_name));
        }

        server.abort();
    }
}
