use bincode::{Decode, Encode};
use noise::{
    utils::{NoiseMapBuilder, PlaneMapBuilder},
    Perlin, Seedable,
};

use super::{Tile, TileType};

#[derive(Debug, Encode, Decode)]
pub struct Player {
    pub username: String,
}

#[derive(Debug, Encode, Decode)]
pub struct World {
    pub players: Vec<Option<Player>>,

    #[bincode(with_serde)]
    pub tiles: ndarray::Array2<Tile>,
    pub width: usize,
    pub height: usize,
}

impl World {
    pub fn new(max_players: usize, width: usize, height: usize) -> Self {
        let players = std::iter::repeat_with(|| None)
            .take(max_players)
            .collect::<Vec<_>>();

        let mut tiles = ndarray::Array2::from_elem(
            (width, height),
            Tile {
                ty: TileType::Grass,
            },
        );

        let perlin = Perlin::default().set_seed(rand::random());

        let map = PlaneMapBuilder::new(&perlin)
            .set_size(width, height)
            .set_x_bounds(-0.5, 0.5)
            .set_y_bounds(-0.5, 0.5)
            .build();

        for ((x, y), tile) in tiles.indexed_iter_mut() {
            if map.get_value(x, y) > 0.5 {
                tile.ty = TileType::Sand;
            }
        }

        Self {
            players,

            tiles,
            width,
            height,
        }
    }
}
