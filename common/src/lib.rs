// use serde::{Serialize, Deserialize};

pub mod world;

pub enum ClientMessage {
    Join,
    Leave,
}

impl TryFrom<u8> for ClientMessage {
    type Error = ();

    fn try_from(val: u8) -> Result<Self, ()> {
        match val {
            0 => Ok(ClientMessage::Join),
            1 => Ok(ClientMessage::Leave),
            _ => Err(()),
        }
    }
}

pub enum ServerMessage {
    JoinResult,
    ClientJoining, // sent to all other clients when a new client joins
}

impl TryFrom<u8> for ServerMessage {
    type Error = ();

    fn try_from(val: u8) -> Result<Self, ()> {
        match val {
            0 => Ok(ServerMessage::JoinResult),
            1 => Ok(ServerMessage::ClientJoining),
            _ => Err(()),
        }
    }
}

// #[derive(Serialize, Deserialize, PartialEq, Debug)]
// pub struct TestStruct {
//     pub x: u8,
//     pub abc: String,
// }

// pub fn serialize<T>(value: &T) -> Result<Vec<u8>, bincode::Error>
// where
//     T: ?Sized + Serialize,
// {
//     bincode::serialize(&value)
// }

// pub fn deserialize<'a, T>(bytes: &'a [u8]) -> Result<T, bincode::Error>
// where
//     T: Deserialize<'a>,
// {
//     bincode::deserialize(&bytes)
// }
