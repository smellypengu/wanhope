use bincode::{Decode, Encode};

pub mod world;

#[derive(Debug, Encode, Decode)]
pub struct Position {
    pub x: f32,
    pub y: f32,
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
    ClientJoin,  // sent to all other clients when a client joins the server
    ClientLeave, // sent to all other clients when a client leaves the server
    Chat,
    GameState,
}
