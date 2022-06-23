use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};

pub mod world;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Position {
    pub x: usize,
    pub y: usize,
}

#[derive(Debug, Encode, Decode)]
pub struct Player {
    pub username: String,
}

#[derive(Clone, Copy, num_enum::TryFromPrimitive)]
#[repr(u8)]
pub enum ClientPacket {
    Join,
    Leave,
    KeepAlive,
    Chat,
    WorldClick,
}

#[derive(Clone, Copy, num_enum::TryFromPrimitive)]
#[repr(u8)]
pub enum ServerPacket {
    JoinResult,
    ClientJoin,  // sent to all clients when a client joins the server
    ClientLeave, // sent to all other clients when a client leaves the server
    Chat,
    ChunkModified,
}
