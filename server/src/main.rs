use std::env;

use tokio::{net::{TcpListener, TcpStream}, io::{AsyncReadExt, AsyncWriteExt}};

pub type Error = Box<dyn std::error::Error + Send + Sync>;

pub type Result<T> = std::result::Result<T, Error>;

#[tokio::main]
async fn main() -> crate::Result<()> {
    let addr = env::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1:8080".to_string());

    let listener = TcpListener::bind(&addr).await?;
    println!("Listening on: {}", addr);

    loop {
        let (socket, _) = listener.accept().await?;
        tokio::spawn(async move {
            let _ = handle_client(socket).await;
        });
    }
}

async fn handle_client(mut socket: TcpStream) -> Result<()> {
    println!("Accepted connection from: {}", socket.peer_addr()?.ip());

    let mut buf = vec![0; 1024];

    // In a loop, read data from the socket and write the data back.
    loop {
        let n = socket
            .read(&mut buf)
            .await
            .expect("Failed to read data from socket");

        if n == 0 {
            break;
        }

        let msg: common::TestStruct = common::deserialize(&buf).unwrap();

        socket
            .write_all(&buf[0..n])
            .await
            .expect("Failed to write data to socket");

        println!("From the sender: {:?}", msg);
    }

    Ok(())
}
