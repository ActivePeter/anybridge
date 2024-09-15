use std::{
    collections::binary_heap,
    pin::Pin,
    process::Stdio,
    task::{Context, Poll},
};

use crate::{
    stream::{ReceiverStream, SenderStream},
    Endpoint,
};
use base64::{engine::general_purpose::STANDARD, DecodeSliceError};
use base64::{prelude::*, Engine};
use futures_util::{FutureExt, SinkExt, StreamExt};
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
pub(crate) struct CommuClientSideWebterminal {}

impl CommuClientSideWebterminal {
    pub fn new() -> Self {
        CommuClientSideWebterminal {}
    }
}

impl Endpoint for CommuClientSideWebterminal {
    // type RxStream = StdOutBase64Wrapper;
    // type TxStream = StdInBase64Wrapper;
    type RxStream = ReceiverStream;
    type TxStream = SenderStream;

    async fn start(&self) -> (Self::RxStream, Self::TxStream) {
        // Create a TCP listener on port 8087
        let addr = "127.0.0.1:8091";
        let listener = TcpListener::bind(addr).await.unwrap();
        println!("WebSocket server listening on ws://{}", addr);

        // Prepare send to remote channel and receive from remote channel
        let (send_2_remote_tx, send_2_remote_rx) = mpsc::channel::<Vec<u8>>(100);
        let (recv_remote_tx, recv_remote_rx) = mpsc::channel::<Vec<u8>>(100);

        tokio::spawn(async move {
            let mut send_2_remote_rx_opt = Some(send_2_remote_rx);
            // recycle the recv_remote_rx
            while let Ok((stream, _)) = listener.accept().await {
                let mut ws_stream = accept_async(stream)
                    .await
                    .expect("Error during WebSocket handshake");
                let (mut ws_sender, mut ws_receiver) = ws_stream.split();

                // Spawn a task to handle receiving messages from WebSocket
                let recv_remote_tx = recv_remote_tx.clone();
                let t1 = tokio::spawn(async move {
                    loop {
                        match ws_receiver.next().await {
                            Some(Ok(msg)) => {
                                let binary = match msg {
                                    Message::Text(text) => {
                                        panic!("Unexpected text message: {}", text)
                                    }
                                    Message::Binary(binary) => binary,
                                    Message::Ping(ping) => {
                                        continue;
                                    }
                                    Message::Pong(_) => {
                                        continue;
                                    }
                                    Message::Close(_) => {
                                        println!(">>> commu client agent side: recv ws close");
                                        break;
                                    }
                                };
                                if binary.is_empty() {
                                    continue;
                                }
                                println!(
                                    "read crt -> proxied tcp, str:{:?}, bin:{:?}",
                                    std::str::from_utf8(binary.as_slice()),
                                    binary.as_slice()
                                );
                                recv_remote_tx
                                    .send(binary)
                                    .await
                                    .expect("Failed to send message");
                            }
                            _ => {
                                println!("WebSocket error");
                                break;
                            } // Ok(0) => break,
                              // Ok(n) => {
                              //     let text = String::from_utf8_lossy(&buf[..n]).to_string();
                              //     recv_remote_tx
                              //         .send(text)
                              //         .await
                              //         .expect("Failed to send message");
                              // }
                              // Err(e) => eprintln!("WebSocket error: {:?}", e),
                        }
                    }
                });

                // Spawn a task to handle sending messages to WebSocket
                let mut send_2_remote_rx = send_2_remote_rx_opt.take().unwrap();
                let t2 = tokio::spawn(async move {
                    // read from send_2_remote_rx

                    while let Some(message) = send_2_remote_rx.recv().await {
                        println!("server send {:?}", std::str::from_utf8(message.as_slice()));
                        if let Err(e) = ws_sender.send(Message::Binary(message)).await {
                            eprintln!("WebSocket send error: {:?}", e);
                        }
                    }

                    send_2_remote_rx
                });

                let _ = t1.await;
                let send_2_remote_rx = t2.await.unwrap();
                send_2_remote_rx_opt = Some(send_2_remote_rx);
            }
        });

        (
            ReceiverStream::new(recv_remote_rx),
            SenderStream::new(send_2_remote_tx),
        )
    }
}
