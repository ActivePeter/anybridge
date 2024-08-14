use std::{
    pin::Pin,
    task::{Context, Poll},
};

use tokio::{
    io::{self, AsyncBufReadExt, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, ReadBuf},
    net::TcpListener,
    process::Command,
    sync::mpsc,
};

pub struct ReceiverStream {
    receiver: mpsc::Receiver<Vec<u8>>,
}

impl ReceiverStream {
    pub fn new(receiver: mpsc::Receiver<Vec<u8>>) -> Self {
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
    pub fn new(sender: mpsc::Sender<Vec<u8>>) -> Self {
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
