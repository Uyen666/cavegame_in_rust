use bevy::prelude::*;
use crate::world::{WorldManager, BlockType};
use crate::render::particles::ParticleManager;

/// 鬆散土壤材質判斷：沙子 (Sand) 與 礫石 (Gravel)
#[inline(always)]
pub fn is_loose_soil(block: BlockType) -> bool {
    matches!(block, BlockType::Sand | BlockType::Gravel)
}

/// 鬆散土壤與沙石安息角自然滑坡系統 (Angle of Repose & Soil Gravity System)
/// 當鬆散土石下方無支撐時垂墜；當下方有固體但側向存在陡峭落差 (>= 2 格空隙) 時，
/// 沿安息角 (約 35°~45°) 自然斜向滑入低窪處，形成真實錐形地貌 (參考 Realistic Block Physics / TFC)
pub fn update_soil_physics_at(
    world: &mut WorldManager,
    commands: &mut Commands,
    particle_mgr: &mut ParticleManager,
    mut pos: IVec3,
) -> bool {
    let block = world.get_block_global(pos);
    if !is_loose_soil(block) {
        return false;
    }

    let mut moved = false;
    let debris_color = crate::render::particles::get_block_debris_color(block);

    // 1. 垂直重力連續下落 (Continuous Vertical Gravity Fall)
    while pos.y > 0 {
        let below_pos = pos + IVec3::NEG_Y;
        let block_below = world.get_block_global(below_pos);
        if block_below == BlockType::Air || block_below.is_torch() {
            world.set_block_global(pos, BlockType::Air, commands);
            world.set_block_global(below_pos, block, commands);
            pos = below_pos;
            moved = true;
        } else {
            break;
        }
    }

    if moved {
        particle_mgr.spawn_debris(pos.as_vec3() + 0.5, debris_color, 4);
    }

    // 若已下落至世界底層 (Y <= 0)，停止後續滑坡判定
    if pos.y <= 0 {
        return moved;
    }

    // 2. 安息角自然斜向滑坡 (Angle of Repose Lateral Slide)
    // 若下方已有支撐，檢測水平 4 個正交軸向是否面臨懸崖空隙 (側方與側下方皆為空氣，落差 >= 2 格)
    let orthogonal_dirs = [
        IVec3::X,
        IVec3::NEG_X,
        IVec3::Z,
        IVec3::NEG_Z,
    ];

    for dir in orthogonal_dirs {
        let side_pos = pos + dir;
        let side_below = side_pos + IVec3::NEG_Y;

        if side_below.y < 0 {
            continue;
        }

        let side_block = world.get_block_global(side_pos);
        let side_below_block = world.get_block_global(side_below);

        // 若側方為空氣且側下方亦為空氣，代表該處斜率超過自然安息角，觸發斜向滑移！
        if (side_block == BlockType::Air || side_block.is_torch()) 
            && (side_below_block == BlockType::Air || side_below_block.is_torch()) {
            world.set_block_global(pos, BlockType::Air, commands);
            world.set_block_global(side_below, block, commands);
            particle_mgr.spawn_debris(side_below.as_vec3() + 0.5, debris_color, 6);

            // 滑移後遞歸檢查是否在側下方繼續垂墜下落或進一步滑坡
            update_soil_physics_at(world, commands, particle_mgr, side_below);
            return true;
        }
    }

    moved
}

/// 喚醒指定半徑內的鬆散土石進行重力/滑坡檢查 (由下至上層序掃描，維持骨牌連續重力下落)
pub fn wake_up_soils_in_radius(
    world: &mut WorldManager,
    commands: &mut Commands,
    particle_mgr: &mut ParticleManager,
    center: IVec3,
) {
    for dy in 1..=3 {
        for dx in -2..=2 {
            for dz in -2..=2 {
                let check_pos = center + IVec3::new(dx, dy, dz);
                update_soil_physics_at(world, commands, particle_mgr, check_pos);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::world::{World as BevyWorld, CommandQueue};
    use crate::world::{ChunkEntry, ChunkLightBuffer, generator};

    fn create_test_world() -> (WorldManager, BevyWorld, CommandQueue) {
        let mut world_manager = WorldManager::default();
        let ecs_world = BevyWorld::new();
        let queue = CommandQueue::default();

        let chunk_pos = IVec3::new(0, 0, 0);
        world_manager.chunks.insert(chunk_pos, ChunkEntry {
            buffer: std::sync::Arc::new(generator::ChunkBuffer { blocks: [BlockType::Air; 32768] }),
            light_buffer: std::sync::Arc::new(ChunkLightBuffer::default()),
            fluid_buffer: None,
            entity: None,
            is_modified: false,
            is_lighting_ready: true,
        });

        (world_manager, ecs_world, queue)
    }

    #[test]
    fn test_soil_vertical_gravity_fall() {
        let (mut world, ecs_world, mut queue) = create_test_world();
        let mut commands = Commands::new(&mut queue, &ecs_world);
        let mut particle_mgr = ParticleManager::default();

        // 設置平坦石頭地面於 Y=4
        for x in 9..=11 {
            for z in 9..=11 {
                world.set_block_global(IVec3::new(x, 4, z), BlockType::Stone, &mut commands);
            }
        }

        let sand_pos = IVec3::new(10, 10, 10);
        world.set_block_global(sand_pos, BlockType::Sand, &mut commands);

        let fell = update_soil_physics_at(&mut world, &mut commands, &mut particle_mgr, sand_pos);
        assert!(fell, "下方為空氣時沙子應垂直下落");
        assert_eq!(world.get_block_global(sand_pos), BlockType::Air);
        // 沙子落於平坦地面 Y=4 上方，即 (10, 5, 10)
        assert_eq!(world.get_block_global(IVec3::new(10, 5, 10)), BlockType::Sand);
    }

    #[test]
    fn test_soil_angle_of_repose_lateral_slide() {
        let (mut world, ecs_world, mut queue) = create_test_world();
        let mut commands = Commands::new(&mut queue, &ecs_world);
        let mut particle_mgr = ParticleManager::default();

        // (10, 10, 10) 放置礫石，下方 (10, 9, 10) 為固體石頭（無法垂直下落）
        let gravel_pos = IVec3::new(10, 10, 10);
        world.set_block_global(IVec3::new(10, 9, 10), BlockType::Stone, &mut commands);
        world.set_block_global(gravel_pos, BlockType::Gravel, &mut commands);

        // 下層平台 (Y=8)：鋪設平坦底面，使 (11, 9, 10) 為滑入點且下方有平坦支撐 (Y=8)
        for x in 11..=13 {
            for z in 9..=11 {
                world.set_block_global(IVec3::new(x, 8, z), BlockType::Stone, &mut commands);
            }
        }
        world.set_block_global(IVec3::new(11, 10, 10), BlockType::Air, &mut commands);
        world.set_block_global(IVec3::new(11, 9, 10), BlockType::Air, &mut commands);

        let slid = update_soil_physics_at(&mut world, &mut commands, &mut particle_mgr, gravel_pos);
        assert!(slid, "面臨懸崖落差時礫石應沿安息角自然斜向滑入 (11, 9, 10)");
        assert_eq!(world.get_block_global(gravel_pos), BlockType::Air);
        assert_eq!(world.get_block_global(IVec3::new(11, 9, 10)), BlockType::Gravel);
    }

    #[test]
    fn test_soil_stable_on_flat_ground() {
        let (mut world, ecs_world, mut queue) = create_test_world();
        let mut commands = Commands::new(&mut queue, &ecs_world);
        let mut particle_mgr = ParticleManager::default();

        let gravel_pos = IVec3::new(10, 10, 10);
        world.set_block_global(IVec3::new(10, 9, 10), BlockType::Stone, &mut commands);
        world.set_block_global(gravel_pos, BlockType::Gravel, &mut commands);

        // 四周平坦 (側下方均有石頭支撐)
        world.set_block_global(IVec3::new(11, 9, 10), BlockType::Stone, &mut commands);
        world.set_block_global(IVec3::new(9, 9, 10), BlockType::Stone, &mut commands);
        world.set_block_global(IVec3::new(10, 9, 11), BlockType::Stone, &mut commands);
        world.set_block_global(IVec3::new(10, 9, 9), BlockType::Stone, &mut commands);

        let moved = update_soil_physics_at(&mut world, &mut commands, &mut particle_mgr, gravel_pos);
        assert!(!moved, "平坦有支撐地面上的鬆散土石應保持穩定，不觸發滑坡");
        assert_eq!(world.get_block_global(gravel_pos), BlockType::Gravel);
    }
}
