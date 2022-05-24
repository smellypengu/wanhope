use std::net::{SocketAddr, UdpSocket};

use crate::app::AppError;

pub struct Network {
    socket: Option<UdpSocket>,
    pub connected: bool,
}

impl Network {
    pub fn new() -> Self {
        Self {
            socket: None,
            connected: false,
        }
    }

    pub fn connect(&mut self) -> anyhow::Result<(), AppError> {
        if !self.connected {
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

                let join_result =
                    common::ServerMessage::try_from(response[0]).unwrap();

                match join_result {
                    common::ServerMessage::JoinResult => {
                        if len > 1 as usize {
                            println!("user id: {}", response[1]);

                            self.connected = true;
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

    pub fn update(&self) -> anyhow::Result<Option<common::ServerMessage>, AppError> {
        match &self.socket {
            Some(socket) => {
                let mut data = vec![0u8; 1_024];

                // set to nonblocking so recv() call doesn't freeze app, probably temporary
                socket.set_nonblocking(true)?;

                match socket.recv(&mut data) {
                    Ok(_len) => {
                        return Ok(common::ServerMessage::try_from(data[0]).ok());
                    }
                    Err(_) => {}
                }

                // let test_struct = common::TestStruct { x: 100, abc: "lol".to_string() };
                // let msg = common::serialize(&test_struct).unwrap();

                // socket.send(&msg).unwrap();
                // let mut data = vec![0u8; 1_000];
                // let len = socket.recv(&mut data).unwrap();

                // let deserialized: common::TestStruct = common::deserialize(&data[..len]).unwrap();

                // println!(
                //     "Received {} bytes:\n{:?}",
                //     len,
                //     deserialized,
                // );
            }
            None => {}
        }

        Ok(None)
    }

    pub fn server_ip(&self) -> Option<SocketAddr> {
        self.socket.as_ref().unwrap().peer_addr().ok()
    }
}
