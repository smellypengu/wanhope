use std::{env, net::SocketAddr, sync::Arc};

use tokio::{
    net::UdpSocket,
    sync::{mpsc, Mutex},
    time,
};

pub type Error = Box<dyn std::error::Error + Send + Sync>;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, Copy)]
struct Client {
    addr: Option<SocketAddr>,
    last_heard: f32,
}

const TICKS_PER_SECOND: usize = 60;
const SECONDS_PER_TICK: f32 = 1.0 / TICKS_PER_SECOND as f32;
const MAX_CLIENTS: usize = 32;
const CLIENT_TIMEOUT: f32 = 5.0;

#[tokio::main]
async fn main() -> crate::Result<()> {
    let world = Arc::new(Mutex::new(common::world::World::new(10, 10)));
    let world2 = world.clone();

    let clients = Arc::new(Mutex::new(
        [Client {
            addr: None,
            last_heard: 0.0,
        }; MAX_CLIENTS],
    ));

    let c = clients.clone();

    let addr = env::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1:8080".to_string());

    let socket = UdpSocket::bind(&addr).await?;
    println!("Listening on: {}", socket.local_addr()?);

    let r = Arc::new(socket);
    let s = r.clone();
    let s2 = s.clone();

    let (tx, mut rx) = mpsc::channel::<(Vec<u8>, SocketAddr)>(1_000);

    tokio::spawn(async move {
        while let Some((bytes, addr)) = rx.recv().await {
            send(&s, addr, bytes).await.unwrap();
        }
    });

    tokio::spawn(async move {
        loop {
            let mut buf = [0; 1024];

            let (len, addr) = r.recv_from(&mut buf).await.unwrap();
            println!("{} bytes received from {}", len, addr);

            match common::ClientMessage::try_from(buf[0]).unwrap() {
                common::ClientMessage::Join => {
                    let mut slot = -1;

                    for i in 0..MAX_CLIENTS {
                        if c.lock().await[i].addr.is_none() {
                            slot = i as i8;
                            break;
                        }
                    }

                    let mut response = Vec::new();
                    response.insert(0, common::ServerMessage::JoinResult as u8);

                    if slot != -1 {
                        println!("client will be assigned to slot: {}", slot);

                        response.insert(1, slot as u8);
                        tx.send((response, addr)).await.unwrap();

                        c.lock().await[slot as usize] = Client {
                            addr: Some(addr),
                            last_heard: 0.0,
                        };

                        // inform all other clients that a new client joined
                        for i in 0..MAX_CLIENTS {
                            if i != slot as usize {
                                if let Some(addr) = c.lock().await[i].addr {
                                    tx.send((
                                        [common::ServerMessage::ClientJoining as u8].to_vec(),
                                        addr,
                                    ))
                                    .await
                                    .unwrap();
                                }
                            }
                        }
                    }
                }
                common::ClientMessage::Leave => {
                    let client_id = buf[1];

                    c.lock().await[client_id as usize] = Client {
                        addr: None,
                        last_heard: 0.0,
                    }
                }
                common::ClientMessage::KeepAlive => {
                    let client_id = buf[1];

                    c.lock().await[client_id as usize].last_heard = 0.0;
                }
                common::ClientMessage::WorldRequest => {
                    tx.send((common::serialize(&*world.lock().await).unwrap(), addr))
                        .await
                        .unwrap();
                }
                common::ClientMessage::WorldClick => {
                    let split = buf.split_at(2);

                    let client_id = split.0[1];

                    let deserialized_position: glam::Vec2 = common::deserialize(split.1).unwrap();

                    world
                        .lock()
                        .await
                        .tiles
                        .get_mut((
                            deserialized_position.x as usize,
                            deserialized_position.y as usize,
                        ))
                        .unwrap()
                        .ty = common::world::TileType::Floor;

                    // send a result?
                }
            }
        }
    });

    let clients2 = clients.clone();

    let mut interval = time::interval(time::Duration::from_secs_f32(SECONDS_PER_TICK));

    loop {
        interval.tick().await;

        for (i, client) in clients2.lock().await.iter_mut().enumerate() {
            if client.addr.is_some() {
                client.last_heard += SECONDS_PER_TICK;

                if client.last_heard > CLIENT_TIMEOUT {
                    println!("client {} timed out", i);

                    clients2.lock().await[i] = Client {
                        addr: None,
                        last_heard: 0.0,
                    }
                }
            }
        }

        let serialized_world = common::serialize(&*world2.lock().await)?;

        // probably not best practise to send the entire world each tick?
        for client in clients2.lock().await.iter() {
            if let Some(addr) = client.addr {
                let mut response = vec![common::ServerMessage::GameState as u8];
                response.extend(serialized_world.iter().copied());

                send(&s2, addr, response).await?;
            }
        }
    }
}

async fn send(socket: &UdpSocket, addr: SocketAddr, bytes: Vec<u8>) -> crate::Result<()> {
    let len = socket.send_to(&bytes, &addr).await?;
    println!("{} bytes sent", len);

    Ok(())
}
