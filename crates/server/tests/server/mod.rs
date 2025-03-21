use std::sync::{LazyLock};
use std::time::Duration;
use tokio::process::Child;
use tokio::process::Command;
use tokio::sync::Mutex;

static SERVER: LazyLock<Mutex<Child>> = LazyLock::new(|| {
    Mutex::new(
        Command::new("../../target/debug/server")
            .spawn()
            .expect("Could not start server"),
    )
});

pub async  fn start_server() {
    let _ = SERVER.lock().await;
    tokio::time::sleep(Duration::from_millis(100)).await;
}

pub async fn stop_server() {
    SERVER.lock().await.kill().await.expect("Could not kill server");
}
