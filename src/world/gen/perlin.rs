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
            let max_y = 10 + (normalized_noise * 20.0) as i32;

            for local_y in 0..CHUNK_SIZE {
                let global_y = chunk_pos.y * CHUNK_SIZE + local_y;

                let block = if global_y > max_y {
                    BlockType::Air
                } else if global_y == max_y {
                    BlockType::Grass
                } else if global_y >= max_y - 3 {
                    BlockType::Dirt
                } else {
                    BlockType::Stone
                };

                if block != BlockType::Air {
                    chunk.set_block(x, local_y, z, block);
                }
            }
        }
    }
}
