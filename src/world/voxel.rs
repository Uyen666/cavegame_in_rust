use serde::{Serialize, Deserialize};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum BlockType {
    Air = 0,
    Grass = 1,
    Stone = 2,
    Dirt = 3,
}

impl BlockType {
    pub fn is_solid(self) -> bool {
        self != BlockType::Air
    }
}
