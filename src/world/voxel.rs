use serde::{Serialize, Deserialize};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum BlockType {
    Air = 0,
    Grass = 1,
    Stone = 2,
    Dirt = 3,
    OakLog = 4,
    OakLeaves = 5,
    Sand = 6,
    Gravel = 7,
    CoalOre = 8,
    IronOre = 9,
    Glass = 10,
    Torch = 11,
    TorchWallN = 12,
    TorchWallS = 13,
    TorchWallE = 14,
    TorchWallW = 15,
}

impl BlockType {
    pub fn is_solid(self) -> bool {
        !matches!(self, BlockType::Air | BlockType::Torch | BlockType::TorchWallN | BlockType::TorchWallS | BlockType::TorchWallE | BlockType::TorchWallW)
    }

    pub fn is_opaque(self) -> bool {
        !matches!(self, BlockType::Air | BlockType::Glass | BlockType::OakLeaves | BlockType::Torch | BlockType::TorchWallN | BlockType::TorchWallS | BlockType::TorchWallE | BlockType::TorchWallW)
    }

    pub fn emitted_light(&self) -> u8 {
        match self {
            BlockType::Torch | BlockType::TorchWallN | BlockType::TorchWallS | BlockType::TorchWallE | BlockType::TorchWallW => 15,
            _ => 0,
        }
    }
    
    pub fn is_torch(self) -> bool {
        matches!(self, BlockType::Torch | BlockType::TorchWallN | BlockType::TorchWallS | BlockType::TorchWallE | BlockType::TorchWallW)
    }

    /// Returns (min, max) bounding box offsets from block origin.
    pub fn get_aabb_offsets(self) -> ([f32; 3], [f32; 3]) {
        match self {
            BlockType::Torch => ([0.375, 0.0, 0.375], [0.625, 0.625, 0.625]),
            BlockType::TorchWallN => ([0.35, 0.1, 0.0], [0.65, 0.8, 0.5]),
            BlockType::TorchWallS => ([0.35, 0.1, 0.5], [0.65, 0.8, 1.0]),
            BlockType::TorchWallE => ([0.5, 0.1, 0.35], [1.0, 0.8, 0.65]),
            BlockType::TorchWallW => ([0.0, 0.1, 0.35], [0.5, 0.8, 0.65]),
            _ => ([0.0, 0.0, 0.0], [1.0, 1.0, 1.0]),
        }
    }
}
