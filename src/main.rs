use std::path::PathBuf;

use loafer::FileDirFetcher;
use tokio::net::TcpListener;
use tracing::info;
use tracing_subscriber::prelude::*;
use tracing_subscriber::{self, fmt, EnvFilter};

#[tokio::main]
async fn main() {
    let listener = TcpListener::bind("127.0.0.1:7000").await.unwrap();

    let fetcher = FileDirFetcher::new(&PathBuf::from("tests/gopherhole"));

    let signal = tokio::signal::ctrl_c();

    let fmt_layer = fmt::layer().with_target(false);
    let filter_layer = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("info"))
        .unwrap();

    tracing_subscriber::registry()
        .with(filter_layer)
        .with(fmt_layer)
        .init();

    info!("Starting gopher server");
    loafer::run(listener, fetcher, signal).await;
}
