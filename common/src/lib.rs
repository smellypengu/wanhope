use serde::{Deserialize, Serialize};

pub mod world;

#[derive(Serialize, Deserialize)]
pub struct Player {
    pub username: String,
}

#[derive(num_enum::TryFromPrimitive)]
#[repr(u8)]
pub enum ClientPacket {
    Join,
    Leave,
    KeepAlive,
    WorldClick,
}

#[derive(num_enum::TryFromPrimitive)]
#[repr(u8)]
pub enum ServerPacket {
    JoinResult,
    ClientJoin, // sent to all other clients when a client joins the server
    ClientLeave, // sent to all other clients when a client leaves the server
    GameState,
}
