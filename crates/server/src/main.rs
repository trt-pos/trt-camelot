extern crate core;

mod handlers;

use std::collections::HashMap;
use std::sync::{Arc, LazyLock};
use tokio::net::TcpStream;
use tokio::sync::RwLock;
use server::{Client, Error};
use trtcp::{ActionType, Request, Response, Status, StatusType};

static CLIENTS: LazyLock<Arc<RwLock<HashMap<String, Client>>>> =
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

            let client_name = client.name().to_string();

            {
                let mut clients = CLIENTS.write().await;
                clients.insert(client_name.to_string(), client);
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
                        let client_name = client.name().to_string();
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
                let _ = client.shutdown().await;
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

    match request.action().r#type() { 
        ActionType::Connect => {
            let response = server::ok_response(client_name);

            client.set_name(client_name.to_string());
            client.write(response).await?;
            Ok(Some(client))
        }
        ActionType::Invoke => {
            let response = handlers::handle_request(&request).await;
            client.write(response).await?;
            client.shutdown().await?;
            Ok(None)
        }
        _ => {
            let response = Response::new(
                server::new_head(client_name),
                Status::new(StatusType::NeedConnection),
                "".as_ref(),
            );

            client.write(response).await?;
            client.shutdown().await?;
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
            let response: Response = client.read_and_wait(&mut buf).await.expect("Could not read");
            
            assert_eq!(response.head().caller(), client_name);
            assert_eq!(*response.status().r#type(), StatusType::OK);
            assert!(CLIENTS.read().await.contains_key(&client_name));
        }

        tokio::time::sleep(Duration::from_millis(100)).await;
        
        assert_eq!(CLIENTS.read().await.len(), 0);
        
        server.abort();
    }
}
