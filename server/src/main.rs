use std::{env, net::SocketAddr, sync::Arc};

use tokio::{net::UdpSocket, sync::Mutex, time};

pub type Error = Box<dyn std::error::Error + Send + Sync>;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub struct State {
    players: Vec<Option<common::world::Player>>,
    world: common::world::World,
}

impl State {
    pub fn new() -> Self {
        let players = std::iter::repeat_with(|| None)
            .take(MAX_CLIENTS)
            .collect::<Vec<_>>();

        let world = common::world::World::new(2, 2);

        Self { players, world }
    }
}

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

    let state = Arc::new(Mutex::new(State::new()));
    let state2 = state.clone();

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
    let s2 = s.clone();
    let s3 = s.clone();

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

                        let serialized_world = bincode::encode_to_vec(
                            &state.lock().await.world,
                            bincode::config::standard(),
                        )
                        .unwrap();

                        let mut data = vec![slot as u8];
                        data.extend(serialized_world.iter().copied());

                        send(&s2, addr, common::ServerPacket::JoinResult, data)
                            .await
                            .unwrap();

                        let split = buf.split_first().unwrap();

                        let username = std::str::from_utf8(split.1)
                            .unwrap()
                            .trim_matches(char::from(0))
                            .to_string();

                        let c = &mut clients2.lock().await;

                        c[slot as usize] = Some(Client {
                            addr,
                            last_heard: 0.0,
                        });

                        state.lock().await.players[slot as usize] =
                            Some(common::world::Player { username });

                        // inform all clients that a client joined the server
                        // sent to the new client aswell so they get the player list
                        broadcast(
                            &s,
                            None,
                            c,
                            common::ServerPacket::ClientJoin,
                            bincode::encode_to_vec(
                                state.lock().await.players.clone(),
                                bincode::config::standard(),
                            )
                            .unwrap(),
                        )
                        .await;
                    }
                }
                common::ClientPacket::Leave => {
                    let client_id = buf[1];

                    let c = &mut clients2.lock().await;

                    c[client_id as usize] = None;

                    state.lock().await.players[client_id as usize] = None;

                    // inform all clients that a client left the server
                    broadcast(
                        &s,
                        Some(client_id),
                        &c,
                        common::ServerPacket::ClientLeave,
                        bincode::encode_to_vec(
                            state.lock().await.players.clone(),
                            bincode::config::standard(),
                        )
                        .unwrap(),
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
                            let split = buf.split_first().unwrap();

                            // should always be some
                            if let Some(player) = &state.lock().await.players[client_id as usize] {
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

                    let c = &mut clients2.lock().await;

                    if let Some(client) = &mut c[client_id as usize] {
                        if verify_client(addr, client.addr) {
                            let deserialized_position: common::Position =
                                bincode::serde::decode_from_slice(
                                    split.1,
                                    bincode::config::standard(),
                                )
                                .unwrap()
                                .0;

                            let chunk_position = common::Position {
                                x: deserialized_position.x / common::world::CHUNK_SIZE,
                                y: deserialized_position.y / common::world::CHUNK_SIZE,
                            };

                            let tile_chunk_position = common::Position {
                                x: deserialized_position.x
                                    - common::world::CHUNK_SIZE
                                        * (deserialized_position.x / common::world::CHUNK_SIZE),
                                y: deserialized_position.y
                                    - common::world::CHUNK_SIZE
                                        * (deserialized_position.y / common::world::CHUNK_SIZE),
                            };

                            state
                                .lock()
                                .await
                                .world
                                .chunks
                                .get_mut((chunk_position.x, chunk_position.y))
                                .unwrap()
                                .tiles
                                .get_mut((tile_chunk_position.x, tile_chunk_position.y))
                                .unwrap()
                                .ty = common::world::TileType::Sand;

                            let serialized_chunk = bincode::serde::encode_to_vec(
                                &state
                                    .lock()
                                    .await
                                    .world
                                    .chunks
                                    .get((chunk_position.x, chunk_position.y))
                                    .unwrap(),
                                bincode::config::standard(),
                            )
                            .unwrap();

                            // send new world to all clients
                            broadcast(
                                &s,
                                None,
                                &c,
                                common::ServerPacket::ChunkModified,
                                serialized_chunk,
                            )
                            .await;
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

                    state2.lock().await.players[client_id] = None;

                    // inform all other clients that a client left the server
                    broadcast(
                        &s3,
                        Some(client_id as u8),
                        &c,
                        common::ServerPacket::ClientLeave,
                        bincode::encode_to_vec(
                            state2.lock().await.players.clone(),
                            bincode::config::standard(),
                        )?,
                    )
                    .await;
                }
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
