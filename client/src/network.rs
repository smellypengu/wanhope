use std::{
    io,
    net::{SocketAddr, UdpSocket},
};

pub struct Network {
    pub ip: String,
    socket: Option<UdpSocket>,

    pub connected: bool,
    pub client_id: Option<u8>,
    pub username: String,

    keep_alive_timer: i16,
}

impl Network {
    pub fn new() -> Self {
        Self {
            ip: "127.0.0.1:8080".to_string(),
            socket: None,

            connected: false,
            client_id: None,
            username: "".to_string(),

            keep_alive_timer: 0,
        }
    }

    pub fn join(&mut self) -> anyhow::Result<(), NetworkError> {
        if !self.connected {
            if self.username != "".to_string() {
                let remote_addr: SocketAddr = self.ip.parse().unwrap();

                let local_addr: SocketAddr = if remote_addr.is_ipv4() {
                    "0.0.0.0:0"
                } else {
                    "[::]:0"
                }
                .parse()
                .unwrap();

                self.socket = UdpSocket::bind(local_addr).ok();

                if let Some(socket) = &self.socket {
                    socket.connect(remote_addr)?;

                    // register as client in server

                    let mut send = vec![common::ClientPacket::Join as u8];
                    send.extend(&mut self.username.as_bytes().iter().copied());

                    socket.send(&send)?;

                    let mut response = vec![0u8; 2];
                    let len = socket.recv(&mut response)?;

                    let join_result = common::ServerPacket::try_from(response[0]).unwrap();

                    match join_result {
                        common::ServerPacket::JoinResult => {
                            if len > 1 as usize {
                                println!("user id: {}", response[1]);

                                self.connected = true;
                                self.client_id = Some(response[1]);
                            } else {
                                log::info!("Server did not let us in");
                            }
                        }
                        _ => {}
                    }
                }
            } else {
                return Err(NetworkError::EmptyUsername);
            }
        }

        Ok(())
    }

    pub fn leave(&mut self) -> anyhow::Result<(), NetworkError> {
        if self.connected {
            if let Some(socket) = &self.socket {
                socket.send(&[common::ClientPacket::Leave as u8, self.client_id.unwrap()])?;

                self.socket = None;

                self.connected = false;
                self.client_id = None;
            }
        }

        Ok(())
    }

    pub fn update(
        &mut self,
    ) -> anyhow::Result<(Option<common::ServerPacket>, Vec<u8>), NetworkError> {
        if self.connected {
            if let Some(socket) = &self.socket {
                // set to nonblocking so recv() call doesn't freeze app, probably temporary
                socket.set_nonblocking(true)?;

                // TODO: figure out a better way to do this timer
                if self.keep_alive_timer == 500 {
                    socket.send(&[
                        common::ClientPacket::KeepAlive as u8,
                        self.client_id.unwrap(),
                    ])?;
                    self.keep_alive_timer = 0;
                }

                self.keep_alive_timer += 1;

                let mut data = vec![0u8; 1_024];

                match socket.recv(&mut data) {
                    Ok(len) => {
                        let x = data[..len].to_vec();
                        let message = common::ServerPacket::try_from(x[0]).ok();
                        let payload = x.split_at(1).1.to_vec();

                        return Ok((message, payload));
                    }
                    Err(_) => {}
                }
            }
        }

        Ok((None, Vec::new()))
    }

    pub fn send_chat_message(&self, message: &String) -> anyhow::Result<(), NetworkError> {
        if self.connected {
            if let Some(socket) = &self.socket {
                let mut send = vec![common::ClientPacket::Chat as u8, self.client_id.unwrap()];

                send.extend(message.as_bytes().iter().copied());

                socket.send(&send)?;
            }
        }

        Ok(())
    }

    pub fn send_client_world_click(
        &self,
        position: glam::Vec2,
    ) -> anyhow::Result<(), NetworkError> {
        if self.connected {
            if let Some(socket) = &self.socket {
                let mut send = vec![
                    common::ClientPacket::WorldClick as u8,
                    self.client_id.unwrap(),
                ];

                let p = common::Position {
                    x: position.x,
                    y: position.y,
                };

                send.extend(
                    &mut bincode::encode_to_vec(p, bincode::config::standard())
                        .unwrap()
                        .iter()
                        .copied(),
                );

                socket.send(&send)?;

                // get a result?
            }
        }

        Ok(())
    }

    pub fn server_ip(&self) -> Option<SocketAddr> {
        self.socket.as_ref().unwrap().peer_addr().ok()
    }
}

#[derive(thiserror::Error, Debug)]
pub enum NetworkError {
    #[error("")]
    NetworkError(#[from] io::Error),
    #[error("Username is empty")]
    EmptyUsername,
}
