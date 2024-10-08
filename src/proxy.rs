use std::pin::Pin;

use tokio::{
    io::{AsyncRead, AsyncWrite},
    net::TcpListener,
};

// listen for incoming connections, and forward them to the remote server
pub struct ProxyVisitorSide {
    listener: TcpListener,
}

struct LoggingWriter<W> {
    tag: String,
    inner: W,
}

impl<W> LoggingWriter<W> {
    fn new(tag: String, inner: W) -> Self {
        LoggingWriter { tag, inner }
    }
}

impl<W: AsyncWrite + Unpin> AsyncWrite for LoggingWriter<W> {
    fn poll_write(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<std::result::Result<usize, std::io::Error>> {
        // if let Ok(s) = std::str::from_utf8(buf) {
        //     println!("{} Writing: {}", self.tag, s);
        // } else {
        println!("{} Writing: {:?}", self.tag, buf);
        // }
        Pin::new(&mut self.inner).poll_write(cx, buf)
    }

    fn poll_flush(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        Pin::new(&mut self.inner).poll_flush(cx)
    }

    fn poll_shutdown(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        Pin::new(&mut self.inner).poll_shutdown(cx)
    }

    // Implement other required methods...
}

impl ProxyVisitorSide {
    pub async fn new(bind_addr: &str) -> Self {
        let listener = TcpListener::bind(bind_addr).await.unwrap();
        ProxyVisitorSide { listener }
    }
    pub async fn run_with<R, T>(self, mut rx: R, mut tx: T)
    where
        T: 'static + AsyncWrite + Send + Unpin,
        R: 'static + AsyncRead + Send + Unpin,
    {
        let mut tx_binder = Some(tx);
        let mut rx_binder = Some(rx);
        loop {
            let tx: T = tx_binder.take().unwrap();
            let mut rx: R = rx_binder.take().unwrap();
            let (stream, _) = self.listener.accept().await.unwrap();
            let (mut read_tcp, mut write_back_2_tcp) = stream.into_split();
            println!("\n>>> new tcp connection");

            let (tcp_done_tx, tcp_done_rx) = tokio::sync::oneshot::channel();
            let t1 = tokio::spawn(async move {
                let mut logging = LoggingWriter::new("->visitor".to_owned(), tx);
                tokio::io::copy(&mut read_tcp, &mut logging).await.unwrap();
                println!(">>> end forward tcp to terminal");
                tcp_done_tx.send(()).unwrap_or_else(|err| {
                    panic!("{:?}", err);
                });
                logging.inner
            });

            let t2 = tokio::spawn(async move {
                let mut logging = LoggingWriter::new("visitor->".to_owned(), write_back_2_tcp);
                tokio::select! {
                    _=tokio::io::copy(&mut rx, &mut logging)=>{
                        println!(">>> end forward response to tcp by webterminal agent script");
                    },
                    _=tcp_done_rx=>{
                        println!(">>> end forward response to tcp by tcp");
                    }
                };
                rx
            });

            tx_binder = Some(t1.await.unwrap_or_else(|e| {
                panic!("{:?}", e);
            }));
            rx_binder = Some(t2.await.unwrap());
        }
    }
}

// connect to the real server, and forward msgs from remote
pub struct ProxyAgentSide {
    stream: tokio::net::TcpStream,
}

impl ProxyAgentSide {
    pub async fn new(remote_addr: &str) -> Self {
        let stream = tokio::net::TcpStream::connect(remote_addr)
            .await
            .unwrap_or_else(|e| {
                panic!("Failed to connect to remote server: {} {}", e, remote_addr);
            });
        ProxyAgentSide { stream }
    }
    pub async fn run_with<R, T>(self, mut rx: R, mut tx: T)
    where
        T: 'static + AsyncWrite + Send + Unpin,
        R: 'static + AsyncRead + Send + Unpin,
    {
        let (mut reader, mut writer) = self.stream.into_split();

        let t1 = tokio::spawn(async move {
            let mut logging = LoggingWriter::new("->agent".to_owned(), tx);
            tokio::io::copy(&mut reader, &mut logging).await.unwrap();
        });

        let t2 = tokio::spawn(async move {
            let mut logging = LoggingWriter::new("agent->".to_owned(), writer);
            tokio::io::copy(&mut rx, &mut logging).await.unwrap();
        });

        t1.await.unwrap();
        t2.await.unwrap();
    }
}
