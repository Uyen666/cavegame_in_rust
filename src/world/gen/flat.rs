use crate::world::chunk::Chunk;
use crate::world::voxel::BlockType;
use crate::utils::math::CHUNK_SIZE;

pub fn generate(chunk: &mut Chunk) {
    for x in 0..CHUNK_SIZE {
        for z in 0..CHUNK_SIZE {
            for y in 0..5 {
                let block = if y == 4 { BlockType::Grass } else { BlockType::Stone };
                chunk.set_block(x, y, z, block);
            }
        }
    }
}
