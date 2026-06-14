use bevy::prelude::IVec3;
use noise::{NoiseFn, Perlin};
use crate::world::chunk::Chunk;
use crate::world::voxel::BlockType;
use crate::utils::math::CHUNK_SIZE;

pub fn generate(chunk: &mut Chunk, chunk_pos: IVec3, seed: u32) {
    let perlin = Perlin::new(seed);
    for x in 0..CHUNK_SIZE {
        for z in 0..CHUNK_SIZE {
            let global_x = chunk_pos.x * CHUNK_SIZE + x;
            let global_z = chunk_pos.z * CHUNK_SIZE + z;
            
            let noise_val = perlin.get([global_x as f64 * 0.015, global_z as f64 * 0.015]);
            let normalized_noise = (noise_val + 1.0) * 0.5; // -1~1 映射到 0~1
            let height = 10 + (normalized_noise * 20.0) as i32;

            for y in 0..=height {
                let block = if y == height {
                    BlockType::Grass
                } else if y >= height - 3 {
                    BlockType::Dirt
                } else {
                    BlockType::Stone
                };
                chunk.set_block(x, y, z, block);
            }
        }
    }
}
