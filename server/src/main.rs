use std::{env, net::SocketAddr, sync::Arc};

use tokio::{
    net::UdpSocket,
    sync::{mpsc, Mutex},
    time,
};

pub type Error = Box<dyn std::error::Error + Send + Sync>;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
struct Client {
    username: Option<String>,
    addr: Option<SocketAddr>,
    last_heard: f32,
}

const TICKS_PER_SECOND: usize = 60;
const SECONDS_PER_TICK: f32 = 1.0 / TICKS_PER_SECOND as f32;
const MAX_CLIENTS: usize = 32;
const CLIENT_TIMEOUT: f32 = 5.0;

#[tokio::main]
async fn main() -> crate::Result<()> {
    simple_logger::SimpleLogger::new()
        .without_timestamps()
        .init()
        .unwrap();

    let world = Arc::new(Mutex::new(common::world::World::new(10, 10)));
    let world2 = world.clone();

    let clients = Arc::new(Mutex::new(
        std::iter::repeat_with(|| Client {
            username: None,
            addr: None,
            last_heard: 0.0,
        })
        .take(MAX_CLIENTS)
        .collect::<Vec<_>>(),
    ));

    let addr = env::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1:8080".to_string());

    let socket = UdpSocket::bind(&addr).await?;
    println!("Listening on: {}", socket.local_addr()?);

    let r = Arc::new(socket);
    let s = r.clone();
    let s2 = s.clone();

    let (tx, mut rx) = mpsc::channel::<(SocketAddr, common::ServerPacket, Vec<u8>)>(1_000);

    tokio::spawn(async move {
        while let Some((addr, packet, data)) = rx.recv().await {
            send(&s, addr, packet, data).await.unwrap();
        }
    });

    let clients2 = clients.clone();

    tokio::spawn(async move {
        loop {
            let mut buf = [0; 1024];

            let (len, addr) = r.recv_from(&mut buf).await.unwrap();
            log::debug!("{} bytes received from {}", len, addr);

            match common::ClientPacket::try_from(buf[0]).unwrap() {
                common::ClientPacket::Join => {
                    let mut slot = -1;

                    for i in 0..MAX_CLIENTS {
                        if clients2.lock().await[i].addr.is_none() {
                            slot = i as i8;
                            break;
                        }
                    }

                    if slot != -1 {
                        log::info!("client will be assigned to slot: {}", slot);

                        if tx
                            .send((addr, common::ServerPacket::JoinResult, vec![slot as u8]))
                            .await
                            .is_err()
                        {
                            // TODO: handle better
                            log::warn!("Failed to send");
                        }

                        let split = buf.split_at(1);

                        let username = std::str::from_utf8(split.1).unwrap().to_string();

                        clients2.lock().await[slot as usize] = Client {
                            username: Some(username),
                            addr: Some(addr),
                            last_heard: 0.0,
                        };

                        // inform all other clients that a new client joined
                        for i in 0..MAX_CLIENTS {
                            if i != slot as usize {
                                if let Some(client_addr) = clients2.lock().await[i].addr {
                                    if tx
                                        .send((
                                            client_addr,
                                            common::ServerPacket::ClientJoining,
                                            vec![],
                                        ))
                                        .await
                                        .is_err()
                                    {
                                        // TODO: handle better
                                        log::warn!("Failed to send");
                                    }
                                }
                            }
                        }
                    }
                }
                common::ClientPacket::Leave => {
                    let client_id = buf[1];

                    if let Some(client_addr) = clients2.lock().await[client_id as usize].addr {
                        if addr.ip() == client_addr.ip() {
                            clients2.lock().await[client_id as usize] = Client {
                                username: None,
                                addr: None,
                                last_heard: 0.0,
                            };
                        } else {
                            log::warn!("leave message from {} expected {}", addr, client_addr);
                        }
                    }
                }
                common::ClientPacket::KeepAlive => {
                    let client_id = buf[1];

                    clients2.lock().await[client_id as usize].last_heard = 0.0;
                }
                common::ClientPacket::WorldClick => {
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

    let clients3 = clients.clone();

    let mut interval = time::interval(time::Duration::from_secs_f32(SECONDS_PER_TICK));

    loop {
        interval.tick().await;

        for (i, client) in clients3.lock().await.iter_mut().enumerate() {
            if client.addr.is_some() {
                client.last_heard += SECONDS_PER_TICK;

                if client.last_heard > CLIENT_TIMEOUT {
                    log::warn!("client {} timed out", i);

                    clients3.lock().await[i] = Client {
                        username: None,
                        addr: None,
                        last_heard: 0.0,
                    }
                }
            }
        }

        let serialized_world = common::serialize(&*world2.lock().await)?;

        // probably not best practise to send the entire world each tick?
        for client in clients3.lock().await.iter() {
            if let Some(client_addr) = client.addr {
                send(
                    &s2,
                    client_addr,
                    common::ServerPacket::GameState,
                    serialized_world.clone(),
                )
                .await?;
            }
        }
    }
}

async fn send(
    socket: &UdpSocket,
    addr: SocketAddr,
    packet: common::ServerPacket,
    data: Vec<u8>,
) -> crate::Result<()> {
    let mut bytes = vec![packet as u8];
    bytes.extend(data.iter().copied());

    let len = socket.send_to(&bytes, &addr).await?;
    log::debug!("{} bytes sent", len);

    Ok(())
}
