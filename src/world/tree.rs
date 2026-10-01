use bevy::prelude::*;
use bevy::utils::HashSet;
use crate::world::{WorldManager, BlockType};
use crate::item::{Inventory, ItemStack, ItemType, ItemRegistry};
use crate::render::particles::{ParticleManager, get_block_debris_color};

/// 工業級樹木連鎖倒塌系統 (Timber Universal Tree Felling System)
/// 當原木被破壞時觸發：
/// 1. 若使用斧頭 (can_harvest == true)：觸發 Timber 連鎖倒塌，整棵樹幹全數收穫為原木，樹冠崩解並掉落樹枝；
/// 2. 若空手/非斧頭 (can_harvest == false)：整棵樹同樣連鎖倒塌（杜絕浮空樹木/重複滑落 bug），
///    保底提供 1 塊原木與樹枝以利初期生存起步，其餘樹幹碎裂為木屑粒子。
pub fn handle_tree_break(
    world: &mut WorldManager,
    commands: &mut Commands,
    particle_mgr: &mut ParticleManager,
    inventory: &mut Inventory,
    registry: &ItemRegistry,
    broken_pos: IVec3,
    can_harvest: bool,
) {
    // 1. 尋找與 broken_pos 連接向上/水平延伸的所有連通原木 (OakLog)
    let mut logs_to_process = Vec::new();
    let mut visited_logs = HashSet::new();
    visited_logs.insert(broken_pos);

    let mut queue = vec![broken_pos];
    while let Some(curr) = queue.pop() {
        // 向上、水平相鄰探測連通原木 (最高探測 32 格高度)
        for dy in 0..=2 {
            for dx in -1..=1 {
                for dz in -1..=1 {
                    if dx == 0 && dy == 0 && dz == 0 {
                        continue;
                    }
                    let neighbor_pos = curr + IVec3::new(dx, dy, dz);
                    if neighbor_pos.y < broken_pos.y || neighbor_pos.y > broken_pos.y + 32 {
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

    // 2. 收集受影響的樹葉 (以所有原木為中心廣域搜尋)
    let mut leaves_to_decay = Vec::new();
    let mut visited_leaves = HashSet::new();
    for &lpos in &visited_logs {
        for dy in -2..=3 {
            for dx in -3..=3 {
                for dz in -3..=3 {
                    let leaf_pos = lpos + IVec3::new(dx, dy, dz);
                    if !visited_leaves.contains(&leaf_pos) {
                        visited_leaves.insert(leaf_pos);
                        if world.get_block_global(leaf_pos) == BlockType::OakLeaves {
                            leaves_to_decay.push(leaf_pos);
                        }
                    }
                }
            }
        }
    }

    let log_color = get_block_debris_color(BlockType::OakLog);
    let leaf_color = get_block_debris_color(BlockType::OakLeaves);

    // 3. 原木處理（無論持斧與否，全數倒塌消除，絕不留浮空原木）
    if can_harvest {
        // ── 持斧採伐：連鎖收穫上方所有連通原木 ──
        for log_pos in logs_to_process {
            world.set_block_global(log_pos, BlockType::Air, commands);
            particle_mgr.spawn_debris(log_pos.as_vec3() + 0.5, log_color, 8);
            inventory.add_item(ItemStack::new(ItemType::OakLog, 1, registry), registry);
            inventory.damage_selected_tool(1);
        }
    } else {
        // ── 徒手/非斧頭破壞：辛苦砍斷樹幹使整棵樹倒塌 ──
        // 保證獲得 1 塊原木掉落（初期拓荒起步，杜絕無木材卡關死局）
        inventory.add_item(ItemStack::new(ItemType::OakLog, 1, registry), registry);

        // 上方其餘原木全數碎裂為木屑粒子
        for log_pos in logs_to_process {
            world.set_block_global(log_pos, BlockType::Air, commands);
            particle_mgr.spawn_debris(log_pos.as_vec3() + 0.5, log_color, 10);
        }
        // 破壞點噴發木屑反饋
        particle_mgr.spawn_debris(broken_pos.as_vec3() + 0.5, log_color, 8);
    }

    // 4. 樹冠崩解與掉落樹枝（全數自然凋零，徹底消滅浮空樹葉）
    for (idx, leaf_pos) in leaves_to_decay.into_iter().enumerate() {
        world.set_block_global(leaf_pos, BlockType::Air, commands);
        particle_mgr.spawn_debris(leaf_pos.as_vec3() + 0.5, leaf_color, 4);

        // 每 3 片樹葉掉落 1 根木棒 (Stick)，提供基礎木質素材
        if idx % 3 == 0 {
            inventory.add_item(ItemStack::new(ItemType::Stick, 1, registry), registry);
        }
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
    fn test_tree_barehand_felling_topple() {
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

        // 建立樹幹：y=1..=3，樹冠：y=4
        for y in 1..=3 {
            world_manager.set_block_global(IVec3::new(5, y, 5), BlockType::OakLog, &mut commands);
        }
        world_manager.set_block_global(IVec3::new(5, 4, 5), BlockType::OakLeaves, &mut commands);

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

        // 樹木應全數倒塌消散，絕無殘留浮空方塊
        assert_eq!(world_manager.get_block_global(IVec3::new(5, 1, 5)), BlockType::Air, "被破壞處應為 Air");
        assert_eq!(world_manager.get_block_global(IVec3::new(5, 2, 5)), BlockType::Air, "上方樹幹應全數倒塌消除");
        assert_eq!(world_manager.get_block_global(IVec3::new(5, 3, 5)), BlockType::Air, "上方樹幹應全數倒塌消除");
        assert_eq!(world_manager.get_block_global(IVec3::new(5, 4, 5)), BlockType::Air, "樹葉應全數凋零消除");

        // 背包獲得 1 塊原木（初期生存起步保障）與樹枝
        let log_count: u16 = inv.slots.iter().flatten().filter(|s| s.item_type == ItemType::OakLog).map(|s| s.count).sum();
        assert_eq!(log_count, 1, "空手打倒整棵樹應保底掉落 1 塊原木以利合成基礎工具");
        let has_stick = inv.slots.iter().flatten().any(|s| s.item_type == ItemType::Stick);
        assert!(has_stick, "樹葉凋零應掉落樹枝");
    }
}
