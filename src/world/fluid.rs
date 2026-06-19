use bevy::prelude::*;
use crate::world::WorldManager;

#[derive(Resource)]
pub struct FluidTickTimer(pub Timer);

pub fn fluid_tick_system(
    time: Res<Time>,
    mut timer: ResMut<FluidTickTimer>,
    mut world_manager: ResMut<WorldManager>,
    config: Res<crate::config::EngineConfig>,
) {
    let tick_speed = config.fluid_tick_speed;
    timer.0.set_duration(std::time::Duration::from_secs_f32(tick_speed));
    if !timer.0.tick(time.delta()).just_finished() {
        return;
    }

    let current_len = world_manager.fluid_queue.len();
    if current_len == 0 {
        return;
    }

    for _ in 0..current_len {
        let Some(pos) = world_manager.fluid_queue.pop_front() else { break; };
        
        if pos.y < 0 || pos.y >= crate::utils::math::WORLD_MAX_Y {
            continue;
        }

        let block = world_manager.get_block_global(pos);
        if block.is_solid() {
            if world_manager.get_fluid_global(pos) > 0 {
                world_manager.set_fluid_global(pos, 0);
            }
            continue;
        }

        let current_level = world_manager.get_fluid_global(pos);
        let mut target_level = 0;
        let source_level = crate::config::MAX_FLUID_LEVEL + 1;
        
        if current_level == source_level {
            target_level = source_level; // Source block, never decays
        } else {
            let above_pos = pos + IVec3::Y;
            if above_pos.y < crate::utils::math::WORLD_MAX_Y {
                let fluid_above = world_manager.get_fluid_global(above_pos);
                if fluid_above > 0 {
                    target_level = crate::config::MAX_FLUID_LEVEL;
                } else {
                    let mut max_n = 0;
                    for dir in [IVec3::X, IVec3::NEG_X, IVec3::Z, IVec3::NEG_Z] {
                        let npos = pos + dir;
                        let f = world_manager.get_fluid_global(npos);
                        let f_val = if f == source_level { crate::config::MAX_FLUID_LEVEL } else { f };
                        if f_val > max_n { max_n = f_val; }
                    }
                    if max_n > 1 {
                        target_level = max_n - 1;
                    } else {
                        target_level = 0;
                    }
                }
            }
        }

        if current_level != target_level {
            world_manager.set_fluid_global(pos, target_level);
            for dir in [IVec3::Y, IVec3::NEG_Y, IVec3::X, IVec3::NEG_X, IVec3::Z, IVec3::NEG_Z] {
                let npos = pos + dir;
                if npos.y >= 0 && npos.y < crate::utils::math::WORLD_MAX_Y {
                    world_manager.fluid_queue.push_back(npos);
                }
            }
        }
    }
}
