use serde::{Deserialize, Serialize};

pub mod world;

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
    ClientJoining, // sent to all other clients when a new client joins
    GameState,
}

pub fn serialize<T>(value: &T) -> Result<Vec<u8>, bincode::Error>
where
    T: ?Sized + Serialize,
{
    bincode::serialize(&value)
}

pub fn deserialize<'a, T>(bytes: &'a [u8]) -> Result<T, bincode::Error>
where
    T: Deserialize<'a>,
{
    bincode::deserialize(&bytes)
}
