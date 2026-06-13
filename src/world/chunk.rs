use bevy::prelude::*;
use super::voxel::BlockType;
use super::palette::Palette;
use crate::utils::math::{voxel_pos_to_index, in_bounds, CHUNK_SIZE};

use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
pub struct ChunkData {
    pub palette: Palette,
}

#[derive(Component)]
pub struct Chunk {
    pub position: IVec3,
    pub palette: Palette,
    pub is_dirty: bool,
    pub is_modified: bool,
}

impl Chunk {
    pub fn new(position: IVec3) -> Self {
        Self {
            position,
            palette: Palette::new(),
            is_dirty: true,
            is_modified: false, // 初始生成時不算修改過
        }
    }

    pub fn get_block(&self, x: i32, y: i32, z: i32) -> BlockType {
        if in_bounds(x, y, z) {
            self.palette.get(voxel_pos_to_index(x, y, z))
        } else {
            BlockType::Air
        }
    }

    pub fn set_block(&mut self, x: i32, y: i32, z: i32, block: BlockType) {
        if in_bounds(x, y, z) {
            self.palette.set(voxel_pos_to_index(x, y, z), block);
            self.is_dirty = true;
            self.is_modified = true;
        }
    }
}
