use std::path::PathBuf;

use loafer::{FileDirFetcher, GopherServer};
use tokio::{net::TcpListener, signal};

#[tokio::main]
async fn main() {
    let listener = TcpListener::bind("127.0.0.1:7000").await.unwrap();

    let fetcher = FileDirFetcher::new(PathBuf::from("tests/gopherhole"));

    let signal = tokio::signal::ctrl_c();

    loafer::run(listener, fetcher, signal).await;
}
