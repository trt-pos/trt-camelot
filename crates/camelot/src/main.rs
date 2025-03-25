mod handlers;

use camelot::{Error, ReadHalfClient, WriteHalfClient};
use std::collections::HashMap;
use std::sync::{Arc, LazyLock};
use tokio::net::TcpStream;
use tokio::sync::{Mutex, RwLock};
use tracing::{error, info};
use trtcp::{ActionType, Head, Request, Response, Status, StatusType};

static CLIENT_WRITERS: LazyLock<Arc<RwLock<HashMap<String, Arc<Mutex<WriteHalfClient>>>>>> =
    LazyLock::new(|| Arc::new(RwLock::new(HashMap::new())));

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let args: Vec<String> = std::env::args().collect();

    let port = if args.len() == 2 {
        args[1].parse().expect("Invalid port")
    } else {
        1237
    };

    start_server(port).await;
}

pub async fn start_server(port: u16) {
    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{}", port))
        .await
        .unwrap_or_else(|_| panic!("Could not bind to 127.0.0.1:{}", port));

    info!("camelot initialized on port {}", port);

    loop {
        match listener.accept().await {
            Ok((socket, _)) => {
                tokio::spawn(async move {
                    handle_client(socket).await;
                });
            }
            Err(e) => error!("couldn't get client connection: {:?}", e),
        }
    }
}

async fn handle_client(socket: TcpStream) {
    let client_addr = socket.peer_addr();
    info!(
        "new tcp connection established with client {:?}",
        client_addr
    );

    let (mut reader, client_name) = match handle_first_connection(socket).await {
        Ok(o) => {
            let (reader, mut writer, caller_name) = match o {
                Some(client) => client,
                None => return,
            };

            {
                let writers = CLIENT_WRITERS.read().await;
                if writers.contains_key(&caller_name) {
                    info!(
                        "disconnecting client that used a name that is already in use ({})",
                        caller_name
                    );
                    let response = Response::new(
                        Head::new_with_version(&caller_name),
                        Status::new(StatusType::AlreadyConnected),
                        "".as_bytes(),
                    );

                    let _ = writer.write(response).await;
                    let _ = writer.shutdown().await;
                    return;
                }
            }

            {
                let mut writers = CLIENT_WRITERS.write().await;
                writers.insert(caller_name.to_string(), Arc::new(Mutex::new(writer)));
            }

            (reader, caller_name)
        }
        Err(e) => {
            error!("error handling first client connection {:?}", e);
            return;
        }
    };

    let mut buffer = vec![];
    info!(
        "persistent connection established with client {:?}",
        client_addr
    );

    loop {
        let request: Result<Request, Error> = reader.read(&mut buffer).await;

        let request = match request {
            Ok(request) => request,
            Err(_) => {
                // Sending the shutdown signal to the client
                {
                    let _ = CLIENT_WRITERS
                        .write()
                        .await
                        .get(&client_name)
                        .expect("Client not found")
                        .lock()
                        .await
                        .shutdown()
                        .await;
                }

                // Removing the client from the list
                {
                    CLIENT_WRITERS.write().await.remove(&client_name);
                }
                info!(
                    "due to an error while reading the client ({:?}) request, this has beed disconnected and removed",
                    client_addr
                );
                return;
            }
        };

        // Creating a response
        let response = handlers::handle_request(&request).await;

        {
            let guard = CLIENT_WRITERS.read().await;
            let mut writer = guard
                .get(&client_name)
                .expect("Client not found")
                .lock()
                .await;

            if writer.write(response).await.is_err() {
                error!("Error writing response to client {:?}", client_addr);
                let _ = writer.shutdown().await;
                drop(writer);
                drop(guard);
                CLIENT_WRITERS.write().await.remove(&client_name);
                break;
            }
        }
    }
}

async fn handle_first_connection(
    socket: TcpStream,
) -> Result<Option<(ReadHalfClient, WriteHalfClient, String)>, Error> {
    let client_addr = socket.peer_addr();
    let (mut reader, mut writer) = camelot::split(socket, "tmp").await;

    let mut buff = Vec::new();
    let request: Request = reader.read(&mut buff).await?;

    let client_name = request.head().caller();

    match request.action().r#type() {
        ActionType::Connect => {
            info!("persistence connection request sended by {:?}", client_addr);
            let response = Response::new_ok(client_name);

            writer.set_name(client_name.to_string());
            reader.set_name(client_name.to_string());

            writer.write(response).await?;
            Ok(Some((reader, writer, client_name.to_string())))
        }
        ActionType::Invoke => {
            info!("temporal connection request (invoke) sended by {:?}", client_addr);
            let response = handlers::handle_request(&request).await;
            writer.write(response).await?;
            writer.shutdown().await?;
            Ok(None)
        }
        _ => {
            info!("invalid request for a temporal connection sended by {:?}", client_addr);
            let response = Response::new(
                Head::new_with_version(client_name),
                Status::new(StatusType::NeedConnection),
                "".as_ref(),
            );

            writer.write(response).await?;
            writer.shutdown().await?;
            Ok(None)
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::time::Duration;
    use trtcp::Head;

    #[tokio::test]
    async fn test_handle_clients() {
        let server = tokio::spawn(async move {
            start_server(1237).await;
        });

        for i in 0..10 {
            let (mut reader, mut writer) = camelot::split(
                TcpStream::connect("localhost:1237")
                    .await
                    .expect("Could not connect"),
                "unknown",
            )
            .await;

            let client_name = format!("test{}", i);

            let request = Request::new(
                Head::new(trtcp::Version::actual(), &client_name),
                trtcp::Action::new(ActionType::Connect, "", ""),
                "".as_bytes(),
            );

            writer.write(request).await.expect("Could not write");

            let mut buf = vec![0u8; 1024];
            let response: Response = reader.read(&mut buf).await.expect("Could not read");

            assert_eq!(response.head().caller(), client_name);
            assert_eq!(*response.status().r#type(), StatusType::OK);
            assert!(CLIENT_WRITERS.read().await.contains_key(&client_name));
        }

        tokio::time::sleep(Duration::from_millis(100)).await;

        assert_eq!(CLIENT_WRITERS.read().await.len(), 0);

        server.abort();
    }
}
