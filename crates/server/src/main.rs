use std::sync::{LazyLock, OnceLock};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

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

static DB_CONN: LazyLock<OnceLock<String>> = LazyLock::new(OnceLock::new);

async fn start_server(db: &str, port: u16) {
    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{}", port))
        .await
        .expect("Could not bind");
    
    DB_CONN.set(db.to_string()).unwrap();
    
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

async fn handle_client(socket: tokio::net::TcpStream) {
    let db = DB_CONN.get().unwrap();
    let _ = database::create_conn_pool(db).await;

    let (reader, writer) = tokio::io::split(socket);

    let mut reader = tokio::io::BufReader::new(reader);
    let mut writer = tokio::io::BufWriter::new(writer);

    loop {
        let mut buf = vec![0; 1024];
        let size = reader.read(&mut buf).await.unwrap();
        
        let response = std::str::from_utf8(&buf[..size]).unwrap();
        let response = response.trim();

        writer.write_all(response.as_bytes()).await.unwrap();
        writer.flush().await.unwrap();
    }
}

#[cfg(test)]
mod test {
    use std::thread::sleep;
    use super::*;

    #[tokio::test]
    async fn test_handle_client() {
        let db = "sqlite::memory:";

        let _server = tokio::spawn(async move {
            start_server(db, 1237).await;
        });

        sleep(std::time::Duration::from_secs(5));
        
        let mut client = tokio::net::TcpStream::connect("localhost:1237")
            .await
            .expect("Could not connect");

        let (reader, writer) = tokio::io::split(client);

        let mut reader = tokio::io::BufReader::new(reader);
        let mut writer = tokio::io::BufWriter::new(writer);
        
        writer.write_all(b"Hello").await.unwrap();
        writer.flush().await.unwrap();
        
        let mut buf = vec![0; 1024];
        let size = reader.read(&mut buf).await.unwrap();
        
        let response = std::str::from_utf8(&buf[..size]).unwrap();
        let response = response.trim();
        
        assert_eq!(response, "Hello");
    }
}