use bincode::{Decode, Encode};
use noise::{
    utils::{NoiseMapBuilder, PlaneMapBuilder},
    Perlin, Seedable,
};

use crate::Position;

use super::{Chunk, Tile, TileType, CHUNK_SIZE};

#[derive(Debug, Clone, Encode, Decode)]
pub struct Player {
    pub username: String,
}

#[derive(Debug, Clone, Encode, Decode)]
pub struct World {
    #[bincode(with_serde)]
    pub chunks: ndarray::Array2<Chunk>,

    pub width: usize,
    pub height: usize,
}

impl World {
    pub fn new(width: usize, height: usize) -> Self {
        let mut chunks =
            ndarray::Array2::from_shape_fn((width, height), |(x, y)| Chunk::new(Position { x, y }));

        let perlin = Perlin::default().set_seed(rand::random());

        let map = PlaneMapBuilder::new(&perlin)
            .set_size(width * CHUNK_SIZE, height * CHUNK_SIZE)
            .set_x_bounds(0.0, 1.0)
            .set_y_bounds(0.0, 1.0)
            .build();

        for ((chunk_x, chunk_y), chunk) in chunks.indexed_iter_mut() {
            for ((tile_x, tile_y), tile) in chunk.tiles.indexed_iter_mut() {
                if map.get_value(chunk_x * CHUNK_SIZE + tile_x, chunk_y * CHUNK_SIZE + tile_y) > 0.2 {
                    tile.ty = TileType::Sand;
                }
            }
        }

        Self {
            chunks,

            width,
            height,
        }
    }
}
