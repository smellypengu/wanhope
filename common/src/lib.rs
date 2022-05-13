use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct TestStruct {
    pub x: u8,
    pub abc: String,
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
