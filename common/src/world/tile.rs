#[derive(Debug, Clone, Copy)]
pub enum TileType {
    Empty,
    Floor,
}

#[derive(Debug, Clone, Copy)]
pub struct Tile {
    pub ty: TileType,
}
