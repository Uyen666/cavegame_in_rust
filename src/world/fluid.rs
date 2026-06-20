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

        let current_raw = world_manager.get_fluid_global(pos);
        // 1. 全域啟用位元遮罩解碼
        let is_source = (current_raw & 0x80) != 0;

        let mut target_level = 0;
        
        if is_source {
            // 標記為無限水源（玩家放置的），數值鎖死在滿格 8
            target_level = crate::config::MAX_FLUID_LEVEL;
        } else {
            let above_pos = pos + IVec3::Y;
            if above_pos.y < crate::utils::math::WORLD_MAX_Y {
                let fluid_above_raw = world_manager.get_fluid_global(above_pos);
                let fluid_above = fluid_above_raw & 0x0F;
                
                let block_below = world_manager.get_block_global(pos + IVec3::NEG_Y);
                let is_suspended = !block_below.is_solid();

                // 2. 修正樓梯流體判定：正下方懸空才能灌滿 8，若踩在固體上則走 BFS 遞減
                if fluid_above > 0 && is_suspended {
                    target_level = crate::config::MAX_FLUID_LEVEL;
                } else {
                    let mut max_n = 0;

                    // 若上方有水但踩在實心方塊(如樓梯)上，將上方水視為 8 的鄰居參與自然衰減
                    if fluid_above > 0 && !is_suspended {
                        max_n = crate::config::MAX_FLUID_LEVEL;
                    }

                    for dir in [IVec3::X, IVec3::NEG_X, IVec3::Z, IVec3::NEG_Z] {
                        let npos = pos + dir;
                        
                        // 3. 剛性方塊防線
                        let neighbor_block = world_manager.get_block_global(npos);
                        if neighbor_block.is_solid() {
                            continue; // 鄰居是實心方塊，水流絕對不准穿透或覆蓋它！
                        }

                        let f_raw = world_manager.get_fluid_global(npos);
                        let f_level = f_raw & 0x0F; // 1. 阻斷水源標籤惡性遺傳：僅擷取乾淨的水位參與比較
                        let n_is_source = (f_raw & 0x80) != 0;
                        
                        if f_level > 0 {
                            // 2. 正向重力水平擴散限制
                            // 一個流體方塊能向「水平四周」擴散的剛性物理前提，只取決於它自己的立足點！
                            let n_block_directly_below = world_manager.get_block_global(npos + IVec3::NEG_Y);
                            let allow_horizontal_spread = n_is_source || n_block_directly_below.is_solid();
                            
                            if allow_horizontal_spread {
                                if f_level > max_n { max_n = f_level; }
                            }
                        }
                    }
                    if max_n > 1 {
                        // 1. 阻斷水源標籤惡性遺傳：計算出純淨的 next_level 寫入
                        target_level = max_n - 1;
                    } else {
                        target_level = 0;
                    }
                }
            }
        }

        let target_raw = if is_source {
            target_level | 0x80
        } else {
            target_level
        };

        if current_raw != target_raw {
            world_manager.set_fluid_global(pos, target_raw);
            for dir in [IVec3::Y, IVec3::NEG_Y, IVec3::X, IVec3::NEG_X, IVec3::Z, IVec3::NEG_Z] {
                let npos = pos + dir;
                if npos.y >= 0 && npos.y < crate::utils::math::WORLD_MAX_Y {
                    world_manager.fluid_queue.push_back(npos);
                }
            }
        }
    }
}
