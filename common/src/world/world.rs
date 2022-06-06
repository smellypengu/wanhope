use bincode::{Decode, Encode};

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

        let tiles = ndarray::Array2::from_elem(
            (width, height),
            Tile {
                ty: TileType::Empty,
            },
        );

        Self {
            players,

            tiles,
            width,
            height,
        }
    }
}
