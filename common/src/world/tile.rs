use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum TileType {
    Empty,
    Floor,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub struct Tile {
    pub ty: TileType,
}
