use std::net::{SocketAddr, UdpSocket};

use crate::app::AppError;

pub struct Network {
    socket: Option<UdpSocket>,
    pub client_id: Option<u8>,

    keep_alive_timer: i16,
}

impl Network {
    pub fn new() -> Self {
        Self {
            socket: None,
            client_id: None,
            keep_alive_timer: 0,
        }
    }

    pub fn connect(&mut self) -> anyhow::Result<(), AppError> {
        if self.client_id.is_none() {
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
                socket.send(&[common::ClientMessage::Join as u8])?;

                let mut response = vec![0u8; 2];
                let len = socket.recv(&mut response)?;

                let join_result = common::ServerMessage::try_from(response[0]).unwrap();

                match join_result {
                    common::ServerMessage::JoinResult => {
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
        }

        Ok(())
    }

    pub fn update(&mut self) -> anyhow::Result<(Option<common::ServerMessage>, Vec<u8>), AppError> {
        match &self.socket {
            Some(socket) => {
                // set to nonblocking so recv() call doesn't freeze app, probably temporary
                socket.set_nonblocking(true)?;

                // TODO: figure out a better way to do this timer
                if self.keep_alive_timer == 500 {
                    socket.send(&[common::ClientMessage::KeepAlive as u8])?;
                    self.keep_alive_timer = 0;
                }

                self.keep_alive_timer += 1;

                let mut data = vec![0u8; 1_024];

                match socket.recv(&mut data) {
                    Ok(len) => {
                        let x = data[..len].to_vec();
                        let message = common::ServerMessage::try_from(x[0]).ok();
                        let payload = x.split_at(1).1.to_vec();

                        return Ok((message, payload));
                    }
                    Err(_) => {}
                }
            }
            None => {}
        }

        Ok((None, vec![]))
    }

    pub fn send_client_world_click(&self, position: glam::Vec2) -> anyhow::Result<(), AppError> {
        match &self.socket {
            Some(socket) => {
                let mut send = vec![
                    common::ClientMessage::WorldClick as u8,
                    self.client_id.unwrap(),
                ];

                send.extend(&mut common::serialize(&position).unwrap().iter().copied());

                socket.send(&send)?;

                // get result
            }
            None => {}
        }

        Ok(())
    }

    pub fn server_ip(&self) -> Option<SocketAddr> {
        self.socket.as_ref().unwrap().peer_addr().ok()
    }
}
