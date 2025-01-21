use std::{
    io::{BufRead, BufReader, Write},
    net::{SocketAddr, TcpListener, TcpStream, ToSocketAddrs},
    path::PathBuf,
    sync::mpsc::{channel, Sender},
    thread,
};

use loafer::{FileDirFetcher, GopherServer};

pub struct TestClient<T: ToSocketAddrs> {
    addr: T,
}

impl<T: ToSocketAddrs> TestClient<T> {
    pub fn new(addr: T) -> TestClient<T> {
        TestClient { addr }
    }

    pub fn send_request(&self, selector: &str) -> String {
        let mut stream = TcpStream::connect(&self.addr).unwrap();

        let mut selector = selector.to_string();
        selector.push_str("\r\n");

        stream.write_all(selector.as_bytes()).unwrap();

        let buf_reader = BufReader::new(stream);

        let lines: Vec<_> = buf_reader
            .lines()
            .map(|result| result.unwrap())
            .take_while(|line| line != ".")
            .collect();

        lines.join("\n")
    }
}

pub struct TestServer {
    signal_send: Sender<()>,
    server_handle: thread::JoinHandle<()>,
    addr: SocketAddr,
}

impl TestServer {
    pub fn spawn() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();

        let addr = listener.local_addr().unwrap();

        let server = GopherServer::new(
            listener,
            FileDirFetcher::new(PathBuf::from("tests/gopherhole/")),
        );

        let (sender, receiver) = channel();

        let server_handle = thread::spawn(|| server.run_with_graceful_shutdown(receiver));

        return TestServer {
            addr,
            server_handle,
            signal_send: sender,
        };
    }

    pub fn shutdown(self) {
        self.signal_send.send(()).unwrap();
        self.server_handle.join().unwrap();
        println!("Server shut down!")
    }

    pub fn get_addr(&self) -> SocketAddr {
        self.addr
    }
}
