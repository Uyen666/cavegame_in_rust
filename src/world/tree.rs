use bevy::prelude::*;
use bevy::utils::HashSet;
use crate::world::{WorldManager, BlockType};
use crate::item::{Inventory, ItemStack, ItemType, ItemRegistry};
use crate::render::particles::{ParticleManager, get_block_debris_color};

/// 工業級樹木連鎖倒塌與重力滑落系統 (Tree Felling & Gravity System)
/// 當原木被破壞時觸發：
/// 1. 若使用斧頭 (can_harvest == true)：觸發 Timber 連鎖倒塌，整棵樹幹被砍伐收穫，樹冠崩解並掉落樹枝；
/// 2. 若空手破壞 (can_harvest == false)：無木材掉落，上方樹幹在重力作用下整齊下落 1 格，杜絕浮空樹木！
pub fn handle_tree_break(
    world: &mut WorldManager,
    commands: &mut Commands,
    particle_mgr: &mut ParticleManager,
    inventory: &mut Inventory,
    registry: &ItemRegistry,
    broken_pos: IVec3,
    can_harvest: bool,
) {
    // 1. 尋找與 broken_pos 連接向上延伸的所有原木 (OakLog)
    let mut logs_to_process = Vec::new();
    let mut visited_logs = HashSet::new();
    visited_logs.insert(broken_pos);

    let mut queue = vec![broken_pos];
    while let Some(curr) = queue.pop() {
        // 向上、水平相鄰 1 格探測連通原木 (最高探測 24 格高度)
        for dy in 0..=2 {
            for dx in -1..=1 {
                for dz in -1..=1 {
                    if dx == 0 && dy == 0 && dz == 0 {
                        continue;
                    }
                    let neighbor_pos = curr + IVec3::new(dx, dy, dz);
                    if neighbor_pos.y <= broken_pos.y || neighbor_pos.y > broken_pos.y + 24 {
                        continue;
                    }
                    if !visited_logs.contains(&neighbor_pos) {
                        let block = world.get_block_global(neighbor_pos);
                        if block == BlockType::OakLog {
                            visited_logs.insert(neighbor_pos);
                            logs_to_process.push(neighbor_pos);
                            queue.push(neighbor_pos);
                        }
                    }
                }
            }
        }
    }

    // 依高度排序：由低至高或由高至低
    if can_harvest {
        // ── 斧頭採伐：連鎖砍倒整棵樹 (Timber Cascade) ──
        let log_color = get_block_debris_color(BlockType::OakLog);
        let leaf_color = get_block_debris_color(BlockType::OakLeaves);

        // 收集受影響的樹葉
        let mut leaves_to_decay = Vec::new();
        let mut all_logs = visited_logs.clone();
        for &lpos in &logs_to_process {
            all_logs.insert(lpos);
        }

        for &lpos in &all_logs {
            for dy in -1..=2 {
                for dx in -2..=2 {
                    for dz in -2..=2 {
                        let leaf_pos = lpos + IVec3::new(dx, dy, dz);
                        if world.get_block_global(leaf_pos) == BlockType::OakLeaves {
                            if !leaves_to_decay.contains(&leaf_pos) {
                                leaves_to_decay.push(leaf_pos);
                            }
                        }
                    }
                }
            }
        }

        // 採收上方所有連通原木
        for log_pos in logs_to_process {
            world.set_block_global(log_pos, BlockType::Air, commands);
            particle_mgr.spawn_debris(log_pos.as_vec3() + 0.5, log_color, 8);
            inventory.add_item(ItemStack::new(ItemType::OakLog, 1, registry), registry);
            inventory.damage_selected_tool(1);
        }

        // 樹冠崩解與掉落樹枝
        for (idx, leaf_pos) in leaves_to_decay.into_iter().enumerate() {
            world.set_block_global(leaf_pos, BlockType::Air, commands);
            particle_mgr.spawn_debris(leaf_pos.as_vec3() + 0.5, leaf_color, 4);

            // 每 3 片樹葉或確定性機率掉落 1 根木棒 (Stick)
            if idx % 3 == 0 {
                inventory.add_item(ItemStack::new(ItemType::Stick, 1, registry), registry);
            }
        }
    } else {
        // ── 空手/非斧頭破壞：重力滑落 (Tree Gravity Fall) ──
        // 上方原木按照由低至高順序，向下掉落 1 格
        logs_to_process.sort_by_key(|p| p.y);
        
        let mut moved_logs = Vec::new();
        for log_pos in logs_to_process {
            let target_pos = log_pos - IVec3::Y;
            if world.get_block_global(target_pos) == BlockType::Air || target_pos == broken_pos {
                world.set_block_global(log_pos, BlockType::Air, commands);
                world.set_block_global(target_pos, BlockType::OakLog, commands);
                moved_logs.push(target_pos);
            }
        }

        // 在樹根產生下墜碎屑反饋
        let log_color = get_block_debris_color(BlockType::OakLog);
        particle_mgr.spawn_debris(broken_pos.as_vec3() + 0.5, log_color, 6);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::world::{World as BevyWorld, CommandQueue};
    use crate::world::voxel::BlockType;
    use crate::world::{ChunkEntry, ChunkLightBuffer, generator};

    #[test]
    fn test_tree_harvest_requirement_definitions() {
        let oak_log = BlockType::OakLog;
        assert_eq!(oak_log.required_tier(), crate::world::registry::ToolTier::Wood);
        assert_eq!(oak_log.preferred_tool(), crate::world::registry::ToolType::Axe);

        let oak_leaves = BlockType::OakLeaves;
        assert_eq!(oak_leaves.hardness(), 0.2);
    }

    #[test]
    fn test_tree_axe_felling_cascade() {
        let mut world_manager = WorldManager::default();
        let ecs_world = BevyWorld::new();
        let mut queue = CommandQueue::default();
        let mut commands = Commands::new(&mut queue, &ecs_world);
        let mut particle_mgr = ParticleManager::default();
        let mut inv = Inventory::new(36);
        let registry = ItemRegistry;

        // 預建區塊 (0, 0, 0)
        let chunk_pos = IVec3::new(0, 0, 0);
        world_manager.chunks.insert(chunk_pos, ChunkEntry {
            buffer: std::sync::Arc::new(generator::ChunkBuffer { blocks: [BlockType::Air; 32768] }),
            light_buffer: std::sync::Arc::new(ChunkLightBuffer::default()),
            fluid_buffer: None,
            entity: None,
            is_modified: false,
            is_lighting_ready: true,
        });

        // 建立一棵樹：y=1..=3 是原木，y=3 周遭有樹葉
        for y in 1..=3 {
            world_manager.set_block_global(IVec3::new(5, y, 5), BlockType::OakLog, &mut commands);
        }
        world_manager.set_block_global(IVec3::new(5, 4, 5), BlockType::OakLeaves, &mut commands);

        // 砍伐底座 (5, 1, 5) 且手持斧頭 (can_harvest = true)
        world_manager.set_block_global(IVec3::new(5, 1, 5), BlockType::Air, &mut commands);
        handle_tree_break(
            &mut world_manager,
            &mut commands,
            &mut particle_mgr,
            &mut inv,
            &registry,
            IVec3::new(5, 1, 5),
            true,
        );

        // 驗證上方所有原木與樹葉皆已被清除
        assert_eq!(world_manager.get_block_global(IVec3::new(5, 2, 5)), BlockType::Air);
        assert_eq!(world_manager.get_block_global(IVec3::new(5, 3, 5)), BlockType::Air);
        assert_eq!(world_manager.get_block_global(IVec3::new(5, 4, 5)), BlockType::Air);

        // 驗證背包已收到原木掉落
        let has_log = inv.slots.iter().flatten().any(|s| s.item_type == ItemType::OakLog);
        assert!(has_log, "斧頭砍伐整棵樹應自動收穫上方連通原木");
    }

    #[test]
    fn test_tree_barehand_gravity_fall() {
        let mut world_manager = WorldManager::default();
        let ecs_world = BevyWorld::new();
        let mut queue = CommandQueue::default();
        let mut commands = Commands::new(&mut queue, &ecs_world);
        let mut particle_mgr = ParticleManager::default();
        let mut inv = Inventory::new(36);
        let registry = ItemRegistry;

        let chunk_pos = IVec3::new(0, 0, 0);
        world_manager.chunks.insert(chunk_pos, ChunkEntry {
            buffer: std::sync::Arc::new(generator::ChunkBuffer { blocks: [BlockType::Air; 32768] }),
            light_buffer: std::sync::Arc::new(ChunkLightBuffer::default()),
            fluid_buffer: None,
            entity: None,
            is_modified: false,
            is_lighting_ready: true,
        });

        // 建立樹幹：y=1..=3
        for y in 1..=3 {
            world_manager.set_block_global(IVec3::new(5, y, 5), BlockType::OakLog, &mut commands);
        }

        // 空手打掉底層 y=1 (can_harvest = false)
        world_manager.set_block_global(IVec3::new(5, 1, 5), BlockType::Air, &mut commands);
        handle_tree_break(
            &mut world_manager,
            &mut commands,
            &mut particle_mgr,
            &mut inv,
            &registry,
            IVec3::new(5, 1, 5),
            false,
        );

        // 原本 y=2 的原木應掉落至 y=1，原本 y=3 的原木應掉落至 y=2，y=3 變為 Air
        assert_eq!(world_manager.get_block_global(IVec3::new(5, 1, 5)), BlockType::OakLog, "樹木重力：上方原木應下落至 y=1");
        assert_eq!(world_manager.get_block_global(IVec3::new(5, 2, 5)), BlockType::OakLog, "樹木重力：上方原木應下落至 y=2");
        assert_eq!(world_manager.get_block_global(IVec3::new(5, 3, 5)), BlockType::Air, "樹木重力：原頂端位置應變為 Air");

        // 背包不應有原木
        let has_log = inv.slots.iter().flatten().any(|s| s.item_type == ItemType::OakLog);
        assert!(!has_log, "空手打樹不應獲得原木掉落");
    }
}
