use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::task;
use tokio::{io::AsyncWriteExt, net::ToSocketAddrs};
use tokio::{net::TcpListener, net::TcpStream};

use loafer::{FileDirFetcher, GopherServer};
use tokio_util::sync::CancellationToken;

pub fn read_file_wo_lastline<P>(path: P) -> String
where
    P: AsRef<Path>,
{
    let mut contents = std::fs::read_to_string(path).unwrap();

    if &contents[contents.len() - 5..] == "\r\n.\r\n" {
        contents.truncate(contents.len() - 5);
    }
    contents
}

pub struct TestClient<T: ToSocketAddrs> {
    addr: T,
}

impl<T: ToSocketAddrs> TestClient<T> {
    pub fn new(addr: T) -> TestClient<T> {
        TestClient { addr }
    }

    pub async fn send_request(&self, selector: &str) -> String {
        let mut stream = TcpStream::connect(&self.addr).await.unwrap();

        let mut selector = selector.to_string();
        selector.push_str("\r\n");

        stream.write_all(selector.as_bytes()).await.unwrap();

        let mut buf_reader = BufReader::new(stream);

        let mut buffer = String::new();

        let mut has_lastline = false;
        while let Ok(amt) = buf_reader.read_line(&mut buffer).await {
            if amt == 0 {
                break;
            }
            if &buffer[buffer.len() - 5..] == "\r\n.\r\n" {
                has_lastline = true;
                break;
            }
        }

        if has_lastline {
            buffer.truncate(buffer.len() - 5);
        }

        buffer
    }
}

pub struct TestServer {
    addr: SocketAddr,
    shutdown_token: CancellationToken,
}

impl TestServer {
    pub async fn spawn() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();

        let addr = listener.local_addr().unwrap();

        let token = CancellationToken::new();

        let cloned_token = token.clone();

        task::spawn(async move {
            GopherServer::new(
                listener,
                FileDirFetcher::new(&PathBuf::from("tests/gopherhole/")),
                cloned_token,
                5,
            )
            .run()
            .await;
        });

        return TestServer {
            addr,
            shutdown_token: token,
        };
    }

    pub fn shutdown(self) {
        self.shutdown_token.cancel();
    }

    pub fn get_addr(&self) -> SocketAddr {
        self.addr
    }
}
