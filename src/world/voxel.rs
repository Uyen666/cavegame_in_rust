use serde::{Serialize, Deserialize};
use super::registry::{
    BLOCK_DEFINITIONS, BlockDefinition, ToolTier, ToolType, SoundCategory, DropTable, BlockTextureMapping
};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[repr(u8)]
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

#[allow(dead_code)]
impl BlockType {
    #[inline(always)]
    pub fn definition(self) -> &'static BlockDefinition {
        &BLOCK_DEFINITIONS[self as usize]
    }

    #[inline(always)]
    pub fn is_solid(self) -> bool {
        self.definition().is_solid
    }

    #[inline(always)]
    pub fn is_opaque(self) -> bool {
        self.definition().is_opaque
    }

    #[inline(always)]
    pub fn emitted_light(&self) -> u8 {
        self.definition().emitted_light
    }

    #[inline(always)]
    pub fn is_torch(self) -> bool {
        self.definition().is_torch
    }

    #[inline(always)]
    pub fn get_aabb_offsets(self) -> ([f32; 3], [f32; 3]) {
        self.definition().aabb_offsets
    }

    #[inline(always)]
    pub fn hardness(self) -> f32 {
        self.definition().hardness
    }

    #[inline(always)]
    pub fn required_tier(self) -> ToolTier {
        self.definition().required_tier
    }

    #[inline(always)]
    pub fn preferred_tool(self) -> ToolType {
        self.definition().preferred_tool
    }

    #[inline(always)]
    pub fn sound_category(self) -> SoundCategory {
        self.definition().sound_category
    }

    #[inline(always)]
    pub fn drop_table(self) -> DropTable {
        self.definition().drop_table
    }

    #[inline(always)]
    pub fn texture_mapping(self) -> BlockTextureMapping {
        self.definition().texture_mapping
    }
}
