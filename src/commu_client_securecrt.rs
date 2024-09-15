use std::{
    collections::binary_heap,
    pin::Pin,
    process::Stdio,
    sync::{Arc, Mutex},
    task::{Context, Poll},
};

use crate::{
    stream::{ReceiverStream, SenderStream},
    Endpoint,
};
use base64::{engine::general_purpose::STANDARD, DecodeSliceError};
use base64::{prelude::*, Engine};
use futures_util::{FutureExt, SinkExt, StreamExt};
use http_body_util::Full;
use hyper::{server::conn::http1, service::service_fn};
use hyper_util::rt::TokioIo;
use std::net::SocketAddr;
use tokio::{
    io::{self, AsyncBufReadExt, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, ReadBuf},
    net::TcpListener,
    process::Command,
    sync::mpsc,
};
use tokio_tungstenite::{accept_async, tungstenite::Message};

// client side is just who created subprocess
// the subprocess is agent.py
// send data to server by write to subprocess stdin
// read data from server by read from subprocess stdout
pub(crate) struct CommuClientSideSecureCrt {}

impl CommuClientSideSecureCrt {
    pub fn new() -> Self {
        CommuClientSideSecureCrt {}
    }
}

// async fn hello(_: Request<hyper::body::Incoming>) -> Result<Response<Full<Bytes>>, Infallible> {
//     Ok(Response::new(Full::new(Bytes::from("Hello, World!"))))
// }

impl Endpoint for CommuClientSideSecureCrt {
    // type RxStream = StdOutBase64Wrapper;
    // type TxStream = StdInBase64Wrapper;
    type RxStream = ReceiverStream;
    type TxStream = SenderStream;

    async fn start(&self) -> (Self::RxStream, Self::TxStream) {
        // start http server to handle secureCRT
        let (send_2_remote_tx, mut send_2_remote_rx) = mpsc::channel::<Vec<u8>>(100);
        let (recv_remote_tx, recv_remote_rx) = mpsc::channel::<Vec<u8>>(100);

        let send_2_remote_rx = Arc::new(Mutex::new(send_2_remote_rx));
        tokio::spawn(async move {
            let addr: SocketAddr = SocketAddr::from(([127, 0, 0, 1], 8091));

            // We create a TcpListener and bind it to 127.0.0.1:3000
            let listener = TcpListener::bind(addr).await.unwrap();

            // let mut send_2_remote_rx_opt = Some(send_2_remote_rx);
            // We start a loop to continuously accept incoming connections
            loop {
                let (stream, _) = listener.accept().await.unwrap();
                let send_2_remote_rx = send_2_remote_rx.clone();
                let recv_remote_tx = recv_remote_tx.clone();
                // let mut send_2_remote_rx = send_2_remote_rx_opt.take().unwrap();

                // Use an adapter to access something implementing `tokio::io` traits as if they implement
                // `hyper::rt` IO traits.
                let io = TokioIo::new(stream);

                // Spawn a tokio task to serve multiple connections concurrently

                // Finally, we bind the incoming connection to our `hello` service
                if let Err(err) = http1::Builder::new()
                    // `service_fn` converts our function in a `Service`
                    .serve_connection(
                        io,
                        service_fn(move |req| {
                            // let body = req.body();
                            let mut send = None;
                            if req.uri().path() != "/" {
                                println!(
                                    ">>> pull data from remote to proxy target {}",
                                    req.uri().path()
                                );
                                // parse [1:] by base64
                                let bytes =
                                    STANDARD.decode(req.uri().path()[1..].as_bytes()).unwrap();
                                // recv_remote_tx.send(bytes).await;
                                send = Some(bytes);
                            }

                            // println!(">>> term script heartbeat to visitor proxy, {:?}", req);
                            let mut send_2_remote_rx = send_2_remote_rx.lock().unwrap();
                            let forward_2_visitor = if let Ok(a) = send_2_remote_rx.try_recv() {
                                println!(">>> forward data to remote {:?}", a);
                                a
                            } else {
                                vec![]
                            };
                            let recv_remote_tx = recv_remote_tx.clone();
                            async move {
                                if let Some(send) = send {
                                    // send to remote
                                    recv_remote_tx.send(send).await;
                                }
                                Ok::<_, hyper::Error>(hyper::Response::new(Full::new(
                                    hyper::body::Bytes::from(forward_2_visitor),
                                )))
                            }
                        }),
                    )
                    .await
                {
                    eprintln!("Error serving connection: {:?}", err);
                }
            }
        });
        // // Create a TCP listener on port 8087
        // let addr = "127.0.0.1:8088";
        // let listener = TcpListener::bind(addr).await.unwrap();
        // println!("WebSocket server listening on ws://{}", addr);

        // // Prepare send to remote channel and receive from remote channel
        // let (send_2_remote_tx, send_2_remote_rx) = mpsc::channel::<Vec<u8>>(100);
        // let (recv_remote_tx, recv_remote_rx) = mpsc::channel::<Vec<u8>>(100);

        // tokio::spawn(async move {
        //     let mut send_2_remote_rx_opt = Some(send_2_remote_rx);
        //     // recycle the recv_remote_rx
        //     while let Ok((stream, _)) = listener.accept().await {
        //         let mut ws_stream = accept_async(stream)
        //             .await
        //             .expect("Error during WebSocket handshake");
        //         let (mut ws_sender, mut ws_receiver) = ws_stream.split();

        //         // Spawn a task to handle receiving messages from WebSocket
        //         let recv_remote_tx = recv_remote_tx.clone();
        //         let t1 = tokio::spawn(async move {
        //             loop {
        //                 match ws_receiver.next().await {
        //                     Some(Ok(msg)) => {
        //                         let binary = match msg {
        //                             Message::Text(text) => {
        //                                 panic!("Unexpected text message: {}", text)
        //                             }
        //                             Message::Binary(binary) => binary,
        //                             Message::Ping(ping) => {
        //                                 continue;
        //                             }
        //                             Message::Pong(_) => {
        //                                 continue;
        //                             }
        //                             Message::Close(_) => {
        //                                 println!(">>> commu client agent side: recv ws close");
        //                                 break;
        //                             }
        //                         };
        //                         // println!(
        //                         //     "server recv {:?}",
        //                         //     std::str::from_utf8(binary.as_slice())
        //                         // );
        //                         recv_remote_tx
        //                             .send(binary)
        //                             .await
        //                             .expect("Failed to send message");
        //                     }
        //                     _ => {
        //                         println!("WebSocket error");
        //                         break;
        //                     } // Ok(0) => break,
        //                       // Ok(n) => {
        //                       //     let text = String::from_utf8_lossy(&buf[..n]).to_string();
        //                       //     recv_remote_tx
        //                       //         .send(text)
        //                       //         .await
        //                       //         .expect("Failed to send message");
        //                       // }
        //                       // Err(e) => eprintln!("WebSocket error: {:?}", e),
        //                 }
        //             }
        //         });

        //         // Spawn a task to handle sending messages to WebSocket
        //         let mut send_2_remote_rx = send_2_remote_rx_opt.take().unwrap();
        //         let t2 = tokio::spawn(async move {
        //             // read from send_2_remote_rx

        //             while let Some(message) = send_2_remote_rx.recv().await {
        //                 println!("server send {:?}", std::str::from_utf8(message.as_slice()));
        //                 if let Err(e) = ws_sender.send(Message::Binary(message)).await {
        //                     eprintln!("WebSocket send error: {:?}", e);
        //                 }
        //             }

        //             send_2_remote_rx
        //         });

        //         let _ = t1.await;
        //         let send_2_remote_rx = t2.await.unwrap();
        //         send_2_remote_rx_opt = Some(send_2_remote_rx);
        //     }
        // });

        (
            ReceiverStream::new(recv_remote_rx),
            SenderStream::new(send_2_remote_tx),
        )
    }
}
