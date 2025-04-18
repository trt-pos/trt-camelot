mod client;
mod server;

use client::TestClient as Client;

#[tokio::test]
async fn call_events() {
    server::start_server().await;

    let test = tokio::spawn(async {
        let mut client1 = Client::new("client1").await;
        let mut client2 = Client::new("client2").await;
        let mut client3 = Client::new("client3").await;
        let mut client4 = Client::new("client4").await;

        check_response(&client1.establish_connection().await);
        check_response(&client2.establish_connection().await);
        check_response(&client3.establish_connection().await);
        check_response(&client4.establish_connection().await);

        check_response(&client1.create_event("test").await);

        check_response(&client1.listen_event("test").await);
        check_response(&client2.listen_event("test").await);
        check_response(&client3.listen_event("test").await);
        check_response(&client4.listen_event("test").await);

        client1.invoke_event("test", "Hello".as_bytes()).await;
        
        check_callback(&client1.read_request().await, "Hello".as_bytes());
        check_callback(&client2.read_request().await, "Hello".as_bytes());
        check_callback(&client3.read_request().await, "Hello".as_bytes());
        check_callback(&client4.read_request().await, "Hello".as_bytes());
        
        check_response(&client1.read_response().await);
    }).await;

    server::stop_server().await;

    assert!(test.is_ok());
}

fn check_response(response: &trtcp::Response) {
    if *response.status().r#type() != trtcp::StatusType::OK {
        panic!("Response status is not OK: {:?}", response);
    }
}

fn check_callback(request: &trtcp::Request, body: &[u8]) {
    if *request.body() != body {
        panic!("Response body is not equal to expected: {:?} != {:?}", request.body(), body);
    }
}