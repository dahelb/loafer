use core::str;
use std::fmt::Debug;
use std::future::Future;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Semaphore;
use tokio_util::sync::CancellationToken;
use tokio_util::task::task_tracker::TaskTracker;
use tracing::{debug, info};

pub mod config;

#[derive(Debug)]
pub struct GopherServer<R: ResourceFetcher + std::fmt::Debug> {
    listener: TcpListener,
    fetcher: R,
    token: CancellationToken,
    semaphore: Arc<Semaphore>,
}

impl<R: ResourceFetcher + std::fmt::Debug> GopherServer<R> {
    pub fn new(
        listener: TcpListener,
        fetcher: R,
        token: CancellationToken,
        max_connections: usize,
    ) -> Self {
        GopherServer {
            listener,
            fetcher,
            token,
            semaphore: Arc::new(Semaphore::new(max_connections)),
        }
    }

    pub async fn run(self) {
        let tracker = TaskTracker::new();

        loop {
            let permit = self.semaphore.clone().acquire_owned().await.unwrap();
            debug!("Permit acquired, processing connection");

            let s = tokio::select! {
                Ok((s, _socket)) = self.listener.accept() => s,
                _ = self.token.cancelled() => {
                    tracker.close();
                    break;
                }
            };

            let handler = Handler {
                fetcher: self.fetcher.clone(),
                token: self.token.clone(),
            };

            tracker.spawn(async move {
                handler.handle_connection(s).await;
                drop(permit);
                debug!("Finished handling task");
            });
        }
        info!("Waiting for tasks to finish ...");
        tracker.wait().await;
    }
}

struct Handler<R: ResourceFetcher> {
    fetcher: R,
    token: CancellationToken,
}

async fn decode(stream: &mut TcpStream) -> String {
    let mut buf_reader = BufReader::new(stream);
    let mut buffer = String::new();

    while let Ok(amt) = buf_reader.read_line(&mut buffer).await {
        if &buffer[buffer.len() - 2..] == "\r\n" || amt == 0 {
            break;
        }
    }

    if buffer.len() >= 2 {
        buffer.truncate(buffer.len() - 2);
    }

    buffer
}

impl<R: ResourceFetcher> Handler<R> {
    async fn handle_connection(&self, mut stream: TcpStream) {
        let selector = tokio::select! {
            s = decode(&mut stream) => s,
            _ = self.token.cancelled() => { return }
        };

        info!(selector, "Handling selector");

        let response = match selector.as_str() {
            "" => self.fetcher.fetch_home().await,
            _ => self.fetcher.fetch_resource(&selector).await,
        };

        if !self.token.is_cancelled() {
            stream.write_all(&response.unwrap()).await.unwrap();
        }
    }
}

pub trait ResourceFetcher: Clone + Debug + Send + Sync + 'static {
    fn fetch_resource(&self, selector: &str) -> impl Future<Output = io::Result<Vec<u8>>> + Send;
    fn fetch_home(&self) -> impl Future<Output = io::Result<Vec<u8>>> + Send;
}

#[derive(Clone, Debug)]
pub struct FileDirFetcher {
    dir: PathBuf,
    index_file: PathBuf,
}

impl FileDirFetcher {
    pub fn new(dir: &Path, index_file: &Path) -> Self {
        FileDirFetcher {
            dir: dir.canonicalize().unwrap(),
            index_file: index_file.canonicalize().unwrap(),
        }
    }
}

fn resolve_path<P: AsRef<Path> + Debug>(root: &Path, component: P) -> io::Result<PathBuf> {
    debug!(?root, ?component, "Canoncializing");

    let canon = if component.as_ref().has_root() {
        root.join(".")
            .join(component.as_ref().strip_prefix("/").unwrap())
            .canonicalize()
            .unwrap()
    } else {
        root.join(component.as_ref()).canonicalize().unwrap()
    };

    debug!(?canon, "Canonical path built");

    if !canon.starts_with(root) {
        Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "Access outside of root dir",
        ))
    } else {
        Ok(canon)
    }
}

impl ResourceFetcher for FileDirFetcher {
    async fn fetch_resource(&self, selector: &str) -> io::Result<Vec<u8>> {
        let path = resolve_path(&self.dir, selector)?;

        info!("Serving file {path:?}");

        fs::read(path).await
    }

    async fn fetch_home(&self) -> io::Result<Vec<u8>> {
        fs::read(self.index_file.clone()).await
    }
}

pub async fn run<R>(
    listener: TcpListener,
    fetcher: R,
    signal: impl Future + Send + 'static,
    max_connections: usize,
) where
    R: ResourceFetcher,
{
    let token = CancellationToken::new();

    let cloned_token = token.clone();

    tokio::spawn(async move {
        signal.await;
        info!("Got shutdown signal, initiating shutdown ...");
        cloned_token.cancel();
    });

    GopherServer::new(listener, fetcher, token, max_connections)
        .run()
        .await;
}
