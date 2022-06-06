use std::{
    io,
    net::{SocketAddr, UdpSocket},
};

pub struct Network {
    socket: Option<UdpSocket>,

    pub username: String,
    pub client_id: Option<u8>,

    keep_alive_timer: i16,
}

impl Network {
    pub fn new() -> Self {
        Self {
            socket: None,

            username: "".to_string(),
            client_id: None,

            keep_alive_timer: 0,
        }
    }

    pub fn join(&mut self) -> anyhow::Result<(), NetworkError> {
        if self.client_id.is_none() {
            if self.username != "".to_string() {
                let remote_addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();

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
        match &self.socket {
            Some(socket) => {
                socket.send(&[common::ClientPacket::Leave as u8, self.client_id.unwrap()])?;

                self.socket = None;
                self.client_id = None;
            }
            None => {}
        }

        Ok(())
    }

    pub fn update(
        &mut self,
    ) -> anyhow::Result<(Option<common::ServerPacket>, Vec<u8>), NetworkError> {
        match &self.socket {
            Some(socket) => {
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
            None => {}
        }

        Ok((None, Vec::new()))
    }

    pub fn send_chat_message(&self, message: &String) -> anyhow::Result<(), NetworkError> {
        match &self.socket {
            Some(socket) => {
                let mut send = vec![
                    common::ClientPacket::Chat as u8,
                    self.client_id.unwrap(),
                ];

                send.extend(message.as_bytes().iter().copied());

                socket.send(&send)?;
            }
            None => {}
        }

        Ok(())
    } 

    pub fn send_client_world_click(
        &self,
        position: glam::Vec2,
    ) -> anyhow::Result<(), NetworkError> {
        match &self.socket {
            Some(socket) => {
                let mut send = vec![
                    common::ClientPacket::WorldClick as u8,
                    self.client_id.unwrap(),
                ];

                send.extend(
                    &mut bincode::serde::encode_to_vec(&position, bincode::config::standard())
                        .unwrap()
                        .iter()
                        .copied(),
                );

                socket.send(&send)?;

                // get a result?
            }
            None => {}
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
