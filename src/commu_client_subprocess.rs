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
pub(crate) struct CommuClientSideSubprocess {}

impl CommuClientSideSubprocess {
    pub fn new() -> Self {
        CommuClientSideSubprocess {}
    }
}

impl Endpoint for CommuClientSideSubprocess {
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
            });

            let exit = cmd.wait().await.unwrap();

            println!("Client Side Subprocess exited {:?}", exit);
        });

        (
            ReceiverStream::new(recv_remote_rx),
            SenderStream::new(send_2_other_tx),
        )
    }
}
