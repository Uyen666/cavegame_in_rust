use crate::world::WorldManager;

pub fn get_distance_to_drop(world_manager: &WorldManager, start_pos: bevy::math::IVec3, dir: bevy::math::IVec3) -> u32 {
    // 🚀 升級 5 格工業級視距
    for step in 1..=5 {
        let check_pos = start_pos + dir * step;
        let b_curr = world_manager.get_block_global(check_pos);
        let b_below = world_manager.get_block_global(check_pos + bevy::math::IVec3::NEG_Y);
        let f_below = world_manager.get_fluid_global(check_pos + bevy::math::IVec3::NEG_Y) & 0x0F;
        
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

pub fn wake_up_fluids_in_radius(world: &mut WorldManager, center: bevy::math::IVec3) {
    let mut pushed_set = std::collections::HashSet::new();

    let mut try_push = |npos: bevy::math::IVec3| {
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
                try_push(center + bevy::math::IVec3::new(dx, dy, dz));
            }
        }
    }
    
    // 2. 無條件空氣喚醒防線：確保變更點本身及周圍 3x3x3 領域都被喚醒
    // 解決「建造隧道時，下方的空氣格不執行 PULL」的果凍水 Bug
    for dy in -1..=1 {
        for dx in -1..=1 {
            for dz in -1..=1 {
                try_push(center + bevy::math::IVec3::new(dx, dy, dz));
            }
        }
    }
}
