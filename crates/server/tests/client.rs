use server::Client;
use std::sync::Arc;
use tokio::net::TcpStream;

pub struct TestClient {
    client: Arc<Client>,
    buff: Vec<u8>,
}

impl TestClient {
    pub async fn new(name: &str) -> Self {
        let mut client = Client::from(
            TcpStream::connect("localhost:1237")
                .await
                .expect("Could not connect"),
        );
        
        client.set_name(name.to_string());
        
        Self {
            client: Arc::new(client),
            buff: Vec::new(),
        }
    }
}

impl TestClient {
    
    pub async fn establish_connection(&mut self) -> trtcp::Response {
        let request = trtcp::Request::new(
            trtcp::Head::new(trtcp::Version::actual(), self.client.name()),
            trtcp::Action::new(trtcp::ActionType::Connect, "", ""),
            "".as_bytes(),
        );

        self.client.write(request).await.unwrap();

        self.client.read_and_wait(&mut self.buff).await.unwrap()
    }
    
    pub async fn invoke_event(&mut self, event: &str, body: &[u8]) {
        let request = trtcp::Request::new(
            trtcp::Head::new(trtcp::Version::actual(), self.client.name()),
            trtcp::Action::new(trtcp::ActionType::Invoke, "test", event),
            body,
        );

        self.client.write(request).await.unwrap();
    }

    pub async fn create_event(&mut self, event: &str) -> trtcp::Response {
        let request = trtcp::Request::new(
            trtcp::Head::new(trtcp::Version::actual(), self.client.name()),
            trtcp::Action::new(trtcp::ActionType::Create, "test", event),
            "".as_bytes(),
        );

        self.client.write(request).await.unwrap();
        
        self.client.read_and_wait(&mut self.buff).await.unwrap()
    }

    pub async fn listen_event(&mut self, event: &str) -> trtcp::Response {
        let request = trtcp::Request::new(
            trtcp::Head::new(trtcp::Version::actual(), self.client.name()),
            trtcp::Action::new(trtcp::ActionType::Listen, "test", event),
            "".as_bytes(),
        );

        self.client.write(request).await.unwrap();

        self.client.read_and_wait(&mut self.buff).await.unwrap()
    }
    
    pub async fn read_response(&mut self) -> trtcp::Response {
        self.client.read_and_wait(&mut self.buff).await.unwrap()
    }
    
    pub async fn read_request(&mut self) -> trtcp::Request {
        self.client.read_and_wait(&mut self.buff).await.unwrap()
    }
}