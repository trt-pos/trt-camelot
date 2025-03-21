#![allow(unused)]

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

        assert_eq!(
            String::from_utf8(client1.read_call().await.body().to_vec()).unwrap(),
            "Hello".to_string()
        );
        assert_eq!(
            String::from_utf8(client2.read_call().await.body().to_vec()).unwrap(),
            "Hello".to_string()
        );
        assert_eq!(
            String::from_utf8(client3.read_call().await.body().to_vec()).unwrap(),
            "Hello".to_string()
        );
        assert_eq!(
            String::from_utf8(client4.read_call().await.body().to_vec()).unwrap(),
            "Hello".to_string()
        );
    });

    let test = test.await;
    server::stop_server().await;

    assert!(test.is_ok())
}

fn check_response(response: &trtcp::Response) {
    if *response.status().r#type() != trtcp::StatusType::OK {
        panic!("Response status is not OK: {:?}", response);
    }
}
