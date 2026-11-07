use bevy::prelude::*;
use bevy::utils::HashSet;
use bevy::render::view::NoFrustumCulling;
use crate::world::{WorldManager, BlockType};
use crate::item::{Inventory, ItemStack, ItemType, ItemRegistry, spawn_item_drop};
use crate::render::particles::{ParticleManager, get_block_debris_color};
use crate::player::{Player, PlayerVitals, ScreenShake};
use crate::phys::components::Velocity;
use crate::render::textures::GameTextures;

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

        let mut tree_entity_spawned = false;
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
                    // 同時掛載 NoFrustumCulling，保證全自訂 Packed Shader 網格在任何視角與傾角下 100% 穩定可見！
                    parent.spawn((
                        MaterialMeshBundle {
                            mesh: mesh_handle,
                            material: material.clone(),
                            transform: Transform::from_translation(Vec3::new(-16.0, 0.0, -16.0)),
                            ..default()
                        },
                        NoFrustumCulling,
                    ));
                });
                tree_entity_spawned = true;
            }
        }

        // 4. 原木處理（世界體素層立即清除；若未生成實體化倒木，直接收穫至背包）
        for log_pos in logs_to_process {
            world.set_block_global(log_pos, BlockType::Air, commands);
            particle_mgr.spawn_debris(log_pos.as_vec3() + 0.5, log_color, 4);
            inventory.damage_selected_tool(1);
            if !tree_entity_spawned {
                inventory.add_item(ItemStack::new(ItemType::OakLog, 1, registry), registry);
            }
        }

        // 5. 樹冠崩解與掉落樹枝（世界體素層立即清除；若未生成實體化倒木，直接收穫至背包）
        for (idx, leaf_pos) in leaves_to_decay.into_iter().enumerate() {
            world.set_block_global(leaf_pos, BlockType::Air, commands);
            particle_mgr.spawn_debris(leaf_pos.as_vec3() + 0.5, leaf_color, 2);

            // 每 3 片樹葉掉落 1 根木棒 (Stick)，提供基礎木質素材
            if !tree_entity_spawned && idx % 3 == 0 {
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
/// 模擬倒木受重力力矩角加速度傾倒，每影格進行體素接觸判定 (Ray/Voxel Contact Detection)。
/// 當撞擊固體地面或障礙物時：
/// 1. 壓砸傷害：若玩家站在倒木下方 (1.8m 內)，施加 35 點鈍擊傷害、向上與後推衝量，以及強烈相機震顫 (Trauma 0.9)；
///    若在 12m 內則施加距離衰減之轟鳴微震顫；
/// 2. 物理掉落物：沿倒塌軀幹在地面噴射生成 3D 原木 (OakLog) 與樹枝 (Stick) 掉落實體 (ItemDrop)；
/// 3. 碎屑粒子：噴散劇烈木屑與落葉碎屑粒子，隨後銷毀倒木實體。
pub fn update_falling_trees(
    mut commands: Commands,
    time: Res<Time>,
    world: Option<Res<WorldManager>>,
    gt: Option<Res<GameTextures>>,
    mut meshes: Option<ResMut<Assets<Mesh>>>,
    registry: Option<Res<ItemRegistry>>,
    mut q_trees: Query<(Entity, &mut Transform, &mut FallingTree)>,
    mut q_player: Query<(&Transform, Option<&mut PlayerVitals>, &mut Velocity), (With<Player>, Without<FallingTree>)>,
    mut shake: Option<ResMut<ScreenShake>>,
    mut particle_mgr: ResMut<ParticleManager>,
) {
    let dt = time.delta_seconds();
    for (entity, mut transform, mut tree) in q_trees.iter_mut() {
        tree.lifetime += dt;
        // 重力力矩角加速度：tau = g * sin(theta)
        let torque = 4.5 * tree.angle.max(0.08).sin() + 0.3;
        tree.angular_velocity += torque * dt;
        tree.angle += tree.angular_velocity * dt;

        let rot = Quat::from_axis_angle(tree.fall_axis, tree.angle);
        transform.rotation = rot;

        // 探測倒木是否碰撞固體地面 (Ray / Voxel Contact Detection)
        // 倒塌至接近地面 (~84 度，即 ~1.46 rad) 或超時保護 (2.5 秒) 作為保底
        let mut hit_ground = tree.angle >= std::f32::consts::FRAC_PI_2 * 0.93 || tree.lifetime > 2.5;

        // 🔥 關鍵修復：唯有倒塌超過 45 度 (~0.80 rad) 且僅針對原木軀幹 (log_offsets) 進行實心障礙物/地面判定
        // 徹底杜絕高空樹冠在 10 度時因微小樹葉刮擦側邊山坡或相鄰樹木而提早原地爆炸
        if !hit_ground && tree.angle > 0.80 {
            if let Some(ref wm) = world {
                for &offset in &tree.log_offsets {
                    let sample_pos = tree.pivot + rot * offset;
                    let block_pos = IVec3::new(
                        sample_pos.x.floor() as i32,
                        sample_pos.y.floor() as i32,
                        sample_pos.z.floor() as i32,
                    );
                    if wm.get_block_global(block_pos).is_solid() {
                        hit_ground = true;
                        break;
                    }
                }
            }
        }

        if hit_ground {
            // 1. 玩家壓砸傷害判定 (Crush Damage & Knockback)
            for (p_trans, maybe_vitals, mut p_vel) in q_player.iter_mut() {
                let p_pos = p_trans.translation;
                let mut min_dist_to_log = f32::INFINITY;
                for &offset in &tree.log_offsets {
                    let log_pos = tree.pivot + rot * offset;
                    let d = log_pos.distance(p_pos);
                    if d < min_dist_to_log {
                        min_dist_to_log = d;
                    }
                }

                if min_dist_to_log < 1.8 {
                    // 倒木重壓致傷：扣除 35 點生命值
                    if let Some(mut vitals) = maybe_vitals {
                        vitals.health = (vitals.health - 35.0).max(0.0);
                    }
                    // 施加強烈擊退衝量 (遠離旋轉軸與朝上)
                    let push_dir = (p_pos - tree.pivot).normalize_or_zero();
                    let knockback = (push_dir + Vec3::new(0.0, 0.45, 0.0)).normalize() * 9.0;
                    p_vel.0 += knockback;

                    // 劇烈相機震顫 (Trauma 0.9)
                    if let Some(ref mut s) = shake {
                        s.trauma = (s.trauma + 0.9).min(1.0);
                    }
                } else {
                    // 若未被直接砸中，但處於近距離 (12m 內)，施加臨場轟鳴微震顫
                    let dist_to_pivot = tree.pivot.distance(p_pos);
                    if dist_to_pivot < 12.0 {
                        let falloff = 1.0 - (dist_to_pivot / 12.0);
                        if let Some(ref mut s) = shake {
                            s.trauma = (s.trauma + 0.5 * falloff).min(1.0);
                        }
                    }
                }
            }

            // 2. 生成 3D 物理掉落物 (ItemDrop)
            let can_spawn_drops = gt.as_ref().map(|g| g.ready).unwrap_or(false)
                && meshes.is_some()
                && registry.is_some();

            if can_spawn_drops {
                let meshes_res = meshes.as_mut().unwrap();
                let gt_res = gt.as_ref().unwrap();
                let reg = registry.as_ref().unwrap();

                // 原木掉落：沿倒木軀幹生成
                for (idx, &offset) in tree.log_offsets.iter().enumerate() {
                    let drop_pos = tree.pivot + rot * offset;
                    let angle_f = idx as f32 * 1.618;
                    let scatter_x = angle_f.cos() * 1.2;
                    let scatter_z = angle_f.sin() * 1.2;
                    let drop_vel = Vec3::new(scatter_x, 2.8, scatter_z);

                    spawn_item_drop(
                        &mut commands,
                        meshes_res,
                        gt_res.material.clone(),
                        ItemStack::new(ItemType::OakLog, 1, reg),
                        drop_pos,
                        drop_vel,
                    );
                }

                // 樹枝掉落：每 3 片樹葉掉落 1 根木棒 (Stick)
                for (idx, &offset) in tree.leaf_offsets.iter().enumerate() {
                    if idx % 3 == 0 {
                        let drop_pos = tree.pivot + rot * offset;
                        let angle_f = (idx + 10) as f32 * 2.399;
                        let scatter_x = angle_f.cos() * 0.9;
                        let scatter_z = angle_f.sin() * 0.9;
                        let drop_vel = Vec3::new(scatter_x, 2.2, scatter_z);

                        spawn_item_drop(
                            &mut commands,
                            meshes_res,
                            gt_res.material.clone(),
                            ItemStack::new(ItemType::Stick, 1, reg),
                            drop_pos,
                            drop_vel,
                        );
                    }
                }
            }

            // 3. 爆散碎裂木屑與落葉粒子
            let log_color = get_block_debris_color(BlockType::OakLog);
            let leaf_color = get_block_debris_color(BlockType::OakLeaves);

            for &offset in &tree.log_offsets {
                let world_pos = tree.pivot + rot * offset;
                particle_mgr.spawn_debris(world_pos, log_color, 8);
            }
            for &offset in &tree.leaf_offsets {
                let world_pos = tree.pivot + rot * offset;
                particle_mgr.spawn_debris(world_pos, leaf_color, 4);
            }

            // 4. 銷毀倒木實體
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

    #[test]
    fn test_falling_tree_crush_damage_and_ground_impact() {
        let mut world = BevyWorld::new();
        world.init_resource::<Time>();
        world.init_resource::<ParticleManager>();
        world.init_resource::<ScreenShake>();

        let mut wm = WorldManager::default();
        let chunk_pos = IVec3::new(0, 0, 0);
        wm.chunks.insert(chunk_pos, ChunkEntry {
            buffer: std::sync::Arc::new(generator::ChunkBuffer { blocks: [BlockType::Air; 32768] }),
            light_buffer: std::sync::Arc::new(ChunkLightBuffer::default()),
            fluid_buffer: None,
            entity: None,
            is_modified: false,
            is_lighting_ready: true,
        });

        // 在倒伏軌跡處放置固體地面 (IVec3::new(1, 0, 0))
        let ecs_world = BevyWorld::new();
        let mut queue = CommandQueue::default();
        let mut commands = Commands::new(&mut queue, &ecs_world);
        wm.set_block_global(IVec3::new(1, 0, 0), BlockType::Stone, &mut commands);
        world.insert_resource(wm);

        // 玩家站立於倒木正下方落地點 (1.0, 0.5, 0.0)
        let player_ent = world.spawn((
            Player::default(),
            PlayerVitals::default(),
            Velocity::default(),
            Transform::from_xyz(1.0, 0.5, 0.0),
        )).id();

        // 倒木朝向 +X 軸旋轉倒塌 (fall_axis = Z 軸)
        let tree_ent = world.spawn((
            SpatialBundle::default(),
            FallingTree {
                pivot: Vec3::new(0.5, 0.5, 0.5),
                fall_axis: Vec3::Z,
                angle: 0.3,
                angular_velocity: 1.5,
                lifetime: 0.1,
                log_offsets: vec![Vec3::new(0.0, 1.0, 0.0), Vec3::new(0.0, 2.0, 0.0)],
                leaf_offsets: vec![Vec3::new(0.0, 3.0, 0.0)],
            },
        )).id();

        let mut schedule = Schedule::default();
        schedule.add_systems(update_falling_trees);

        // 推進時間，倒木旋轉掃過並碰觸 (1, 0, 0) 之固體石頭方塊
        {
            let mut time = world.resource_mut::<Time>();
            time.advance_by(std::time::Duration::from_millis(500));
        }
        schedule.run(&mut world);

        // 倒木應提早撞地碎裂並銷毀
        assert!(world.get::<FallingTree>(tree_ent).is_none(), "倒木碰觸固體方塊應提早觸發砸地碎裂銷毀");

        // 玩家應受到 35 點鈍擊傷害 (100 - 35 = 65)
        let vitals = world.get::<PlayerVitals>(player_ent).expect("PlayerVitals should exist");
        assert_eq!(vitals.health, 65.0, "遭受倒木重壓應扣除 35 點生命值");

        // 玩家應被施加擊退衝量
        let vel = world.get::<Velocity>(player_ent).expect("Velocity should exist");
        assert!(vel.0.length() > 1.0, "遭受倒木重擊應施加擊退衝量");

        // 螢幕震顫應劇烈跳升至 ~0.9
        let shake = world.resource::<ScreenShake>();
        assert!(shake.trauma >= 0.85, "被倒木直接砸中應誘發極度強烈的相機震顫");
    }

    #[test]
    fn test_falling_tree_distant_rumble() {
        let mut world = BevyWorld::new();
        world.init_resource::<Time>();
        world.init_resource::<ParticleManager>();
        world.init_resource::<ScreenShake>();

        // 玩家站立於 6 格之外 (遠離 1.8m 壓砸危險區，但處於 12m 轟鳴半徑內)
        let player_ent = world.spawn((
            Player::default(),
            PlayerVitals::default(),
            Velocity::default(),
            Transform::from_xyz(6.5, 0.5, 0.5),
        )).id();

        let tree_ent = world.spawn((
            SpatialBundle::default(),
            FallingTree {
                pivot: Vec3::new(0.5, 0.5, 0.5),
                fall_axis: Vec3::Z,
                angle: 1.45,
                angular_velocity: 2.0,
                lifetime: 1.0,
                log_offsets: vec![Vec3::new(0.0, 1.0, 0.0)],
                leaf_offsets: vec![],
            },
        )).id();

        let mut schedule = Schedule::default();
        schedule.add_systems(update_falling_trees);

        {
            let mut time = world.resource_mut::<Time>();
            time.advance_by(std::time::Duration::from_millis(200));
        }
        schedule.run(&mut world);

        assert!(world.get::<FallingTree>(tree_ent).is_none());

        // 玩家不受傷害
        let vitals = world.get::<PlayerVitals>(player_ent).unwrap();
        assert_eq!(vitals.health, 100.0, "處於安全距離之外玩家不應受到壓砸傷害");

        // 但相機應收到轟鳴震顫 (trauma > 0)
        let shake = world.resource::<ScreenShake>();
        assert!(shake.trauma > 0.1, "近距離倒木撞地應對周遭玩家注入轟鳴震顫");
    }
}
