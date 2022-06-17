use std::{env, net::SocketAddr, sync::Arc};

use tokio::{net::UdpSocket, sync::Mutex, time};

pub type Error = Box<dyn std::error::Error + Send + Sync>;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
struct Client {
    addr: SocketAddr,
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
        .init()?;

    let world = Arc::new(Mutex::new(common::world::World::new(MAX_CLIENTS, 10, 10)));
    let world2 = world.clone();

    let clients = Arc::new(Mutex::new(
        std::iter::repeat_with(|| None)
            .take(MAX_CLIENTS)
            .collect::<Vec<_>>(),
    ));

    let addr = env::args()
        .nth(1)
        .unwrap_or_else(|| "0.0.0.0:8080".to_string());

    let socket = UdpSocket::bind(&addr).await?;
    println!("Listening on: {}", socket.local_addr()?);

    let s = Arc::new(socket);
    let s1 = s.clone();
    let s2 = s.clone();

    let clients2 = clients.clone();

    tokio::spawn(async move {
        loop {
            let mut buf = [0; 1024];

            let (len, addr) = s.recv_from(&mut buf).await.unwrap();
            log::debug!("{} bytes received from {}", len, addr);

            match common::ClientPacket::try_from(buf[0]).unwrap() {
                common::ClientPacket::Join => {
                    let mut slot = -1;

                    for i in 0..MAX_CLIENTS {
                        if clients2.lock().await[i].is_none() {
                            slot = i as i8;
                            break;
                        }
                    }

                    if slot != -1 {
                        log::info!("client will be assigned to slot: {}", slot);

                        send(
                            &s1,
                            addr,
                            common::ServerPacket::JoinResult,
                            vec![slot as u8],
                        )
                        .await
                        .unwrap();

                        let split = buf.split_at(1);

                        let username = std::str::from_utf8(split.1)
                            .unwrap()
                            .trim_matches(char::from(0))
                            .to_string();

                        let c = &mut clients2.lock().await;

                        c[slot as usize] = Some(Client {
                            addr,
                            last_heard: 0.0,
                        });

                        world.lock().await.players[slot as usize] =
                            Some(common::world::Player { username });

                        // inform all other clients that a client joined the server
                        broadcast(
                            &s,
                            Some(slot as u8),
                            c,
                            common::ServerPacket::ClientJoin,
                            Vec::new(),
                        )
                        .await;
                    }
                }
                common::ClientPacket::Leave => {
                    let client_id = buf[1];

                    let c = &mut clients2.lock().await;

                    c[client_id as usize] = None;

                    world.lock().await.players[client_id as usize] = None;

                    // inform all other clients that a client left the server
                    broadcast(
                        &s,
                        Some(client_id),
                        &c,
                        common::ServerPacket::ClientLeave,
                        Vec::new(),
                    )
                    .await;
                }
                common::ClientPacket::KeepAlive => {
                    let client_id = buf[1];

                    if let Some(client) = &mut clients2.lock().await[client_id as usize] {
                        if verify_client(addr, client.addr) {
                            client.last_heard = 0.0;
                        }
                    }
                }
                common::ClientPacket::Chat => {
                    let client_id = buf[1];

                    let c = &mut clients2.lock().await;

                    if let Some(client) = &mut c[client_id as usize] {
                        if verify_client(addr, client.addr) {
                            let split = buf.split_at(1);

                            // should always be some
                            if let Some(player) = &world.lock().await.players[client_id as usize] {
                                let message = player.username.clone()
                                    + ": "
                                    + std::str::from_utf8(split.1).unwrap();

                                // send chat message to all clients
                                broadcast(
                                    &s,
                                    None,
                                    &c,
                                    common::ServerPacket::Chat,
                                    message.as_bytes().to_vec(),
                                )
                                .await;
                            }
                        }
                    }
                }
                common::ClientPacket::WorldClick => {
                    let split = buf.split_at(2);

                    let client_id = split.0[1];

                    if let Some(client) = &mut clients2.lock().await[client_id as usize] {
                        if verify_client(addr, client.addr) {
                            let deserialized_position: common::Position =
                                bincode::decode_from_slice(split.1, bincode::config::standard())
                                    .unwrap()
                                    .0;

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
            }
        }
    });

    let clients3 = clients.clone();

    let mut interval = time::interval(time::Duration::from_secs_f32(SECONDS_PER_TICK));

    loop {
        interval.tick().await;

        for (client_id, client) in clients3.lock().await.iter_mut().enumerate() {
            if let Some(client) = client {
                client.last_heard += SECONDS_PER_TICK;

                if client.last_heard > CLIENT_TIMEOUT {
                    log::warn!("client {} timed out", client_id);

                    let c = &mut clients3.lock().await;

                    c[client_id] = None;

                    world2.lock().await.players[client_id] = None;

                    // inform all other clients that a client left the server
                    broadcast(
                        &s2,
                        Some(client_id as u8),
                        &c,
                        common::ServerPacket::ClientLeave,
                        Vec::new(),
                    )
                    .await;
                }
            }
        }

        let serialized_world =
            bincode::encode_to_vec(&*world2.lock().await, bincode::config::standard())?;

        // probably not best practise to send the entire world each tick?
        for client in clients3.lock().await.iter() {
            if let Some(client) = client {
                send(
                    &s2,
                    client.addr,
                    common::ServerPacket::GameState,
                    serialized_world.clone(),
                )
                .await?;
            }
        }
    }
}

fn verify_client(addr1: SocketAddr, addr2: SocketAddr) -> bool {
    if addr1.ip() == addr2.ip() {
        return true;
    }

    log::warn!("message from {} expected {}", addr1, addr2);
    return false;
}

async fn broadcast(
    socket: &UdpSocket,
    client_id: Option<u8>,
    clients: &Vec<Option<Client>>,
    packet: common::ServerPacket,
    data: Vec<u8>,
) {
    for i in 0..MAX_CLIENTS {
        if let Some(client_id) = client_id {
            if i == client_id as usize {
                continue;
            }
        }

        if let Some(client) = &clients[i] {
            if send(&socket, client.addr, packet, data.clone())
                .await
                .is_err()
            {
                // TODO: handle better
                log::warn!("Failed to send");
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
