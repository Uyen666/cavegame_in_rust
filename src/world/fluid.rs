use bevy::prelude::*;
use crate::world::{WorldManager, BlockType};

#[derive(Resource)]
pub struct FluidTickTimer(pub Timer);

pub fn fluid_tick_system(
    time: Res<Time>,
    mut timer: ResMut<FluidTickTimer>,
    mut world_manager: ResMut<WorldManager>,
) {
    if !timer.0.tick(time.delta()).just_finished() {
        return;
    }

    let current_len = world_manager.fluid_queue.len();
    if current_len == 0 {
        return;
    }

    for _ in 0..current_len {
        let Some(pos) = world_manager.fluid_queue.pop_front() else { break; };
        let current_level = world_manager.get_fluid_global(pos);
        // 垂直截斷鐵律 (Vertical Drop Override)
        let below_pos = pos + IVec3::new(0, -1, 0);
        if below_pos.y >= 0 {
            let below_block = world_manager.get_block_global(below_pos);
            if below_block == BlockType::Air {
                let below_fluid = world_manager.get_fluid_global(below_pos);
                if below_fluid < 8 {
                    world_manager.set_fluid_global(below_pos, 8);
                    world_manager.fluid_queue.push_back(below_pos);
                }
                // 當前格水成功往下流，不再向水平 4 方向擴散！
                continue;
            }
        }

        // 當水位只有 1 時，不允許再向四周水平蔓延（但它剛才已經有機會往下掉落了！）
        if current_level <= 1 {
            continue;
        }

        // 水平 4 方向擴散
        let next_level = current_level - 1;
        let neighbors = [
            pos + IVec3::X,
            pos - IVec3::X,
            pos + IVec3::Z,
            pos - IVec3::Z,
        ];

        for &npos in &neighbors {
            if npos.y < 0 || npos.y >= crate::utils::math::WORLD_MAX_Y {
                continue;
            }

            let n_block = world_manager.get_block_global(npos);
            if n_block == BlockType::Air {
                let n_fluid = world_manager.get_fluid_global(npos);
                if n_fluid < next_level {
                    world_manager.set_fluid_global(npos, next_level);
                    world_manager.fluid_queue.push_back(npos);
                }
            }
        }
    }
}
