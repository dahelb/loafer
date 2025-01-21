use core::str;
use std::future::Future;
use std::path::PathBuf;
use std::time::Duration;
use tokio::fs;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::oneshot;

pub struct GopherServer<R: ResourceFetcher> {
    listener: TcpListener,
    fetcher: R,
    notify_shutdown: oneshot::Receiver<()>
}

impl<R: ResourceFetcher> GopherServer<R> {
    pub fn new(listener: TcpListener, fetcher: R, notify_shutdown: oneshot::Receiver<()> ) -> Self {
        GopherServer { listener, fetcher, notify_shutdown }
    }

    pub async fn run(mut self) {
        loop {
            let s = tokio::select! {
                Ok((stream, _socket)) = self.listener.accept() => stream,
                _ = &mut self.notify_shutdown => {
                    println!("Received shutdown signal ...");
                    break;
                }
            };

            let handler = Handler {
                fetcher: self.fetcher.clone(),
            };

            tokio::spawn(async move {
                handler.handle_connection(s).await;
            });
        }
    }
}

struct Handler<R: ResourceFetcher> {
    fetcher: R,
}

impl<R: ResourceFetcher> Handler<R> {
    async fn handle_connection(&self, mut stream: TcpStream) {
        let mut buf_reader = BufReader::new(&mut stream);

        let mut buffer = String::new();

        while let Ok(amt) = buf_reader.read_line(&mut buffer).await {
            if &buffer[buffer.len() - 2..] == "\r\n" || amt == 0 {
                break;
            }
        }

        let selector = &buffer[..buffer.len() - 2];

        let response = match selector {
            "" => self.fetcher.fetch_home().await,
            _ => self.fetcher.fetch_resource(selector).await,
        };

        tokio::time::sleep(Duration::from_secs(10)).await;

        stream.write_all(&response).await.unwrap();
    }
}

pub trait ResourceFetcher: Clone + Send + Sync + 'static {
    fn fetch_resource(&self, selector: &str) -> impl Future<Output = Vec<u8>> + Send;
    fn fetch_home(&self) -> impl Future<Output = Vec<u8>> + Send;
}

#[derive(Clone)]
pub struct FileDirFetcher {
    dir: PathBuf,
}

impl FileDirFetcher {
    pub fn new(dir: PathBuf) -> Self {
        FileDirFetcher { dir }
    }
}

impl ResourceFetcher for FileDirFetcher {
    async fn fetch_resource(&self, selector: &str) -> Vec<u8> {
        let path = self.dir.join(selector);

        println!("Serving file {path:?}");

        let data = fs::read(path).await.unwrap();

        data
    }

    async fn fetch_home(&self) -> Vec<u8> {
        self.fetch_resource("index").await
    }
}

pub async fn run<R>(listener: TcpListener, fetcher: R, signal: impl Future + Send + 'static) where R: ResourceFetcher {
    let (sender, receiver) = oneshot::channel();

    tokio::spawn(async move {
        signal.await;
        sender.send(()).unwrap();
    });

    GopherServer::new(listener, fetcher, receiver).run().await;
}
