#![allow(dead_code)]
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use super::voxel::BlockType;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ToolTier {
    None = 0,
    Wood = 1,
    Stone = 2,
    Iron = 3,
    Diamond = 4,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToolType {
    None,
    Pickaxe,
    Shovel,
    Axe,
    Sword,
    Hoe,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SoundCategory {
    None,
    Grass,
    Dirt,
    Stone,
    Wood,
    Gravel,
    Sand,
    Glass,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DropTable {
    None,
    SelfDrop,
    ItemDrop { item_id: u16, count: u8 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockTextureMapping {
    Single(u32),
    TopBottomSide { top: u32, bottom: u32, side: u32 },
    SixFaces([u32; 6]), // [+X (East), -X (West), +Y (Top), -Y (Bottom), +Z (North), -Z (South)]
}

impl BlockTextureMapping {
    #[inline(always)]
    pub fn get_layer(&self, d: usize, normal: i32) -> u32 {
        match self {
            BlockTextureMapping::Single(layer) => *layer,
            BlockTextureMapping::TopBottomSide { top, bottom, side } => {
                match (d, normal) {
                    (1, 1) => *top,
                    (1, -1) => *bottom,
                    _ => *side,
                }
            }
            BlockTextureMapping::SixFaces(faces) => {
                let face_id = (d * 2) + if normal > 0 { 0 } else { 1 };
                faces[face_id]
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct BlockDefinition {
    pub id: BlockType,
    pub name: &'static str,
    pub is_solid: bool,
    pub is_opaque: bool,
    pub emitted_light: u8,
    pub hardness: f32,
    pub required_tier: ToolTier,
    pub preferred_tool: ToolType,
    pub drop_table: DropTable,
    pub sound_category: SoundCategory,
    pub aabb_offsets: ([f32; 3], [f32; 3]),
    pub is_torch: bool,
    pub texture_mapping: BlockTextureMapping,
}

/// 全域 Static 數據驅動 Block 定義表 (0 鎖定開銷直尋)
pub static BLOCK_DEFINITIONS: [BlockDefinition; 16] = [
    // 0: Air
    BlockDefinition {
        id: BlockType::Air,
        name: "Air",
        is_solid: false,
        is_opaque: false,
        emitted_light: 0,
        hardness: 0.0,
        required_tier: ToolTier::None,
        preferred_tool: ToolType::None,
        drop_table: DropTable::None,
        sound_category: SoundCategory::None,
        aabb_offsets: ([0.0, 0.0, 0.0], [1.0, 1.0, 1.0]),
        is_torch: false,
        texture_mapping: BlockTextureMapping::Single(0),
    },
    // 1: Grass
    BlockDefinition {
        id: BlockType::Grass,
        name: "Grass",
        is_solid: true,
        is_opaque: true,
        emitted_light: 0,
        hardness: 0.6,
        required_tier: ToolTier::None,
        preferred_tool: ToolType::Shovel,
        drop_table: DropTable::SelfDrop,
        sound_category: SoundCategory::Grass,
        aabb_offsets: ([0.0, 0.0, 0.0], [1.0, 1.0, 1.0]),
        is_torch: false,
        texture_mapping: BlockTextureMapping::TopBottomSide { top: 2, bottom: 1, side: 3 },
    },
    // 2: Stone
    BlockDefinition {
        id: BlockType::Stone,
        name: "Stone",
        is_solid: true,
        is_opaque: true,
        emitted_light: 0,
        hardness: 1.5,
        required_tier: ToolTier::Wood,
        preferred_tool: ToolType::Pickaxe,
        drop_table: DropTable::SelfDrop,
        sound_category: SoundCategory::Stone,
        aabb_offsets: ([0.0, 0.0, 0.0], [1.0, 1.0, 1.0]),
        is_torch: false,
        texture_mapping: BlockTextureMapping::Single(0),
    },
    // 3: Dirt
    BlockDefinition {
        id: BlockType::Dirt,
        name: "Dirt",
        is_solid: true,
        is_opaque: true,
        emitted_light: 0,
        hardness: 0.5,
        required_tier: ToolTier::None,
        preferred_tool: ToolType::Shovel,
        drop_table: DropTable::SelfDrop,
        sound_category: SoundCategory::Dirt,
        aabb_offsets: ([0.0, 0.0, 0.0], [1.0, 1.0, 1.0]),
        is_torch: false,
        texture_mapping: BlockTextureMapping::Single(1),
    },
    // 4: OakLog
    BlockDefinition {
        id: BlockType::OakLog,
        name: "Oak Log",
        is_solid: true,
        is_opaque: true,
        emitted_light: 0,
        hardness: 2.0,
        required_tier: ToolTier::None,
        preferred_tool: ToolType::Axe,
        drop_table: DropTable::SelfDrop,
        sound_category: SoundCategory::Wood,
        aabb_offsets: ([0.0, 0.0, 0.0], [1.0, 1.0, 1.0]),
        is_torch: false,
        texture_mapping: BlockTextureMapping::TopBottomSide { top: 6, bottom: 6, side: 5 },
    },
    // 5: OakLeaves
    BlockDefinition {
        id: BlockType::OakLeaves,
        name: "Oak Leaves",
        is_solid: true,
        is_opaque: false,
        emitted_light: 0,
        hardness: 0.2,
        required_tier: ToolTier::None,
        preferred_tool: ToolType::None,
        drop_table: DropTable::SelfDrop,
        sound_category: SoundCategory::Grass,
        aabb_offsets: ([0.0, 0.0, 0.0], [1.0, 1.0, 1.0]),
        is_torch: false,
        texture_mapping: BlockTextureMapping::Single(7),
    },
    // 6: Sand
    BlockDefinition {
        id: BlockType::Sand,
        name: "Sand",
        is_solid: true,
        is_opaque: true,
        emitted_light: 0,
        hardness: 0.5,
        required_tier: ToolTier::None,
        preferred_tool: ToolType::Shovel,
        drop_table: DropTable::SelfDrop,
        sound_category: SoundCategory::Sand,
        aabb_offsets: ([0.0, 0.0, 0.0], [1.0, 1.0, 1.0]),
        is_torch: false,
        texture_mapping: BlockTextureMapping::Single(8),
    },
    // 7: Gravel
    BlockDefinition {
        id: BlockType::Gravel,
        name: "Gravel",
        is_solid: true,
        is_opaque: true,
        emitted_light: 0,
        hardness: 0.6,
        required_tier: ToolTier::None,
        preferred_tool: ToolType::Shovel,
        drop_table: DropTable::SelfDrop,
        sound_category: SoundCategory::Gravel,
        aabb_offsets: ([0.0, 0.0, 0.0], [1.0, 1.0, 1.0]),
        is_torch: false,
        texture_mapping: BlockTextureMapping::Single(9),
    },
    // 8: CoalOre
    BlockDefinition {
        id: BlockType::CoalOre,
        name: "Coal Ore",
        is_solid: true,
        is_opaque: true,
        emitted_light: 0,
        hardness: 3.0,
        required_tier: ToolTier::Wood,
        preferred_tool: ToolType::Pickaxe,
        drop_table: DropTable::SelfDrop,
        sound_category: SoundCategory::Stone,
        aabb_offsets: ([0.0, 0.0, 0.0], [1.0, 1.0, 1.0]),
        is_torch: false,
        texture_mapping: BlockTextureMapping::Single(10),
    },
    // 9: IronOre
    BlockDefinition {
        id: BlockType::IronOre,
        name: "Iron Ore",
        is_solid: true,
        is_opaque: true,
        emitted_light: 0,
        hardness: 3.0,
        required_tier: ToolTier::Stone,
        preferred_tool: ToolType::Pickaxe,
        drop_table: DropTable::SelfDrop,
        sound_category: SoundCategory::Stone,
        aabb_offsets: ([0.0, 0.0, 0.0], [1.0, 1.0, 1.0]),
        is_torch: false,
        texture_mapping: BlockTextureMapping::Single(11),
    },
    // 10: Glass
    BlockDefinition {
        id: BlockType::Glass,
        name: "Glass",
        is_solid: true,
        is_opaque: false,
        emitted_light: 0,
        hardness: 0.3,
        required_tier: ToolTier::None,
        preferred_tool: ToolType::None,
        drop_table: DropTable::None,
        sound_category: SoundCategory::Glass,
        aabb_offsets: ([0.0, 0.0, 0.0], [1.0, 1.0, 1.0]),
        is_torch: false,
        texture_mapping: BlockTextureMapping::Single(12),
    },
    // 11: Torch
    BlockDefinition {
        id: BlockType::Torch,
        name: "Torch",
        is_solid: false,
        is_opaque: false,
        emitted_light: 15,
        hardness: 0.0,
        required_tier: ToolTier::None,
        preferred_tool: ToolType::None,
        drop_table: DropTable::SelfDrop,
        sound_category: SoundCategory::Wood,
        aabb_offsets: ([0.375, 0.0, 0.375], [0.625, 0.625, 0.625]),
        is_torch: true,
        texture_mapping: BlockTextureMapping::Single(13),
    },
    // 12: TorchWallN
    BlockDefinition {
        id: BlockType::TorchWallN,
        name: "Torch Wall North",
        is_solid: false,
        is_opaque: false,
        emitted_light: 15,
        hardness: 0.0,
        required_tier: ToolTier::None,
        preferred_tool: ToolType::None,
        drop_table: DropTable::SelfDrop,
        sound_category: SoundCategory::Wood,
        aabb_offsets: ([0.35, 0.1, 0.0], [0.65, 0.8, 0.5]),
        is_torch: true,
        texture_mapping: BlockTextureMapping::Single(13),
    },
    // 13: TorchWallS
    BlockDefinition {
        id: BlockType::TorchWallS,
        name: "Torch Wall South",
        is_solid: false,
        is_opaque: false,
        emitted_light: 15,
        hardness: 0.0,
        required_tier: ToolTier::None,
        preferred_tool: ToolType::None,
        drop_table: DropTable::SelfDrop,
        sound_category: SoundCategory::Wood,
        aabb_offsets: ([0.35, 0.1, 0.5], [0.65, 0.8, 1.0]),
        is_torch: true,
        texture_mapping: BlockTextureMapping::Single(13),
    },
    // 14: TorchWallE
    BlockDefinition {
        id: BlockType::TorchWallE,
        name: "Torch Wall East",
        is_solid: false,
        is_opaque: false,
        emitted_light: 15,
        hardness: 0.0,
        required_tier: ToolTier::None,
        preferred_tool: ToolType::None,
        drop_table: DropTable::SelfDrop,
        sound_category: SoundCategory::Wood,
        aabb_offsets: ([0.5, 0.1, 0.35], [1.0, 0.8, 0.65]),
        is_torch: true,
        texture_mapping: BlockTextureMapping::Single(13),
    },
    // 15: TorchWallW
    BlockDefinition {
        id: BlockType::TorchWallW,
        name: "Torch Wall West",
        is_solid: false,
        is_opaque: false,
        emitted_light: 15,
        hardness: 0.0,
        required_tier: ToolTier::None,
        preferred_tool: ToolType::None,
        drop_table: DropTable::SelfDrop,
        sound_category: SoundCategory::Wood,
        aabb_offsets: ([0.0, 0.1, 0.35], [0.5, 0.8, 0.65]),
        is_torch: true,
        texture_mapping: BlockTextureMapping::Single(13),
    },
];

#[derive(Resource, Debug, Clone, Default)]
pub struct BlockRegistry;

impl BlockRegistry {
    pub fn get(&self, block: BlockType) -> &'static BlockDefinition {
        &BLOCK_DEFINITIONS[block as usize]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_block_definitions_coverage() {
        let registry = BlockRegistry;
        let blocks = [
            BlockType::Air,
            BlockType::Grass,
            BlockType::Stone,
            BlockType::Dirt,
            BlockType::OakLog,
            BlockType::OakLeaves,
            BlockType::Sand,
            BlockType::Gravel,
            BlockType::CoalOre,
            BlockType::IronOre,
            BlockType::Glass,
            BlockType::Torch,
            BlockType::TorchWallN,
            BlockType::TorchWallS,
            BlockType::TorchWallE,
            BlockType::TorchWallW,
        ];

        for block in blocks {
            let def = block.definition();
            assert_eq!(def.id, block);
            assert_eq!(block.is_solid(), def.is_solid);
            assert_eq!(block.is_opaque(), def.is_opaque);
            assert_eq!(block.emitted_light(), def.emitted_light);
            assert_eq!(block.is_torch(), def.is_torch);
            assert_eq!(block.get_aabb_offsets(), def.aabb_offsets);
            assert_eq!(block.hardness(), def.hardness);
            assert_eq!(block.required_tier(), def.required_tier);
            assert_eq!(block.preferred_tool(), def.preferred_tool);
            assert_eq!(block.sound_category(), def.sound_category);
            assert_eq!(block.drop_table(), def.drop_table);
            assert_eq!(registry.get(block).id, block);
        }
    }

    #[test]
    fn test_six_faces_mapping() {
        let mapping = BlockTextureMapping::SixFaces([0, 1, 2, 3, 4, 5]);
        assert_eq!(mapping.get_layer(0, 1), 0);  // +X (East)
        assert_eq!(mapping.get_layer(0, -1), 1); // -X (West)
        assert_eq!(mapping.get_layer(1, 1), 2);  // +Y (Top)
        assert_eq!(mapping.get_layer(1, -1), 3); // -Y (Bottom)
        assert_eq!(mapping.get_layer(2, 1), 4);  // +Z (North)
        assert_eq!(mapping.get_layer(2, -1), 5); // -Z (South)
    }
}
