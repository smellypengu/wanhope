use serde::{Deserialize, Serialize};

use super::{Tile, TileType};

#[derive(Serialize, Deserialize)]
pub struct World {
    pub tiles: ndarray::Array2<Tile>,
    pub width: usize,
    pub height: usize,
}

impl World {
    pub fn new(width: usize, height: usize) -> Self {
        let tiles = ndarray::Array2::from_elem(
            (width, height),
            Tile {
                ty: TileType::Empty,
            },
        );

        Self {
            tiles,
            width,
            height,
        }
    }
}
