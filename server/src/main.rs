use std::{env, net::SocketAddr, sync::Arc};

use tokio::{net::UdpSocket, sync::mpsc};

pub type Error = Box<dyn std::error::Error + Send + Sync>;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, Copy)]
struct Client {
    addr: Option<SocketAddr>,
}

const MAX_CLIENTS: usize = 32;

#[tokio::main]
async fn main() -> crate::Result<()> {
    let mut clients = [Client { addr: None }; MAX_CLIENTS];

    let addr = env::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1:8080".to_string());

    let socket = UdpSocket::bind(&addr).await?;
    println!("Listening on: {}", socket.local_addr()?);

    let r = Arc::new(socket);
    let s = r.clone();
    let (tx, mut rx) = mpsc::channel::<(Vec<u8>, SocketAddr)>(1_000);

    tokio::spawn(async move {
        while let Some((bytes, addr)) = rx.recv().await {
            let len = s.send_to(&bytes, &addr).await.unwrap();
            println!("{:?} bytes sent", len);
        }
    });

    let mut buf = [0; 1024];
    loop {
        let (len, addr) = r.recv_from(&mut buf).await?;
        println!("{:?} bytes received from {:?}", len, addr);

        match common::ClientMessage::try_from(buf[0]).unwrap() {
            common::ClientMessage::Join => {
                let mut slot = -1;

                for i in 0..MAX_CLIENTS {
                    if clients[i].addr.is_none() {
                        slot = i as i8;
                        break;
                    }
                }

                let mut response = Vec::new();
                response.insert(0, common::ServerMessage::JoinResult as u8);

                if slot != -1 {
                    println!("client will be assigned to slot: {}", slot);

                    response.insert(1, slot as u8);
                    tx.send((response, addr)).await?;

                    clients[slot as usize] = Client { addr: Some(addr) };

                    // inform all other clients that a new client joined
                    for i in 0..MAX_CLIENTS {
                        if i != slot as usize {
                            if let Some(addr) = clients[i].addr {
                                tx.send((
                                    [common::ServerMessage::ClientJoining as u8].to_vec(),
                                    addr,
                                ))
                                .await?;
                            }
                        }
                    }
                }
            }
            common::ClientMessage::Leave => {
                let client_index = buf[1];

                clients[client_index as usize] = Client { addr: None }
            }
            common::ClientMessage::WorldClick => {
                let deserialized_position: glam::Vec2 = common::deserialize(buf.split_first().unwrap().1)?;

                println!("{}", deserialized_position);

                // send result
            }
        }
    }
}
