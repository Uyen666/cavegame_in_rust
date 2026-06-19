use bevy::prelude::*;
use super::voxel::BlockType;
use super::generator::ChunkBuffer;
use crate::utils::math::in_bounds;

use serde::{Serialize, Deserialize};

pub const TOTAL_V_SIZE: usize = 32 * 32 * 32;

#[derive(Clone)]
pub struct ChunkLightBuffer {
    pub light_data: [u8; TOTAL_V_SIZE],
}

impl Default for ChunkLightBuffer {
    fn default() -> Self {
        Self { light_data: [0u8; TOTAL_V_SIZE] }
    }
}

impl ChunkLightBuffer {
    pub fn get_sky_light(&self, idx: usize) -> u8 {
        (self.light_data[idx] >> 4) & 0x0F
    }

    pub fn set_sky_light(&mut self, idx: usize, value: u8) {
        self.light_data[idx] = (self.light_data[idx] & 0x0F) | ((value & 0x0F) << 4);
    }
}

#[derive(Serialize, Deserialize)]
pub struct ChunkData {
    pub buffer: ChunkBuffer,
}

#[derive(Component)]
pub struct Chunk {
    pub position: IVec3,
    pub buffer: ChunkBuffer,
    pub light_buffer: ChunkLightBuffer,
    pub is_dirty: bool,
    pub is_modified: bool,
    pub non_air_count: u16,
}

impl Chunk {
    pub fn new(position: IVec3) -> Self {
        Self {
            position,
            buffer: ChunkBuffer::default(),
            light_buffer: ChunkLightBuffer::default(),
            is_dirty: true,
            is_modified: false,
            non_air_count: 0,
        }
    }

    pub fn get_block(&self, x: usize, y: usize, z: usize) -> BlockType {
        if in_bounds(x, y, z) {
            let idx = x + y * 32 + z * 1024;
            self.buffer.blocks[idx]
        } else {
            BlockType::Air
        }
    }

    pub fn set_block(&mut self, x: usize, y: usize, z: usize, block: BlockType) {
        if in_bounds(x, y, z) {
            let idx = x + y * 32 + z * 1024;
            let old_block = self.buffer.blocks[idx];

            if old_block == BlockType::Air && block != BlockType::Air {
                self.non_air_count += 1;
            } else if old_block != BlockType::Air && block == BlockType::Air {
                self.non_air_count -= 1;
            }

            self.buffer.blocks[idx] = block;
            self.is_dirty = true;
            self.is_modified = true;
        }
    }

    #[allow(dead_code)]
    pub fn is_pure_air(&self) -> bool {
        self.non_air_count == 0
    }
}
