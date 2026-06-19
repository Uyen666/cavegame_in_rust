use std::collections::VecDeque;
use crate::world::voxel::BlockType;
use crate::world::generator::ChunkBuffer;
use crate::world::chunk::ChunkLightBuffer;

pub fn init_sunlight(
    chunk_pos: bevy::prelude::IVec3,
    blocks: &ChunkBuffer, 
    light_buffer: &mut ChunkLightBuffer, 
    max_surface_y_map: &[i32; 1024]
) {
    for bz in 0..32 {
        for bx in 0..32 {
            let col_idx = bx + bz * 32;
            let max_surface_y = max_surface_y_map[col_idx];
            
            for by in (0..32).rev() {
                let gy = chunk_pos.y * 32 + by as i32;
                let idx = bx + by * 32 + bz * 1024;
                let block = blocks.blocks[idx];

                if gy > max_surface_y {
                    if block == BlockType::Air {
                        light_buffer.set_sky_light(idx, 15);
                    } else {
                        light_buffer.set_sky_light(idx, 0);
                    }
                } else {
                    light_buffer.set_sky_light(idx, 0);
                }
            }
        }
    }
}

pub fn propagate_sky_light(blocks: &ChunkBuffer, light_buffer: &mut ChunkLightBuffer) {
    let mut queue = VecDeque::new();

    for idx in 0..32768 {
        let light = light_buffer.get_sky_light(idx);
        if light > 0 {
            let bx = idx % 32;
            let by = (idx / 32) % 32;
            let bz = idx / 1024;

            let neighbors = [
                (bx.wrapping_sub(1), by, bz),
                (bx + 1, by, bz),
                (bx, by.wrapping_sub(1), bz),
                (bx, by + 1, bz),
                (bx, by, bz.wrapping_sub(1)),
                (bx, by, bz + 1),
            ];

            let mut should_enqueue = false;
            for &(nx, ny, nz) in &neighbors {
                if nx < 32 && ny < 32 && nz < 32 {
                    let nidx = nx + ny * 32 + nz * 1024;
                    if blocks.blocks[nidx] == BlockType::Air && light_buffer.get_sky_light(nidx) < light {
                        should_enqueue = true;
                        break;
                    }
                }
            }

            if should_enqueue {
                queue.push_back(idx);
            }
        }
    }

    while let Some(idx) = queue.pop_front() {
        let light = light_buffer.get_sky_light(idx);
        if light <= 1 {
            continue;
        }

        let bx = idx % 32;
        let by = (idx / 32) % 32;
        let bz = idx / 1024;

        let neighbors = [
            (bx.wrapping_sub(1), by, bz),
            (bx + 1, by, bz),
            (bx, by.wrapping_sub(1), bz),
            (bx, by + 1, bz),
            (bx, by, bz.wrapping_sub(1)),
            (bx, by, bz + 1),
        ];

        for &(nx, ny, nz) in &neighbors {
            if nx < 32 && ny < 32 && nz < 32 {
                let nidx = nx + ny * 32 + nz * 1024;
                if blocks.blocks[nidx] == BlockType::Air {
                    if light_buffer.get_sky_light(nidx) < light - 1 {
                        light_buffer.set_sky_light(nidx, light - 1);
                        queue.push_back(nidx);
                    }
                }
            }
        }
    }
}

pub fn propagate_sky_light_global(
    world_manager: &mut crate::world::WorldManager,
    q_chunks: &mut bevy::prelude::Query<(bevy::prelude::Entity, &mut crate::world::Chunk)>,
    mut queue: VecDeque<bevy::prelude::IVec3>,
) {
    while let Some(pos) = queue.pop_front() {
        let light = world_manager.get_light_global(pos);
        if light <= 1 {
            continue;
        }

        let current_block = world_manager.get_block_global(pos);
        if current_block != BlockType::Air {
            continue;
        }

        let neighbors = [
            pos + bevy::prelude::IVec3::X,
            pos - bevy::prelude::IVec3::X,
            pos + bevy::prelude::IVec3::Y,
            pos - bevy::prelude::IVec3::Y,
            pos + bevy::prelude::IVec3::Z,
            pos - bevy::prelude::IVec3::Z,
        ];

        for &npos in &neighbors {
            if npos.y < 0 || npos.y >= crate::utils::math::WORLD_MAX_Y {
                continue;
            }

            let n_block = world_manager.get_block_global(npos);
            
            if n_block == BlockType::Air {
                let n_light = world_manager.get_light_global(npos);
                if n_light < light - 1 {
                    world_manager.set_light_global(npos, light - 1, q_chunks);
                    queue.push_back(npos);
                }
            }
        }
    }
}
