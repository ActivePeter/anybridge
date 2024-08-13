use proxy::{ProxyAgentSide, ProxyVisitorSide};
use std::fmt::Debug;
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt};

mod proxy;
mod subprocess;

trait Endpoint {
    type RxStream: 'static + AsyncRead + Send + Unpin;
    type TxStream: 'static + AsyncWrite + Send + Unpin;
    async fn start(&self) -> (Self::RxStream, Self::TxStream);
}

#[tokio::main]
async fn main() {
    // arg should be one
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <subprocess_server|subprocess_client>", args[0]);
        std::process::exit(1);
    }

    match args[1].as_str() {
        "subprocess_server" => {
            let server = subprocess::SubprocessServerSide::new();
            let (rx, tx) = server.start().await;
            // handle_recv_and_send("subprocess_server".to_string(), rx, tx).await;
            ProxyAgentSide::new("127.0.0.1:22")
                .await
                .run_with(rx, tx)
                .await;
        }
        "subprocess_client" => {
            let client = subprocess::SubprocessClientSide::new();
            let (rx, tx) = client.start().await;
            ProxyVisitorSide::new("127.0.0.1:2233")
                .await
                .run_with(rx, tx)
                .await;
            // handle_recv_and_send("subprocess_client".to_string(), rx, tx).await;
        }
        _ => {
            eprintln!("Usage: {} <subprocess_server|subprocess_client>", args[0]);
            std::process::exit(1);
        }
    }
}

async fn handle_recv_and_send<R, T>(comment: String, recv: R, mut send: T)
where
    T: 'static + AsyncWrite + Send + Unpin,
    R: 'static + AsyncRead + Send + Unpin,
{
    println!("handle_recv_and_send {}", comment);
    tokio::spawn(async move {
        let mut recv = tokio::io::BufReader::new(recv);
        // let mut send = tokio::io::BufWriter::new(send);

        let mut line = String::new();
        loop {
            line.clear();
            let len = recv.read_line(&mut line).await.unwrap();
            if len == 0 {
                println!("EOF");
                break;
            }
            println!("{} recv: {}", comment, line);
            // send.write_all(line.as_bytes()).await.unwrap();
            // send.flush().await.unwrap();

            // line.clear();
        }
    });

    // continuous read shell input and send
    let mut line = String::new();
    let mut reader = tokio::io::BufReader::new(tokio::io::stdin());
    loop {
        reader.read_line(&mut line).await.unwrap();
        send.write_all(line.as_bytes()).await.unwrap();
        send.flush().await.unwrap();

        line.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_msg_trans_complete() {
        let server = subprocess::SubprocessServerSide::new();
        let (rx, mut tx) = server.start().await;
        let mut rx = tokio::io::BufReader::new(rx);
        let client = subprocess::SubprocessClientSide::new();
        let (rx2, mut tx2) = client.start().await;
        let mut rx2 = tokio::io::BufReader::new(rx2);
        // generate 4000 bytes
        let mut msg = String::new();
        for i in 0..8000 {
            // rand
            let randc = std::char::from_u32(rand::random::<u32>() % 26 + 65).unwrap();
            if i == 8000 - 1 {
                msg.push('\n');
            } else {
                msg.push(randc);
            }
        }
        tx.write_all(msg.as_bytes()).await.unwrap();
        let mut recvbuf = String::new();
        let recvlen = rx2.read_line(&mut recvbuf).await.unwrap();
        assert_eq!(recvbuf, msg);

        tx2.write_all(msg.as_bytes()).await.unwrap();
        recvbuf.clear();
        let recvlen = rx.read_line(&mut recvbuf).await.unwrap();
        assert_eq!(recvbuf, msg);

        tx.write_all(msg.as_bytes()).await.unwrap();
        recvbuf.clear();
        let recvlen = rx2.read_line(&mut recvbuf).await.unwrap();
        assert_eq!(recvbuf, msg);

        tx2.write_all(msg.as_bytes()).await.unwrap();
        recvbuf.clear();
        let recvlen = rx.read_line(&mut recvbuf).await.unwrap();
        assert_eq!(recvbuf, msg);
    }
}
