use bevy::prelude::IVec3;
use serde::{Serialize, Deserialize, Serializer, Deserializer};

pub const CHUNK_SIZE: usize = 32;
pub const TOTAL_V_SIZE: usize = CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE;

// 向上探測空氣層的防線緩衝高度（32 + 4）
const CACHE_EXTENSION: usize = 4;
pub const CACHE_Y_SIZE: usize = CHUNK_SIZE + CACHE_EXTENSION; // 36

pub trait NoiseModule: Send + Sync {
    fn sample_2d(&self, x: f64, z: f64) -> f32;
    fn sample_3d(&self, x: f64, y: f64, z: f64) -> f32;
}

use crate::world::voxel::BlockType;

pub struct ChunkBuffer {
    pub blocks: [BlockType; TOTAL_V_SIZE],
}

impl Default for ChunkBuffer {
    fn default() -> Self {
        Self {
            blocks: [BlockType::Air; TOTAL_V_SIZE],
        }
    }
}

impl ChunkBuffer {
    pub fn is_pure_air(&self) -> bool {
        self.blocks.iter().all(|&b| b == BlockType::Air)
    }
}

impl Serialize for ChunkBuffer {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut rle_encoded: Vec<(u8, u16)> = Vec::new();
        let mut current_id = self.blocks[0] as u8;
        let mut current_run = 1u16;

        for i in 1..TOTAL_V_SIZE {
            let id = self.blocks[i] as u8;
            if id == current_id && current_run < u16::MAX {
                current_run += 1;
            } else {
                rle_encoded.push((current_id, current_run));
                current_id = id;
                current_run = 1;
            }
        }
        rle_encoded.push((current_id, current_run));
        rle_encoded.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ChunkBuffer {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let rle_encoded = Vec::<(u8, u16)>::deserialize(deserializer)?;
        let mut chunk_buffer = ChunkBuffer::default();
        let mut idx = 0;

        for (id, run) in rle_encoded {
            let block = match id {
                1 => BlockType::Grass,
                2 => BlockType::Stone,
                3 => BlockType::Dirt,
                _ => BlockType::Air,
            };
            let run_len = run as usize;
            if idx + run_len <= TOTAL_V_SIZE {
                chunk_buffer.blocks[idx..(idx + run_len)].fill(block);
                idx += run_len;
            } else {
                return Err(serde::de::Error::custom("RLE block data out of bounds"));
            }
        }
        Ok(chunk_buffer)
    }
}

pub struct TerrainGenerator<N: NoiseModule> {
    pub noise_provider: N,
}

impl<N: NoiseModule> TerrainGenerator<N> {
    pub fn calculate_global_density(&self, gx: i32, gy: i32, gz: i32) -> (f32, f32) {
        let fx = gx as f64;
        let fy = gy as f64;
        let fz = gz as f64;

        let base_h = self.noise_provider.sample_2d(fx * 0.005, fz * 0.005) * 35.0 + 96.0;
        
        // 1. 2D 險峻度調變
        let ruggedness = self.noise_provider.sample_2d(fx * 0.002, fz * 0.002).max(0.0);
        let amplitude = 6.0 + ruggedness * 35.0;
        let terrain_noise = self.noise_provider.sample_3d(fx * 0.008, fy * 0.008, fz * 0.008) * amplitude;
        
        let mut density = base_h - (gy as f32) + terrain_noise;

        if 16 < gy && gy < 115 {
            let dy = (gy as f32 - 64.0) / 35.0;
            let mut cave_intensity = (1.0 - dy * dy).max(0.0);

            // 2. 地表洞穴破口閘門 (Fbm 閥值調整)
            let entrance_gate = self.noise_provider.sample_2d(fx * 0.015, fz * 0.015);
            if entrance_gate > 0.25 && gy > 64 {
                cave_intensity = f32::max(cave_intensity, 0.7);
            }

            let cave_noise = self.noise_provider.sample_3d(fx * 0.008, fy * 0.008, fz * 0.008);
            if cave_noise * cave_intensity > 0.2 {
                density = f32::min(density, -0.5);
            }
        }

        (density, base_h)
    }

    pub fn resolve_block_type(&self, gy: i32, density: f32, base_h: f32, local_by: usize, y_densities: &[f32]) -> BlockType {
        if density <= 0.0 {
            return BlockType::Air;
        }

        let density_above = y_densities[local_by + 1];

        // 頂層暴露在空氣中
        if density_above <= 0.0 {
            // 如果在適合長草的高度內，則為草地，否則為裸露的石頭
            if ((gy as f32) - base_h).abs() <= 15.0 {
                return BlockType::Grass;
            } else {
                return BlockType::Stone; // 高山或深淵直接裸露岩石
            }
        }

        // 泥土層（草地下方的幾格）
        for offset in 1..=3 {
            if local_by + offset < y_densities.len() {
                // 如果上方 1~3 格有空氣（代表接近地表），且高度在合理範圍，則填泥土
                if y_densities[local_by + offset] <= 0.0 && ((gy as f32) - base_h).abs() <= 15.0 {
                    return BlockType::Dirt;
                }
            }
        }

        BlockType::Stone
    }

    pub fn generate_chunk_data(&self, chunk_pos: IVec3) -> (ChunkBuffer, u16) {
        let mut density_cache = vec![0.0f32; CHUNK_SIZE * CHUNK_SIZE * CACHE_Y_SIZE];
        let mut base_h_cache = vec![0.0f32; CHUNK_SIZE * CHUNK_SIZE];

        for bz in 0..32 {
            for bx in 0..32 {
                let gx = chunk_pos.x * 32 + bx as i32;
                let gz = chunk_pos.z * 32 + bz as i32;

                let mut current_base_h = 0.0;
                for by in 0..CACHE_Y_SIZE {
                    let gy = chunk_pos.y * 32 + by as i32;
                    let (density, base_h) = self.calculate_global_density(gx, gy, gz);
                    current_base_h = base_h;
                    let cache_idx = by + bx * 36 + bz * 36 * 32;
                    density_cache[cache_idx] = density;
                }
                base_h_cache[bx + bz * 32] = current_base_h;
            }
        }

        let mut chunk_buffer = ChunkBuffer::default();
        let mut local_non_air = 0u16;

        for bz in 0..32 {
            for bx in 0..32 {
                let col_offset = bx * 36 + bz * 36 * 32;
                let y_densities = &density_cache[col_offset..(col_offset + 36)];
                let base_h = base_h_cache[bx + bz * 32];
                
                for by in 0..32 {
                    let gy = chunk_pos.y * 32 + by as i32;
                    let density = y_densities[by];
                    let block_type = self.resolve_block_type(gy, density, base_h, by, y_densities);
                    if block_type != BlockType::Air {
                        local_non_air += 1;
                    }
                    chunk_buffer.blocks[bx + by * 32 + bz * 1024] = block_type;
                }
            }
        }

        (chunk_buffer, local_non_air)
    }
}
