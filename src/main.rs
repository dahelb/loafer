use std::path::PathBuf;

use loafer::FileDirFetcher;
use tokio::net::TcpListener;
use tracing::info;
use tracing_subscriber;

#[tokio::main]
async fn main() {
    let listener = TcpListener::bind("127.0.0.1:7000").await.unwrap();

    let fetcher = FileDirFetcher::new(&PathBuf::from("tests/gopherhole"));

    let signal = tokio::signal::ctrl_c();

    tracing_subscriber::fmt::init();

    info!("Starting gopher server");
    loafer::run(listener, fetcher, signal).await;
}
