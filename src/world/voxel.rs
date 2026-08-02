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
}
