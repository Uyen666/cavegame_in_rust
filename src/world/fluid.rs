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

    let mut pushed_this_tick = std::collections::HashSet::new();

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
                                // 🌟 Minecraft 規格的 4/5 格動態尋路與改道系統 🌟
                                let mut min_dist = 999;
                                let npos_fluid_level = world_manager.get_fluid_global(npos) & 0x0F; // 🚀 本體水流級數

                                // 1. 實作鄰居懸崖探測函數：對 npos 的 4 個水平方向進行探測
                                for inner_dir in [IVec3::X, IVec3::NEG_X, IVec3::Z, IVec3::NEG_Z] {
                                    let target_scan_pos = npos + inner_dir;
                                    let target_fluid_level = world_manager.get_fluid_global(target_scan_pos) & 0x0F; // 🚀 目標探測點的水位

                                    // 🚀 正確的逆流屏蔽：如果探測方向的鄰居水位 >= 本體水流級數，說明是上游或平級，屏蔽它！
                                    let dist = if target_fluid_level >= npos_fluid_level {
                                        999
                                    } else {
                                        // 🚀 剛性校準：必須傳入獨立的內層方向，絕對不准用外層的變數！
                                        get_distance_to_drop(&world_manager, npos, inner_dir)
                                    };

                                    if dist < min_dist {
                                        min_dist = dist;
                                    }
                                }

                                let flow_dir = pos - npos; // 🚀 確保方向是從鄰居指向當前方塊的正向流動向量
                                
                                let dist_to_pos = if (world_manager.get_fluid_global(npos + flow_dir) & 0x0F) >= npos_fluid_level {
                                    999
                                } else {
                                    get_distance_to_drop(&world_manager, npos, flow_dir)
                                };

                                // 3. 重構水平擴散決策 (Flow Direction Selection)
                                let b_curr_pos = world_manager.get_block_global(pos);
                                
                                // 🚀 尋路完全體鋼鐵防線：
                                // 條件 1：四周全是平地（min_dist == 999），允許 Fallback 自然擴散。
                                // 條件 2：該方向就是通往最優懸崖的路徑（dist_to_pos == min_dist）。
                                // 條件 3【局部平地開路特權】：如果主流鎖定的懸崖還在遠處（min_dist != 1），但目標格子就在正隔壁，
                                //       這說明它是一個水平延伸的平地渠道或一格高隧道！系統必須網開一面，無條件允許水流橫向溢入！
                                let allow_flow_here = min_dist == 999 
                                    || dist_to_pos == min_dist 
                                    || (min_dist != 1 && b_curr_pos == crate::world::voxel::BlockType::Air);

                                if allow_flow_here {
                                    if npos_fluid_level > max_n { max_n = npos_fluid_level; }
                                }
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
                    let neighbor_block = world_manager.get_block_global(npos);
                    let neighbor_fluid = world_manager.get_fluid_global(npos) & 0x0F;
                    // 🚀 剛性過濾與高效 HashSet 去重：禁止固體進入，且同一幀內不重複推播
                    if neighbor_block == crate::world::voxel::BlockType::Air || neighbor_fluid > 0 {
                        if pushed_this_tick.insert(npos) {
                            world_manager.fluid_queue.push_back(npos);
                        }
                    }
                }
            }
        }
    }
}

pub fn get_distance_to_drop(world_manager: &WorldManager, start_pos: IVec3, dir: IVec3) -> u32 {
    // 🚀 升級 5 格工業級視距
    for step in 1..=5 {
        let check_pos = start_pos + dir * step;
        let b_curr = world_manager.get_block_global(check_pos);
        let b_below = world_manager.get_block_global(check_pos + IVec3::NEG_Y);
        let f_below = world_manager.get_fluid_global(check_pos + IVec3::NEG_Y) & 0x0F;
        
        // 🚀 防線一：如果前進方向撞到固體方塊，此路不通，直接截斷
        if b_curr.is_solid() {
            return 999;
        }
        
        // 🚀 只有真正的物理深淵（空氣且沒水），才叫懸崖！防止現有的瀑布水柱將 min_dist 惡性拉低到 1
        if b_below == crate::world::voxel::BlockType::Air && f_below == 0 {
            return step as u32;
        }
    }
    999
}

pub fn wake_up_fluids_in_radius(world: &mut WorldManager, center: IVec3) {
    let mut pushed_set = std::collections::HashSet::new();

    let mut try_push = |npos: IVec3| {
        let block = world.get_block_global(npos);
        let fluid_level = world.get_fluid_global(npos) & 0x0F;
        
        // 🚀 純淨去重喚醒防線
        if block == crate::world::voxel::BlockType::Air || fluid_level > 0 {
            if pushed_set.insert(npos) {
                world.fluid_queue.push_back(npos);
            }
        }
    };

    for dy in -1..=1 {
        for dx in -4..=4 {
            for dz in -4..=4 {
                try_push(center + IVec3::new(dx, dy, dz));
            }
        }
    }
    
    // 2. 無條件空氣喚醒防線：確保變更點本身及周圍 3x3x3 領域都被喚醒
    // 解決「建造隧道時，下方的空氣格不執行 PULL」的果凍水 Bug
    for dy in -1..=1 {
        for dx in -1..=1 {
            for dz in -1..=1 {
                try_push(center + IVec3::new(dx, dy, dz));
            }
        }
    }
}
