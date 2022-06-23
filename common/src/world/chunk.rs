use serde::{Deserialize, Serialize};

use crate::Position;

use super::{Tile, TileType};

pub const CHUNK_SIZE: usize = 12;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chunk {
    pub position: Position,

    pub tiles: ndarray::Array2<Tile>,
}

impl Chunk {
    pub fn new(position: Position) -> Self {
        let tiles = ndarray::Array2::from_elem(
            (CHUNK_SIZE, CHUNK_SIZE),
            Tile {
                ty: TileType::Grass,
            },
        );

        Self { position, tiles }
    }
}
