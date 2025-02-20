extern crate core;

mod error;
mod handlers;

pub use error::Error;

// TODO: Hacer un ping para mantener la conexión y cerrarla si no hablo en los ultimos 5 mins

use std::collections::HashMap;
use std::sync::{Arc, LazyLock, OnceLock};
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader, BufWriter, ReadHalf, WriteHalf};
use tokio::net::TcpStream;
use tokio::sync::{Mutex, RwLock};
use trtcp::{ActionType, Head, Request, Response};

static DB_POOL: LazyLock<OnceLock<sqlx::SqlitePool>> = LazyLock::new(OnceLock::new);
static CLIENTS: LazyLock<Arc<RwLock<HashMap<String, Client>>>> =
    LazyLock::new(|| Arc::new(RwLock::new(HashMap::new())));

struct Client {
    name: String,
    reader: Arc<Mutex<ReadHalf<TcpStream>>>,
    writer: Arc<Mutex<WriteHalf<TcpStream>>>,
}

impl Client {
    fn new(reader: ReadHalf<TcpStream>, writer: WriteHalf<TcpStream>) -> Self {
        Self {
            name: String::new(),
            reader: Arc::new(Mutex::new(reader)),
            writer: Arc::new(Mutex::new(writer)),
        }
    }

    async fn read<'r>(&self, buf: &'r mut Vec<u8>) -> Result<Request<'r>, Error> {
        buf.clear();

        let mut reader = self.reader.lock().await;

        read_stream(&mut reader, buf).await?;

        let request = Request::try_from(buf.as_slice())?;
        Ok(request)
    }

    async fn write(&self, response: Response<'_>) -> Result<(), Error> {
        let mut writer = self.writer.lock().await;

        let response_bytes: Vec<u8> = response.try_into()?;

        write_stream(&mut writer, response_bytes.as_slice()).await?;

        Ok(())
    }

    async fn shutdown(&self) -> Result<(), Error> {
        let mut writer = self.writer.lock().await;
        writer.shutdown().await?;

        Ok(())
    }
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() != 2 {
        eprintln!("Usage: {} <database-url> [port]", args[0]);
        std::process::exit(1);
    }

    let db = &args[1];

    let port = if args.len() == 3 {
        args[2].parse().expect("Invalid port")
    } else {
        1237
    };

    start_server(db, port).await;
}

async fn start_server(db: &str, port: u16) {
    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{}", port))
        .await
        .expect("Could not bind");

    DB_POOL.set(database::create_conn_pool(db).await).unwrap();

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

            CLIENTS.write().await.insert(client.name.clone(), client);

            client_name
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            return;
        }
    };

    let mut buffer = vec![];

    loop {
        let request = {
            let guard = CLIENTS.read().await;
            let client = guard.get(&client_name).expect("Client not found");

            if let Ok(request) = client.read(&mut buffer).await {
                request
            } else {
                client.shutdown().await.expect("Could not shutdown client");
                drop(guard);
                CLIENTS.write().await.remove(&client_name);
                break;
            }
        };

        // Creating a response
        let response = handlers::handle_request(request).await;

        {
            let guard = CLIENTS.read().await;
            let client = guard.get(&client_name).expect("Client not found");

            if client.write(response).await.is_err() {
                client.shutdown().await.expect("Could not shutdown client");
                drop(guard);
                CLIENTS.write().await.remove(&client_name);
                break;
            }
        }
    }
}

async fn handle_first_connection(socket: TcpStream) -> Result<Option<Client>, Error> {
    let (reader, writer) = tokio::io::split(socket);
    let mut client = Client::new(reader, writer);

    let mut buff = Vec::new();

    let request = client.read(&mut buff).await?;

    let client_name = request.head().caller.to_string();

    if *request.action().r#type() != ActionType::Connect {
        let response = Response::new(
            head("server"),
            trtcp::Status::new(trtcp::StatusType::NeedConnection),
            "",
        );

        client.write(response).await?;
        client.shutdown().await?;
        Ok(None)
    } else {
        let response = Response::new(
            head("server"),
            trtcp::Status::new(trtcp::StatusType::OK),
            "",
        );

        client.name = client_name;
        client.write(response).await?;
        Ok(Some(client))
    }
}

async fn read_stream(reader: &mut ReadHalf<TcpStream>, bytes: &mut Vec<u8>) -> Result<(), Error> {
    let mut reader = BufReader::new(reader);
    loop {
        let mut buf = [0; 1024];
        let size = reader
            .read(&mut buf)
            .await
            .map_err(|_| Error::ReadingError)?;
        bytes.extend_from_slice(&buf[..size]);

        if size < 1024 {
            break;
        }
    }

    Ok(())
}

async fn write_stream(writer: &mut WriteHalf<TcpStream>, bytes: &[u8]) -> Result<(), Error> {
    let mut writer = BufWriter::new(writer);
    writer.write_all(bytes).await?;

    writer.flush().await?;
    Ok(())
}

fn head(caller: &str) -> Head {
    Head::new(trtcp::Version::actual(), caller)
}

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    async fn test_handle_clients() {
        let db = "sqlite::memory:";

        let server = tokio::spawn(async move {
            start_server(db, 1237).await;
        });

        for i in 0..10 {
            let client = TcpStream::connect("localhost:1237")
                .await
                .expect("Could not connect");

            let client_name = format!("test{}", i);

            let (mut reader, mut writer) = tokio::io::split(client);

            let request = Request::new(
                Head::new(trtcp::Version::actual(), &client_name),
                trtcp::Action::new(ActionType::Connect, "", ""),
                "",
            );

            let request_bytes: Vec<u8> = request.try_into().unwrap();
            write_stream(&mut writer, request_bytes.as_slice())
                .await
                .expect("Could not write");

            let mut buf = vec![0; 1024];
            let size = reader.read(&mut buf).await.unwrap();

            let response = Response::try_from(&buf[..size]).unwrap();

            assert_eq!(*response.status().r#type(), trtcp::StatusType::OK);
            assert!(CLIENTS.read().await.contains_key(&client_name));
        }

        server.abort();
    }
}
