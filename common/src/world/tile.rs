use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TileType {
    Grass,
    Sand,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Tile {
    pub ty: TileType,
}
