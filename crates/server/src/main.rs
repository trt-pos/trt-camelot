extern crate core;

mod error;
mod handlers;

pub use error::Error;

// TODO: Hacer un ping para mantener la conexión y cerrarla si no hablo en los ultimos 5 mins

use std::collections::HashMap;
use std::ops::DerefMut;
use std::sync::{Arc, LazyLock, OnceLock};
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader, BufWriter, ReadHalf, WriteHalf};
use tokio::net::TcpStream;
use tokio::sync::{Mutex, RwLock};
use trtcp::{ActionType, Head, Request, Response};

static DB_POOL: LazyLock<OnceLock<sqlx::SqlitePool>> = LazyLock::new(OnceLock::new);
static CLIENTS: LazyLock<Arc<RwLock<HashMap<String, ClientData>>>> =
    LazyLock::new(|| Arc::new(RwLock::new(HashMap::new())));

struct ClientData {
    reader: Arc<Mutex<ReadHalf<TcpStream>>>,
    writer: Arc<Mutex<WriteHalf<TcpStream>>>,
}

impl ClientData {
    pub fn new(reader: ReadHalf<TcpStream>, writer: WriteHalf<TcpStream>) -> Self {
        Self {
            reader: Arc::new(Mutex::new(reader)),
            writer: Arc::new(Mutex::new(writer)),
        }
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
    // TODO: Improve the error handling
    let (mut reader, mut writer) = tokio::io::split(socket);

    let client_name = match handle_first_connection(&mut reader).await {
        Ok((response, valid, client_name)) => {
            let response_bytes: Vec<u8> = response.try_into().expect("Invalid response");
            write_stream(&mut writer, response_bytes.as_slice())
                .await
                .expect("Could not write");
            if valid {
                CLIENTS
                    .write()
                    .await
                    .insert(client_name.clone(), ClientData::new(reader, writer));
            } else {
                writer.shutdown().await.expect("Could not shutdown client");
                return;
            }

            client_name
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            writer.shutdown().await.expect("Could not shutdown");
            return;
        }
    };

    loop {
        // TODO: Make a get_request and send_response function inside the ClientData struct
        // Reading the client stream
        let mutex_guard = CLIENTS.read().await;
        let clients_hash_map = mutex_guard.get(&client_name).expect("Client not found");

        let mut reader = clients_hash_map.reader.lock().await;
        let request_bytes = read_stream(reader.deref_mut())
            .await
            .expect("Could not read");

        drop(reader);
        drop(mutex_guard);

        // Creating a response
        let request = Request::try_from(request_bytes.as_slice()).expect("Invalid request");
        let response = handlers::handle_request(request).await;
        let response_bytes: Vec<u8> = response.try_into().expect("Invalid response");

        // Writing the response to the client stream
        let mutex_guard = CLIENTS.read().await;
        let clients_hash_map = mutex_guard.get(&client_name).expect("Client not found");
        
        let mut writer = clients_hash_map.writer.lock().await;
        write_stream(writer.deref_mut(), response_bytes.as_slice())
            .await
            .expect("Could not write");
        
        drop(writer);
        drop(mutex_guard);
    }
}

async fn handle_first_connection<'a>(
    reader: &mut ReadHalf<TcpStream>,
) -> Result<(Response<'a>, bool, String), Error> {
    let request_bytes = read_stream(reader).await?;
    let request = Request::try_from(request_bytes.as_slice())?;
    let caller = request.head().caller.to_string();

    let ret = if *request.action().r#type() != ActionType::Connect {
        (
            Response::new(
                head("server"),
                trtcp::Status::new(trtcp::StatusType::NeedConnection),
                "",
            ),
            false,
            caller,
        )
    } else {
        (
            Response::new(
                head("server"),
                trtcp::Status::new(trtcp::StatusType::OK),
                "",
            ),
            true,
            caller,
        )
    };

    Ok(ret)
}

async fn read_stream(reader: &mut ReadHalf<TcpStream>) -> Result<Vec<u8>, Error> {
    let mut reader = BufReader::new(reader);
    let mut req_bytes = Vec::new();
    loop {
        let mut buf = [0; 1024];
        let size = reader
            .read(&mut buf)
            .await
            .map_err(|_| Error::ReadingError)?;
        req_bytes.extend_from_slice(&buf[..size]);

        if size < 1024 {
            break;
        }
    }

    Ok(req_bytes)
}

async fn write_stream(
    writer: &mut WriteHalf<TcpStream>,
    bytes: &[u8],
) -> Result<(), Error> {
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
    async fn test_handle_client() {
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
            
            writer.shutdown().await.expect("Error shutting down client");
        }
        
        assert_eq!(CLIENTS.read().await.len(), 10);

        server.abort();
    }
}
