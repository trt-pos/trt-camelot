mod handlers;

use server::{Error, ReadHalfClient, WriteHalfClient};
use std::collections::HashMap;
use std::sync::{Arc, LazyLock};
use tokio::net::TcpStream;
use tokio::sync::{Mutex, RwLock};
use tracing::error;
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
        .expect("Could not bind");

    loop {
        match listener.accept().await {
            Ok((socket, _)) => {
                tokio::spawn(async move {
                    handle_client(socket).await;
                });
            }
            Err(e) => error!("couldn't accept client connection: {:?}", e),
        }
    }
}

async fn handle_client(mut socket: TcpStream) {
    let socket = &mut socket;
    let (mut reader, client_name) = match handle_first_connection(socket).await {
        Ok(o) => {
            let (reader, writer, caller_name) = match o {
                Some(client) => client,
                None => return,
            };

            {
                let mut writers = CLIENT_WRITERS.write().await;
                writers.insert(caller_name.to_string(), Arc::new(Mutex::new(writer)));
            }

            (reader, caller_name)
        }
        Err(e) => {
            error!("{:?}", e);
            return;
        }
    };

    let mut buffer = vec![];

    loop {
        let request: Result<Request, Error> = reader.read(&mut buffer, false).await;

        let request = match request {
            Ok(request) => request,
            Err(e) => match e {
                Error::NoData => continue,
                _ => {
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
                    return;
                }
            },
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
                error!("Error writing response to client {}", client_name);
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
    socket: &mut TcpStream,
) -> Result<Option<(ReadHalfClient, WriteHalfClient, String)>, Error> {
    let (mut reader, mut writer) = server::split(socket, "tmp").await;

    let mut buff = Vec::new();
    let request: Request = reader.read_and_wait(&mut buff).await?;

    let client_name = request.head().caller();

    match request.action().r#type() {
        ActionType::Connect => {
            let response = Response::new_ok(client_name);

            writer.set_name(client_name.to_string());
            reader.set_name(client_name.to_string());

            writer.write(response).await?;
            Ok(Some((reader, writer, client_name.to_string())))
        }
        ActionType::Invoke => {
            let response = handlers::handle_request(&request).await;
            writer.write(response).await?;
            writer.shutdown().await?;
            Ok(None)
        }
        _ => {
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
            let response: Response = client
                .read_and_wait(&mut buf)
                .await
                .expect("Could not read");

            assert_eq!(response.head().caller(), client_name);
            assert_eq!(*response.status().r#type(), StatusType::OK);
            assert!(CLIENT_WRITERS.read().await.contains_key(&client_name));
        }

        tokio::time::sleep(Duration::from_millis(100)).await;

        assert_eq!(CLIENT_WRITERS.read().await.len(), 0);

        server.abort();
    }
}
