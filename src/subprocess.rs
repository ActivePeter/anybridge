use std::{
    collections::binary_heap,
    pin::Pin,
    process::Stdio,
    task::{Context, Poll},
};

use crate::Endpoint;
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
pub(crate) struct SubprocessClientSide {}

impl SubprocessClientSide {
    pub fn new() -> Self {
        SubprocessClientSide {}
    }
}

// pub struct StdInBase64Wrapper {
//     inner: tokio::io::BufWriter<tokio::process::ChildStdin>,
// }

// impl StdInBase64Wrapper {
//     fn new(inner: tokio::io::BufWriter<tokio::process::ChildStdin>) -> Self {
//         StdInBase64Wrapper { inner }
//     }
// }

// impl AsyncWrite for StdInBase64Wrapper {
//     fn poll_write(
//         mut self: Pin<&mut Self>,
//         cx: &mut Context<'_>,
//         buf: &[u8],
//     ) -> Poll<Result<usize, io::Error>> {
//         let mut buf2 = STANDARD.encode(buf) + "\n";
//         print!("encoded {:?} to {:?}", buf, buf2);
//         // buf.push('\n' as u8);
//         Pin::new(&mut self.inner).poll_write(cx, buf2.as_bytes())
//     }

//     fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
//         Pin::new(&mut self.inner).poll_flush(cx)
//     }

//     fn poll_shutdown(
//         mut self: Pin<&mut Self>,
//         cx: &mut Context<'_>,
//     ) -> Poll<Result<(), io::Error>> {
//         Pin::new(&mut self.inner).poll_shutdown(cx)
//     }
// }

// pub struct StdOutBase64Wrapper {
//     inner: tokio::io::BufReader<tokio::process::ChildStdout>,
// }

// impl StdOutBase64Wrapper {
//     fn new(inner: tokio::io::BufReader<tokio::process::ChildStdout>) -> Self {
//         StdOutBase64Wrapper { inner }
//     }
// }

// impl AsyncRead for StdOutBase64Wrapper {
//     fn poll_read(
//         mut self: Pin<&mut Self>,
//         cx: &mut Context<'_>,
//         buf: &mut ReadBuf<'_>,
//     ) -> Poll<Result<(), io::Error>> {
//         match Pin::new(&mut self.inner).poll_read(cx, buf) {
//             Poll::Ready(Ok(ok)) => {
//                 // Process the data read from the inner reader

//                 // Attempt to find a newline in the accumulated buffer
//                 if let Some(pos) = self.inner.buffer().iter().position(|&b| b == b'\n') {
//                     // Decode the data up to the newline
//                     let linelen = {
//                         let (line, rest) = self.inner.buffer().split_at(pos);
//                         // self.inner.buffer() = rest[1..].to_vec().into(); // Update buffer to be the rest after newline

//                         // Decode from Base64
//                         let decoded = STANDARD
//                             .decode(line)
//                             .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
//                         // Copy the decoded data into the provided buffer
//                         println!("decoded: {:?}", decoded);
//                         buf.put_slice(&decoded);
//                         line.len()
//                     };

//                     Pin::new(&mut self.inner).consume(linelen + 1);

//                     Poll::Ready(Ok(()))
//                 } else {
//                     Poll::Pending
//                 }
//             }
//             Poll::Ready(Err(err)) => Poll::Ready(Err(err)),
//             Poll::Pending => Poll::Pending,
//         }
//     }
// }

impl Endpoint for SubprocessClientSide {
    // type RxStream = StdOutBase64Wrapper;
    // type TxStream = StdInBase64Wrapper;
    type RxStream = ReceiverStream;
    type TxStream = SenderStream;

    async fn start(&self) -> (Self::RxStream, Self::TxStream) {
        let mut cmd = Command::new("python3")
            .arg("agent2.py")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Failed to start subprocess");

        let stdin = cmd.stdin.take().expect("Failed to open stdin");
        let stdout = cmd.stdout.take().expect("Failed to open stdout");
        // Prepare send to remote channel and receive from remote channel
        let (send_2_other_tx, mut send_2_other_rx) = mpsc::channel::<Vec<u8>>(100);
        let (recv_other_tx, recv_remote_rx) = mpsc::channel::<Vec<u8>>(100);

        tokio::spawn(async move {
            // write to stdin
            tokio::spawn(async move {
                let mut stdin = tokio::io::BufWriter::new(stdin);
                loop {
                    let msg = send_2_other_rx.recv().await.unwrap();
                    let mut buf = STANDARD.encode(msg);
                    buf.push('\n');

                    stdin.write_all(buf.as_bytes()).await.unwrap();
                    stdin.flush().await.unwrap();
                }
            });

            // read from stdout
            tokio::spawn(async move {
                let mut stdout = tokio::io::BufReader::new(stdout);

                //read lines
                let mut line = String::new();
                loop {
                    line.clear();
                    let len = stdout.read_line(&mut line).await.unwrap();
                    if len == 0 {
                        println!("EOF");
                        break;
                    }
                    let decoded = STANDARD.decode(&line.as_bytes()[..len - 1]).unwrap();
                    println!("decoded: {:?}", decoded);
                    recv_other_tx.send(decoded).await.unwrap();
                }

                // let mut buf = vec![0u8; 1024];
                // let mut offset = 0;
                // loop {
                //     let len = stdout.read(&mut buf[offset..]).await.unwrap();
                //     if len == 0 {
                //         println!("EOF");
                //         break;
                //     }
                //     // print!(
                //     //     "client recv server base64 {:?}",
                //     //     std::str::from_utf8(&buf).unwrap()
                //     // );

                //     let Ok(decoded) = STANDARD.decode(&buf[0..len]) else {
                //         // expand the buffer
                //         offset += len;
                //         buf.resize(buf.len() * 2, 0);
                //         continue;
                //     };
                //     println!("decoded: {:?}", decoded);
                //     recv_other_tx.send(decoded).await.unwrap();
                // }
            });

            let exit = cmd.wait().await.unwrap();

            println!("Client Side Subprocess exited {:?}", exit);
        });
        // tokio::spawn(async move {
        //     let mut stdin = tokio::io::BufWriter::new(stdin);
        //     let mut stdout = tokio::io::BufReader::new(stdout);

        //     let mut line = String::new();
        //     loop {
        //         tokio::io::stdin().read_line(&mut line).await.unwrap();
        //         stdin.write_all(line.as_bytes()).await.unwrap();
        //         stdin.flush().await.unwrap();

        //         line.clear();
        //         stdout.read_line(&mut line).await.unwrap();
        //         print!("{}", line);
        //         line.clear();
        //     }
        // });

        (
            ReceiverStream::new(recv_remote_rx),
            SenderStream::new(send_2_other_tx),
        )
    }
}

// server side start a websocket server
// agent will send stdin to server
// agent will write server msg to stdout (print)
pub(crate) struct SubprocessServerSide {}

impl SubprocessServerSide {
    pub fn new() -> Self {
        SubprocessServerSide {}
    }
}

impl Endpoint for SubprocessServerSide {
    type RxStream = ReceiverStream;
    type TxStream = SenderStream;

    async fn start(&self) -> (Self::RxStream, Self::TxStream) {
        // Create a TCP listener on port 8087
        let listener = TcpListener::bind("127.0.0.1:8087").await.unwrap();
        println!("WebSocket server listening on ws://127.0.0.1:8087");

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
                                    Message::Close(_) => todo!(),
                                };
                                println!(
                                    "server recv {:?}",
                                    std::str::from_utf8(binary.as_slice())
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

pub struct ReceiverStream {
    receiver: mpsc::Receiver<Vec<u8>>,
}

impl ReceiverStream {
    fn new(receiver: mpsc::Receiver<Vec<u8>>) -> Self {
        ReceiverStream { receiver }
    }
}

impl Drop for ReceiverStream {
    fn drop(&mut self) {
        println!("ReceiverStream dropped");
    }
}

impl AsyncRead for ReceiverStream {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let mut receiver = Pin::new(&mut self.get_mut().receiver);
        match receiver.poll_recv(cx) {
            Poll::Ready(Some(msg)) => {
                buf.put_slice(&msg);
                Poll::Ready(Ok(()))
            }
            Poll::Ready(None) => Poll::Ready(Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "Channel closed",
            ))),
            Poll::Pending => Poll::Pending,
        }
    }
}

pub struct SenderStream {
    sender: mpsc::Sender<Vec<u8>>,
    failed: Vec<u8>,
}

impl SenderStream {
    fn new(sender: mpsc::Sender<Vec<u8>>) -> Self {
        SenderStream {
            sender,
            failed: vec![],
        }
    }
}

impl Drop for SenderStream {
    fn drop(&mut self) {
        println!("SenderStream dropped");
    }
}

impl AsyncWrite for SenderStream {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<Result<usize, std::io::Error>> {
        // let msg = String::from_utf8_lossy(buf).to_string();
        if self.failed.len() > 0 {
            println!(
                "SenderStream failed with {:?}, sending {:?}",
                self.failed, buf
            );
        }
        match self.sender.try_send(buf.to_owned()) {
            Ok(()) => Poll::Ready(Ok(buf.len())),
            Err(_) => {
                self.get_mut().failed.extend_from_slice(buf);
                Poll::Pending
            }
        }
    }

    fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), std::io::Error>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(
        self: Pin<&mut Self>,
        _: &mut Context<'_>,
    ) -> Poll<Result<(), std::io::Error>> {
        Poll::Ready(Ok(()))
    }
}
