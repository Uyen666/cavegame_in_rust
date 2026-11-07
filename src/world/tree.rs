use bevy::prelude::*;
use bevy::utils::HashSet;
use crate::world::{WorldManager, BlockType};
use crate::item::{Inventory, ItemStack, ItemType, ItemRegistry};
use crate::render::particles::{ParticleManager, get_block_debris_color};

/// 實體化傾倒樹木組件 (Dynamic Falling Tree Component)
/// 模擬大樹受砍伐後帶著角動量傾倒、掃過半空並撞擊地面碎裂的真實物理過程 (參考 Dynamic Trees / HT's TreeChop)
#[derive(Component, Debug, Clone)]
pub struct FallingTree {
    pub pivot: Vec3,
    pub fall_axis: Vec3,
    pub angle: f32,
    pub angular_velocity: f32,
    pub lifetime: f32,
    pub log_offsets: Vec<Vec3>,
    pub leaf_offsets: Vec<Vec3>,
}

/// 工業級樹木連鎖倒塌與重力滑落系統 (Tree Felling & Gravity System)
/// 當原木被破壞時觸發：
/// 1. 若使用斧頭 (can_harvest == true)：觸發 Timber 連鎖倒塌，整棵樹幹全數收穫為原木，樹冠崩解並掉落樹枝；
///    若提供視效資源 (visual_assets)，將整棵樹打包為單一複合網格實體，並生成 FallingTree 進行物理傾倒動畫！
/// 2. 若空手/非斧頭 (can_harvest == false)：觸發重力滑落，無原木掉落，上方樹幹在重力作用下整齊下落 1 格，杜絕浮空樹木！
pub fn handle_tree_break(
    world: &mut WorldManager,
    commands: &mut Commands,
    particle_mgr: &mut ParticleManager,
    inventory: &mut Inventory,
    registry: &ItemRegistry,
    broken_pos: IVec3,
    can_harvest: bool,
    player_pos: Option<Vec3>,
    mut visual_assets: Option<(&mut Assets<Mesh>, Handle<crate::render::material::VoxelMaterial>)>,
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

    if can_harvest {
        // ── 持斧採伐：連鎖倒塌收穫 (Timber Cascade) 與倒木傾倒動畫 ──
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

        // 3. 準備實體化倒木 (Falling Tree Entity) 之複合網格數據
        let log_offsets: Vec<Vec3> = logs_to_process.iter().map(|&p| (p - broken_pos).as_vec3()).collect();
        let leaf_offsets: Vec<Vec3> = leaves_to_decay.iter().map(|&p| (p - broken_pos).as_vec3()).collect();

        if let Some((ref mut meshes, ref material)) = visual_assets {
            let mut mesh_blocks = Vec::with_capacity(logs_to_process.len() + leaves_to_decay.len());
            for &lpos in &logs_to_process {
                let rel = lpos - broken_pos;
                let local_p = rel + IVec3::new(16, 0, 16);
                if local_p.x >= 0 && local_p.x <= 32 && local_p.y >= 0 && local_p.y <= 32 && local_p.z >= 0 && local_p.z <= 32 {
                    mesh_blocks.push((local_p, BlockType::OakLog));
                }
            }
            for &leaf_pos in &leaves_to_decay {
                let rel = leaf_pos - broken_pos;
                let local_p = rel + IVec3::new(16, 0, 16);
                if local_p.x >= 0 && local_p.x <= 32 && local_p.y >= 0 && local_p.y <= 32 && local_p.z >= 0 && local_p.z <= 32 {
                    mesh_blocks.push((local_p, BlockType::OakLeaves));
                }
            }

            if !mesh_blocks.is_empty() {
                let compound_mesh = crate::render::greedy::build_compound_voxel_mesh(&mesh_blocks);
                let mesh_handle = meshes.add(compound_mesh);

                // 計算樹木傾倒軸向：背離玩家劈砍方向傾倒
                let push_dir = if let Some(p_pos) = player_pos {
                    let diff = broken_pos.as_vec3() - p_pos;
                    let xz = Vec3::new(diff.x, 0.0, diff.z);
                    if xz.length_squared() > 0.001 {
                        xz.normalize()
                    } else {
                        Vec3::new(0.0, 0.0, 1.0)
                    }
                } else {
                    Vec3::new(0.0, 0.0, 1.0)
                };
                let mut fall_axis = Vec3::Y.cross(push_dir);
                if fall_axis.length_squared() > 0.001 {
                    fall_axis = fall_axis.normalize();
                } else {
                    fall_axis = Vec3::X;
                }

                let pivot = broken_pos.as_vec3() + Vec3::new(0.5, 0.5, 0.5);
                commands.spawn((
                    SpatialBundle {
                        transform: Transform::from_translation(pivot),
                        ..default()
                    },
                    FallingTree {
                        pivot,
                        fall_axis,
                        angle: 0.05, // 初始微傾角
                        angular_velocity: 0.4,
                        lifetime: 0.0,
                        log_offsets: log_offsets.clone(),
                        leaf_offsets: leaf_offsets.clone(),
                    },
                )).with_children(|parent| {
                    // 注意：FallingTree 的子網格因 compound_voxel_mesh 以 (16, 0, 16) 為中心構建，
                    // 故此處需施加 (-16, 0, -16) 局部平移使旋轉軸心精確對齊父實體的 pivot 座標。
                    parent.spawn(MaterialMeshBundle {
                        mesh: mesh_handle,
                        material: material.clone(),
                        transform: Transform::from_translation(Vec3::new(-16.0, 0.0, -16.0)),
                        ..default()
                    });
                });
            }
        }

        // 4. 原木處理（世界體素層立即清除，全數收穫）
        for log_pos in logs_to_process {
            world.set_block_global(log_pos, BlockType::Air, commands);
            particle_mgr.spawn_debris(log_pos.as_vec3() + 0.5, log_color, 4);
            inventory.add_item(ItemStack::new(ItemType::OakLog, 1, registry), registry);
            inventory.damage_selected_tool(1);
        }

        // 5. 樹冠崩解與掉落樹枝（世界體素層立即清除）
        for (idx, leaf_pos) in leaves_to_decay.into_iter().enumerate() {
            world.set_block_global(leaf_pos, BlockType::Air, commands);
            particle_mgr.spawn_debris(leaf_pos.as_vec3() + 0.5, leaf_color, 2);

            // 每 3 片樹葉掉落 1 根木棒 (Stick)，提供基礎木質素材
            if idx % 3 == 0 {
                inventory.add_item(ItemStack::new(ItemType::Stick, 1, registry), registry);
            }
        }
    } else {
        // ── 徒手/非斧頭破壞：重力滑落 (Tree Gravity Fall) ──
        // 嚴格落實工具門檻：不掉落任何原木，上方原木在重力作用下整齊下落 1 格
        logs_to_process.sort_by_key(|p| p.y);
        for log_pos in logs_to_process {
            let target_pos = log_pos - IVec3::Y;
            if world.get_block_global(target_pos) == BlockType::Air || target_pos == broken_pos {
                world.set_block_global(log_pos, BlockType::Air, commands);
                world.set_block_global(target_pos, BlockType::OakLog, commands);
            }
        }
        let log_color = get_block_debris_color(BlockType::OakLog);
        particle_mgr.spawn_debris(broken_pos.as_vec3() + 0.5, log_color, 6);
    }
}

/// 倒木物理更新系統 (Falling Tree Physics System)
/// 模擬倒木受重力力矩角加速度傾倒，並在砸地瞬間噴散碎裂粒子與銷毀實體
pub fn update_falling_trees(
    mut commands: Commands,
    time: Res<Time>,
    mut q_trees: Query<(Entity, &mut Transform, &mut FallingTree)>,
    mut particle_mgr: ResMut<ParticleManager>,
) {
    let dt = time.delta_seconds();
    for (entity, mut transform, mut tree) in q_trees.iter_mut() {
        tree.lifetime += dt;
        // 重力力矩角加速度：tau = g * sin(theta)
        let torque = 4.5 * tree.angle.max(0.08).sin() + 0.3;
        tree.angular_velocity += torque * dt;
        tree.angle += tree.angular_velocity * dt;

        transform.rotation = Quat::from_axis_angle(tree.fall_axis, tree.angle);

        // 倒塌至接近地面 (~85 度，即 ~1.48 rad) 或超時保護 (2.5 秒)
        if tree.angle >= std::f32::consts::FRAC_PI_2 * 0.94 || tree.lifetime > 2.5 {
            let log_color = get_block_debris_color(BlockType::OakLog);
            let leaf_color = get_block_debris_color(BlockType::OakLeaves);

            let rot = transform.rotation;
            for &offset in &tree.log_offsets {
                let world_pos = tree.pivot + rot * offset;
                particle_mgr.spawn_debris(world_pos, log_color, 8);
            }
            for &offset in &tree.leaf_offsets {
                let world_pos = tree.pivot + rot * offset;
                particle_mgr.spawn_debris(world_pos, leaf_color, 4);
            }

            commands.entity(entity).despawn_recursive();
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
            None,
            None,
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
            None,
            None,
        );

        // 原本 y=2 的原木應掉落至 y=1，原本 y=3 的原木應掉落至 y=2，y=3 變為 Air
        assert_eq!(world_manager.get_block_global(IVec3::new(5, 1, 5)), BlockType::OakLog, "樹木重力：上方原木應下落至 y=1");
        assert_eq!(world_manager.get_block_global(IVec3::new(5, 2, 5)), BlockType::OakLog, "樹木重力：上方原木應下落至 y=2");
        assert_eq!(world_manager.get_block_global(IVec3::new(5, 3, 5)), BlockType::Air, "樹木重力：原頂端位置應變為 Air");

        // 空手破壞底座嚴格不應獲得原木掉落
        let log_count: u16 = inv.slots.iter().flatten().filter(|s| s.item_type == ItemType::OakLog).map(|s| s.count).sum();
        assert_eq!(log_count, 0, "空手打樹嚴格不應獲得任何原木掉落 (遵守 ToolTier::Wood 採伐門檻)");
    }

    #[test]
    fn test_falling_tree_topple_animation_and_impact() {
        let mut world = BevyWorld::new();
        world.init_resource::<Time>();
        world.init_resource::<ParticleManager>();

        let entity = world.spawn((
            SpatialBundle::default(),
            FallingTree {
                pivot: Vec3::ZERO,
                fall_axis: Vec3::X,
                angle: 0.1,
                angular_velocity: 1.0,
                lifetime: 0.0,
                log_offsets: vec![Vec3::new(0.0, 1.0, 0.0)],
                leaf_offsets: vec![Vec3::new(0.0, 2.0, 0.0)],
            },
        )).id();

        let mut schedule = Schedule::default();
        schedule.add_systems(update_falling_trees);

        // Tick 1 (dt = 0.1s): 驗證樹木開始旋轉加速
        {
            let mut time = world.resource_mut::<Time>();
            time.advance_by(std::time::Duration::from_millis(100));
        }
        schedule.run(&mut world);

        let tree = world.get::<FallingTree>(entity).expect("FallingTree should exist during topple");
        assert!(tree.angle > 0.1, "倒木角度應持續增加");
        assert!(tree.angular_velocity > 1.0, "角速度應受重力力矩加速");

        // Tick 2: 前進 2.0s 直至倒伏撞地 (~1.57 rad)
        {
            let mut time = world.resource_mut::<Time>();
            time.advance_by(std::time::Duration::from_millis(2000));
        }
        schedule.run(&mut world);

        // 撞擊地面後實體應自動被銷毀
        assert!(world.get::<FallingTree>(entity).is_none(), "倒木撞擊地面後應銷毀實體並觸發粒子");
    }
}
