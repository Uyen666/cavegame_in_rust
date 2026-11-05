use bevy::prelude::*;
use crate::world::{WorldManager, BlockType};
use crate::render::particles::ParticleManager;
use crate::player::PlayerVitals;

/// 地質坍塌深度門檻 (覆岩應力層)：Y <= 64 的地底開採將面臨岩層頂板下沉應力
pub const CAVEIN_MAX_Y: i32 = 64;

/// 礦坑木支架 (Support Beam / Timber) 水平最大支護半徑 (方塊)
pub const BEAM_SUPPORT_RADIUS_XZ: i32 = 4;

/// 礦坑木支架垂直最大支護範圍 (方塊)
pub const BEAM_SUPPORT_RADIUS_Y: i32 = 4;

/// 天然岩柱/實心岩壁最大自穩支護跨度 (方塊)
/// 即窄於 5 格 (半徑 <= 2) 的探礦平巷能依靠岩拱效應自然保持穩定
pub const SOLID_WALL_SUPPORT_SPAN: i32 = 2;

/// 判斷該方塊是否屬於易坍塌的地質岩石/礦石
#[inline(always)]
pub fn is_cavein_prone(block: BlockType) -> bool {
    matches!(
        block,
        BlockType::Stone | BlockType::CoalOre | BlockType::IronOre | BlockType::Gravel
    )
}

/// 判斷該方塊是否可作為礦坑支護樑 (Support Beam)
#[inline(always)]
pub fn is_support_beam(block: BlockType) -> bool {
    block == BlockType::OakLog
}

/// 檢驗頂板體素是否處於安全支護狀態 (拱架支護或岩壁支撐)
pub fn is_roof_supported(world: &WorldManager, roof_pos: IVec3) -> bool {
    // 1. 若該方塊下方直接為實心固體，代表並非外露懸空頂板，結構完全穩定
    if world.get_block_global(roof_pos + IVec3::NEG_Y).is_solid() {
        return true;
    }

    // 2. 檢測附近是否存在礦坑木支架 (OakLog Timber)
    // 支護樑可在半徑 4 格內提供強固結構牽引
    for dy in -BEAM_SUPPORT_RADIUS_Y..=2 {
        for dx in -BEAM_SUPPORT_RADIUS_XZ..=BEAM_SUPPORT_RADIUS_XZ {
            for dz in -BEAM_SUPPORT_RADIUS_XZ..=BEAM_SUPPORT_RADIUS_XZ {
                let check_pos = roof_pos + IVec3::new(dx, dy, dz);
                if is_support_beam(world.get_block_global(check_pos)) {
                    return true;
                }
            }
        }
    }

    // 3. 檢測天然岩壁支護 (拱效應自穩跨度)
    // 若在 SOLID_WALL_SUPPORT_SPAN (2 格) 範圍內存在接地的實心岩壁柱，則受自穩拱保護
    for dx in -SOLID_WALL_SUPPORT_SPAN..=SOLID_WALL_SUPPORT_SPAN {
        for dz in -SOLID_WALL_SUPPORT_SPAN..=SOLID_WALL_SUPPORT_SPAN {
            if dx == 0 && dz == 0 {
                continue;
            }
            let wall_top = roof_pos + IVec3::new(dx, 0, dz);
            let wall_below = roof_pos + IVec3::new(dx, -1, dz);

            // 必須是上下連續的實心方塊 (代表非懸空飛簷，而是支撐柱或岩壁)
            if world.get_block_global(wall_top).is_solid() && world.get_block_global(wall_below).is_solid() {
                return true;
            }
        }
    }

    // 無木支架且跨度超過天然岩壁極限 -> 頂板失穩！
    false
}

/// 地質坍塌預警前兆組件 (Precursor Collapse & Stress Yield Delay)
/// 當採礦破壞頂板受力平衡時，岩層進入塑性變形與剪切屈服階段 (約 1.2 秒)，
/// 期間頂板持續飄落石塵粒子作為前兆警示，給予玩家反應逃生窗口。
#[derive(Component, Debug, Clone)]
pub struct PrecursorCollapse {
    pub collapsing_blocks: Vec<IVec3>,
    pub danger_center: IVec3,
    pub timer: f32,
    pub dust_timer: f32,
}

/// 採礦地質穩定性檢測與前兆警報生成
pub fn handle_mining_collapse(
    world: &WorldManager,
    commands: &mut Commands,
    particle_mgr: &mut ParticleManager,
    mined_pos: IVec3,
    mined_block: BlockType,
) -> Option<Entity> {
    // 僅在地底深處開採岩石或礦石時觸發地質應力失穩
    if mined_pos.y > CAVEIN_MAX_Y || (!is_cavein_prone(mined_block) && mined_block != BlockType::Dirt) {
        return None;
    }

    let mut collapsing_blocks = Vec::new();

    // 掃描被開採方塊上方及周圍的頂板方塊
    for dy in 1..=3 {
        for dx in -2..=2 {
            for dz in -2..=2 {
                let roof_pos = mined_pos + IVec3::new(dx, dy, dz);
                let block = world.get_block_global(roof_pos);
                if is_cavein_prone(block) {
                    let block_below = world.get_block_global(roof_pos + IVec3::NEG_Y);
                    if !block_below.is_solid() && !is_roof_supported(world, roof_pos) {
                        if !collapsing_blocks.contains(&roof_pos) {
                            collapsing_blocks.push(roof_pos);
                        }
                    }
                }
            }
        }
    }

    if collapsing_blocks.is_empty() {
        return None;
    }

    // 由下至上排序
    collapsing_blocks.sort_by_key(|pos| pos.y);

    // 噴射初階微裂落塵粒子
    let dust_color = Color::srgb_u8(130, 120, 110);
    for &c_pos in &collapsing_blocks {
        particle_mgr.spawn_debris(c_pos.as_vec3() + Vec3::new(0.5, 0.0, 0.5), dust_color, 4);
    }

    println!("【地質預警】岩層承受巨大剪應力，頂板岩石開始破裂剝落！1.2 秒內可能發生大面積坍塌，請立即撤離！");

    let entity = commands.spawn(PrecursorCollapse {
        collapsing_blocks,
        danger_center: mined_pos,
        timer: 1.2,
        dust_timer: 0.15,
    }).id();

    Some(entity)
}

/// 執行實際坍塌物理結算：清除頂板方塊、生成墜落碎石、觸發安息角滑坡並對危險區內生物結算重擊傷害
pub fn execute_collapse(
    world: &mut WorldManager,
    commands: &mut Commands,
    particle_mgr: &mut ParticleManager,
    collapsing_blocks: &[IVec3],
    danger_center: IVec3,
    mut player_data: Option<(Vec3, &mut PlayerVitals)>,
) {
    if collapsing_blocks.is_empty() {
        return;
    }

    let mut sorted_blocks = collapsing_blocks.to_vec();
    sorted_blocks.sort_by_key(|pos| pos.y);

    let dust_color = Color::srgb_u8(130, 120, 110);
    let rubble_color = Color::srgb_u8(110, 100, 90);

    for &c_pos in &sorted_blocks {
        // 頂板崩解為空氣
        world.set_block_global(c_pos, BlockType::Air, commands);
        particle_mgr.spawn_debris(c_pos.as_vec3() + 0.5, dust_color, 14);

        // 追蹤重力下落軌跡，將碎石堆疊於底部地面
        let mut fall_y = c_pos.y - 1;
        while fall_y > 0 && !world.get_block_global(IVec3::new(c_pos.x, fall_y, c_pos.z)).is_solid() {
            fall_y -= 1;
        }

        let landing_pos = IVec3::new(c_pos.x, fall_y + 1, c_pos.z);
        if landing_pos.y <= c_pos.y {
            // 坍塌碎石堆積為碎石堆 (Gravel)
            world.set_block_global(landing_pos, BlockType::Gravel, commands);
            crate::world::fluid::wake_up_fluids_in_radius(world, landing_pos);
            // 觸發自然安息角滑坡檢查 (Angle of Repose)
            crate::world::soil::update_soil_physics_at(world, commands, particle_mgr, landing_pos);
            particle_mgr.spawn_debris(landing_pos.as_vec3() + 0.5, rubble_color, 8);
        }
    }

    // 結算玩家受創判定：若玩家仍處於坍塌垂直投影區內，承受碎石衝擊傷害
    if let Some((player_pos, ref mut vitals)) = player_data {
        let is_in_danger_zone = (player_pos.x - danger_center.x as f32).abs() <= 2.8
            && (player_pos.z - danger_center.z as f32).abs() <= 2.8
            && player_pos.y >= (danger_center.y - 2) as f32
            && player_pos.y <= (danger_center.y + 5) as f32;

        if is_in_danger_zone {
            let damage = (sorted_blocks.len() as f32 * 8.0).clamp(15.0, 70.0);
            vitals.health = (vitals.health - damage).max(0.0);
            println!(
                "【地質坍塌】礦坑頂板徹底崩塌！受到 {:.1} 點落石重擊傷害！剩餘生命: {:.1}",
                damage, vitals.health
            );
        }
    }
}

/// 地質坍塌應力屈服計時與前兆落塵粒子系統
pub fn update_precursor_collapses(
    mut commands: Commands,
    mut world: ResMut<WorldManager>,
    mut particle_mgr: ResMut<ParticleManager>,
    time: Res<Time>,
    mut q_collapses: Query<(Entity, &mut PrecursorCollapse)>,
    mut q_player: Query<(&Transform, Option<&mut PlayerVitals>), With<crate::player::Player>>,
) {
    let delta = time.delta_seconds();
    let dust_color = Color::srgb_u8(130, 120, 110);

    let mut ready_to_collapse = Vec::new();

    for (entity, mut precursor) in q_collapses.iter_mut() {
        precursor.timer -= delta;
        precursor.dust_timer -= delta;

        // 預警期間週期性飄落微細石屑粒子
        if precursor.dust_timer <= 0.0 && precursor.timer > 0.0 {
            precursor.dust_timer = 0.15;
            for &c_pos in &precursor.collapsing_blocks {
                particle_mgr.spawn_debris(c_pos.as_vec3() + Vec3::new(0.5, 0.0, 0.5), dust_color, 2);
            }
        }

        // 應力屈服極限到達，引發徹底坍塌
        if precursor.timer <= 0.0 {
            commands.entity(entity).despawn();
            ready_to_collapse.push((precursor.collapsing_blocks.clone(), precursor.danger_center));
        }
    }

    if !ready_to_collapse.is_empty() {
        let mut player_opt = q_player.get_single_mut().ok();
        for (blocks, center) in ready_to_collapse {
            let player_data = match player_opt {
                Some((ref tf, Some(ref mut vitals))) => Some((tf.translation, &mut **vitals)),
                _ => None,
            };
            execute_collapse(
                &mut world,
                &mut commands,
                &mut particle_mgr,
                &blocks,
                center,
                player_data,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::world::{World as BevyWorld, CommandQueue};
    use crate::world::{ChunkEntry, ChunkLightBuffer, generator};

    fn create_test_world() -> (WorldManager, BevyWorld, CommandQueue) {
        let mut world = WorldManager::default();
        let ecs_world = BevyWorld::new();
        let queue = CommandQueue::default();
        let chunk_pos = IVec3::new(0, 1, 0); // Y: 32..64
        world.chunks.insert(chunk_pos, ChunkEntry {
            buffer: std::sync::Arc::new(generator::ChunkBuffer { blocks: [BlockType::Air; 32768] }),
            light_buffer: std::sync::Arc::new(ChunkLightBuffer::default()),
            fluid_buffer: None,
            entity: None,
            is_modified: false,
            is_lighting_ready: true,
        });
        (world, ecs_world, queue)
    }

    #[test]
    fn test_roof_supported_by_solid_floor_directly() {
        let (mut world, ecs_world, mut queue) = create_test_world();
        let mut commands = Commands::new(&mut queue, &ecs_world);

        let pos = IVec3::new(10, 40, 10);
        world.set_block_global(pos, BlockType::Stone, &mut commands);
        world.set_block_global(pos + IVec3::NEG_Y, BlockType::Stone, &mut commands);

        assert!(is_roof_supported(&world, pos), "下方直接為實心方塊時必然穩定");
    }

    #[test]
    fn test_roof_supported_by_support_beam() {
        let (mut world, ecs_world, mut queue) = create_test_world();
        let mut commands = Commands::new(&mut queue, &ecs_world);

        let roof_pos = IVec3::new(10, 45, 10);
        world.set_block_global(roof_pos, BlockType::Stone, &mut commands);
        world.set_block_global(roof_pos + IVec3::NEG_Y, BlockType::Air, &mut commands);

        assert!(!is_roof_supported(&world, roof_pos), "無岩壁無木支架時應判定為失穩");

        let beam_pos = IVec3::new(13, 44, 10);
        world.set_block_global(beam_pos, BlockType::OakLog, &mut commands);

        assert!(is_roof_supported(&world, roof_pos), "木支架半徑內應獲安全支護");
    }

    #[test]
    fn test_roof_supported_by_natural_wall() {
        let (mut world, ecs_world, mut queue) = create_test_world();
        let mut commands = Commands::new(&mut queue, &ecs_world);

        let roof_pos = IVec3::new(10, 45, 10);
        world.set_block_global(roof_pos, BlockType::Stone, &mut commands);
        world.set_block_global(roof_pos + IVec3::NEG_Y, BlockType::Air, &mut commands);

        let wall_x = 11;
        world.set_block_global(IVec3::new(wall_x, 45, 10), BlockType::Stone, &mut commands);
        world.set_block_global(IVec3::new(wall_x, 44, 10), BlockType::Stone, &mut commands);

        assert!(is_roof_supported(&world, roof_pos), "相鄰岩柱/岩壁在自穩跨度內應能支護頂板");
    }

    #[test]
    fn test_precursor_collapse_spawns_on_deep_mining() {
        let (mut world, ecs_world, mut queue) = create_test_world();
        let mut commands = Commands::new(&mut queue, &ecs_world);
        let mut particle_mgr = ParticleManager::default();

        // 設置 5x5 地底空腔 (Y=40 為地面, Y=43 為頂板)
        for x in 8..=12 {
            for z in 8..=12 {
                world.set_block_global(IVec3::new(x, 40, z), BlockType::Stone, &mut commands);
                world.set_block_global(IVec3::new(x, 41, z), BlockType::Air, &mut commands);
                world.set_block_global(IVec3::new(x, 42, z), BlockType::Air, &mut commands);
                world.set_block_global(IVec3::new(x, 43, z), BlockType::Stone, &mut commands);
            }
        }

        let mined_pos = IVec3::new(10, 41, 10);
        let entity_opt = handle_mining_collapse(
            &world,
            &mut commands,
            &mut particle_mgr,
            mined_pos,
            BlockType::Stone,
        );

        assert!(entity_opt.is_some(), "深層開採無支護頂板應生成 PrecursorCollapse 預警實體");
    }

    #[test]
    fn test_execute_collapse_places_gravel_and_damages_player_in_danger_zone() {
        let (mut world, ecs_world, mut queue) = create_test_world();
        let mut commands = Commands::new(&mut queue, &ecs_world);
        let mut particle_mgr = ParticleManager::default();

        for x in 8..=12 {
            for z in 8..=12 {
                world.set_block_global(IVec3::new(x, 40, z), BlockType::Stone, &mut commands);
                world.set_block_global(IVec3::new(x, 41, z), BlockType::Air, &mut commands);
                world.set_block_global(IVec3::new(x, 42, z), BlockType::Air, &mut commands);
                world.set_block_global(IVec3::new(x, 43, z), BlockType::Stone, &mut commands);
            }
        }

        let collapsing_blocks = vec![IVec3::new(10, 43, 10)];
        let danger_center = IVec3::new(10, 41, 10);
        let player_pos = Vec3::new(10.0, 41.0, 10.0);
        let mut vitals = PlayerVitals::default();

        execute_collapse(
            &mut world,
            &mut commands,
            &mut particle_mgr,
            &collapsing_blocks,
            danger_center,
            Some((player_pos, &mut vitals)),
        );

        assert_eq!(world.get_block_global(IVec3::new(10, 43, 10)), BlockType::Air, "坍塌頂板應崩解為空氣");
        assert_eq!(world.get_block_global(IVec3::new(10, 41, 10)), BlockType::Gravel, "碎石應落於底部地面");
        assert!(vitals.health < 100.0, "處於坍塌下方的玩家應受到落石重擊傷害");
    }

    #[test]
    fn test_execute_collapse_spares_escaped_player() {
        let (mut world, ecs_world, mut queue) = create_test_world();
        let mut commands = Commands::new(&mut queue, &ecs_world);
        let mut particle_mgr = ParticleManager::default();

        for x in 8..=12 {
            for z in 8..=12 {
                world.set_block_global(IVec3::new(x, 40, z), BlockType::Stone, &mut commands);
                world.set_block_global(IVec3::new(x, 41, z), BlockType::Air, &mut commands);
                world.set_block_global(IVec3::new(x, 42, z), BlockType::Air, &mut commands);
                world.set_block_global(IVec3::new(x, 43, z), BlockType::Stone, &mut commands);
            }
        }

        let collapsing_blocks = vec![IVec3::new(10, 43, 10)];
        let danger_center = IVec3::new(10, 41, 10);
        // 玩家聽到預警聲並看見落塵後，在 1.2 秒內成功逃離至 10 格外安全區域
        let player_pos = Vec3::new(20.0, 41.0, 20.0);
        let mut vitals = PlayerVitals::default();

        execute_collapse(
            &mut world,
            &mut commands,
            &mut particle_mgr,
            &collapsing_blocks,
            danger_center,
            Some((player_pos, &mut vitals)),
        );

        assert_eq!(vitals.health, 100.0, "成功逃離危險區的玩家不應受到傷害");
    }
}
