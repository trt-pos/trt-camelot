use tokio::net::TcpStream;
use camelot::{ReadHalfClient, WriteHalfClient};

pub struct TestClient {
    reader: ReadHalfClient,
    writer: WriteHalfClient,
    buff: Vec<u8>,
}

impl TestClient {
    pub async fn new(name: &str) -> Self {
        let (reader, writer) = camelot::split(
            TcpStream::connect("localhost:1237")
                .await
                .expect("Could not connect"),
            name
        ).await;
        
        Self {
            reader,
            writer,
            buff: Vec::new(),
        }
    }
}

impl TestClient {
    
    pub async fn establish_connection(&mut self) -> trtcp::Response {
        let request = trtcp::Request::new(
            trtcp::Head::new_with_version(self.reader.name()),
            trtcp::Action::new(trtcp::ActionType::Connect, "", ""),
            "".as_bytes(),
        );

        self.writer.write(request).await.unwrap();

        self.reader.read(&mut self.buff).await.unwrap()
    }
    
    pub async fn invoke_event(&mut self, event: &str, body: &[u8]) {
        let request = trtcp::Request::new(
            trtcp::Head::new(trtcp::Version::actual(), self.reader.name()),
            trtcp::Action::new(trtcp::ActionType::Invoke, "test", event),
            body,
        );

        self.writer.write(request).await.unwrap();
    }

    pub async fn create_event(&mut self, event: &str) -> trtcp::Response {
        let request = trtcp::Request::new(
            trtcp::Head::new(trtcp::Version::actual(), self.reader.name()),
            trtcp::Action::new(trtcp::ActionType::Create, "test", event),
            "".as_bytes(),
        );

        self.writer.write(request).await.unwrap();
        
        self.reader.read(&mut self.buff).await.unwrap()
    }

    pub async fn listen_event(&mut self, event: &str) -> trtcp::Response {
        let request = trtcp::Request::new(
            trtcp::Head::new(trtcp::Version::actual(), self.reader.name()),
            trtcp::Action::new(trtcp::ActionType::Listen, "test", event),
            "".as_bytes(),
        );

        self.writer.write(request).await.unwrap();

        self.reader.read(&mut self.buff).await.unwrap()
    }
    
    pub async fn read_response(&mut self) -> trtcp::Response {
        self.reader.read(&mut self.buff).await.unwrap()
    }
    
    pub async fn read_request(&mut self) -> trtcp::Request {
        self.reader.read(&mut self.buff).await.unwrap()
    }
}