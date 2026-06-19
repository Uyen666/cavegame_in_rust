pub mod voxel;
pub mod chunk;
pub mod storage;
pub mod gen;
pub mod generator;
pub mod lighting;
use bevy::prelude::*;
use bevy::utils::{HashMap, HashSet};
use bevy::tasks::{AsyncComputeTaskPool, Task};
use futures_lite::future;
use bevy::render::primitives::Aabb;

pub use chunk::{Chunk, ChunkData, ChunkLightBuffer};
pub use voxel::BlockType;
use crate::utils::math::CHUNK_SIZE;
use noise::{NoiseFn, Perlin, Fbm};

struct TerrainNoise(Fbm<Perlin>);

impl generator::NoiseModule for TerrainNoise {
    fn sample_2d(&self, x: f64, z: f64) -> f32 {
        self.0.get([x, z]) as f32
    }
    fn sample_3d(&self, x: f64, y: f64, z: f64) -> f32 {
        self.0.get([x, y, z]) as f32
    }
}
pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WorldManager>()
            .add_systems(Startup, setup_world)
            .add_systems(
                Update,
                (
                    update_chunks,
                    poll_loading_chunks,
                ).run_if(in_state(crate::GameState::InGame))
            );
    }
}

#[derive(Component)]
pub struct GeneratingChunk(pub Task<(IVec3, ChunkData, ChunkLightBuffer, u16, Box<[i32; 1024]>)>);

const RENDER_DISTANCE: i32 = 2;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WorldType {
    Flat,
    #[default]
    PerlinHills,
    FloatingIslands,
}

/// 每個已加載的區塊在 WorldManager 中的條目。
/// 資料與渲染實體徹底解耦：
///   - `palette` 永遠存在，用於全域方塊查詢
///   - `entity`  僅非空氣區塊才有，`None` 代表純空氣，不佔用任何 ECS Transform 開銷
pub struct ChunkEntry {
    pub buffer: generator::ChunkBuffer,
    pub light_buffer: ChunkLightBuffer,
    pub entity:  Option<Entity>,
    pub is_modified: bool,
}

#[derive(Resource)]
pub struct WorldManager {
    pub chunks:         HashMap<IVec3, ChunkEntry>,
    pub loading_chunks: HashSet<IVec3>,
    pub world_type:     WorldType,
    pub seed:           u32,
    pub heightmap_cache: HashMap<IVec2, Box<[i32; 1024]>>,
}

impl Default for WorldManager {
    fn default() -> Self {
        Self {
            chunks:         HashMap::default(),
            loading_chunks: HashSet::default(),
            world_type:     WorldType::PerlinHills,
            seed:           12345,
            heightmap_cache: HashMap::default(),
        }
    }
}

impl WorldManager {
    pub fn global_to_chunk_pos(pos: IVec3) -> (IVec3, IVec3) {
        let chunk_x = pos.x.div_euclid(CHUNK_SIZE);
        let chunk_y = pos.y.div_euclid(CHUNK_SIZE);
        let chunk_z = pos.z.div_euclid(CHUNK_SIZE);

        let local_x = pos.x.rem_euclid(CHUNK_SIZE);
        let local_y = pos.y.rem_euclid(CHUNK_SIZE);
        let local_z = pos.z.rem_euclid(CHUNK_SIZE);

        (IVec3::new(chunk_x, chunk_y, chunk_z), IVec3::new(local_x, local_y, local_z))
    }

    /// 直接從 ChunkEntry.palette 查詢，無需 ECS Query
    pub fn get_block_global(&self, pos: IVec3) -> BlockType {
        if pos.y < 0 || pos.y >= crate::utils::math::WORLD_MAX_Y {
            return BlockType::Air;
        }
        let (chunk_pos, local) = Self::global_to_chunk_pos(pos);
        if let Some(entry) = self.chunks.get(&chunk_pos) {
            let idx = crate::utils::math::voxel_pos_to_index(local.x as usize, local.y as usize, local.z as usize);
            return entry.buffer.blocks[idx];
        }
        BlockType::Air
    }

    /// 相容舊簽名的全域方塊查詢（供 greedy.rs 等使用），實際上直接查 palette
    pub fn get_block_global_mut(&self, pos: IVec3, _q_chunks: &Query<(Entity, &mut Chunk)>) -> BlockType {
        self.get_block_global(pos)
    }

    pub fn get_light_global(&self, pos: IVec3) -> u8 {
        if pos.y < 0 || pos.y >= crate::utils::math::WORLD_MAX_Y {
            return 15; // Above world = full sky
        }
        let (chunk_pos, local) = Self::global_to_chunk_pos(pos);
        if let Some(entry) = self.chunks.get(&chunk_pos) {
            let idx = crate::utils::math::voxel_pos_to_index(local.x as usize, local.y as usize, local.z as usize);
            return entry.light_buffer.get_sky_light(idx);
        }
        
        let chunk_col = IVec2::new(chunk_pos.x, chunk_pos.z);
        if let Some(heightmap) = self.heightmap_cache.get(&chunk_col) {
            let local_x = local.x as usize;
            let local_z = local.z as usize;
            let max_surface_y = heightmap[local_x + local_z * 32];
            if pos.y > max_surface_y {
                return 15;
            } else {
                return 0;
            }
        }
        
        0
    }

    pub fn set_light_global(&mut self, pos: IVec3, light: u8, q_chunks: &mut Query<(Entity, &mut Chunk)>) {
        if pos.y < 0 || pos.y >= crate::utils::math::WORLD_MAX_Y { return; }
        let (chunk_pos, local) = Self::global_to_chunk_pos(pos);
        if let Some(entry) = self.chunks.get_mut(&chunk_pos) {
            let idx = crate::utils::math::voxel_pos_to_index(local.x as usize, local.y as usize, local.z as usize);
            entry.light_buffer.set_sky_light(idx, light);
            if let Some(entity) = entry.entity {
                if let Ok((_, mut chunk)) = q_chunks.get_mut(entity) {
                    chunk.light_buffer.set_sky_light(idx, light);
                    chunk.is_dirty = true;
                }
            }
        }

        let boundary_neighbors: &[(i32, IVec3)] = &[
            (local.x,          IVec3::new(-1,  0,  0)),
            (crate::utils::math::CHUNK_SIZE - 1 - local.x, IVec3::new( 1,  0,  0)),
            (local.y,          IVec3::new( 0, -1,  0)),
            (crate::utils::math::CHUNK_SIZE - 1 - local.y, IVec3::new( 0,  1,  0)),
            (local.z,          IVec3::new( 0,  0, -1)),
            (crate::utils::math::CHUNK_SIZE - 1 - local.z, IVec3::new( 0,  0,  1)),
        ];

        for &(dist_to_edge, offset) in boundary_neighbors {
            if dist_to_edge == 0 {
                let neighbor_chunk_pos = chunk_pos + offset;
                if let Some(neighbor_entry) = self.chunks.get(&neighbor_chunk_pos) {
                    if let Some(neighbor_entity) = neighbor_entry.entity {
                        if let Ok((_, mut neighbor_chunk)) = q_chunks.get_mut(neighbor_entity) {
                            neighbor_chunk.is_dirty = true;
                        }
                    }
                }
            }
        }
    }

    pub fn set_block_global(
        &mut self,
        pos: IVec3,
        block: BlockType,
        q_chunks: &mut Query<(Entity, &mut Chunk)>,
        commands: &mut Commands,
    ) {
        if pos.y < 0 || pos.y >= crate::utils::math::WORLD_MAX_Y { return; }
        let (chunk_pos, local) = Self::global_to_chunk_pos(pos);
        let Some(entry) = self.chunks.get_mut(&chunk_pos) else { return };

        // 1. 同步資料層 buffer
        let idx = crate::utils::math::voxel_pos_to_index(local.x as usize, local.y as usize, local.z as usize);
        entry.buffer.blocks[idx] = block;
        entry.is_modified = true;

        // 2. 若該 Chunk 已有實體，同步到 ECS Chunk（set_block 內部會設 is_dirty = true）
        if let Some(entity) = entry.entity {
            if let Ok((_, mut chunk)) = q_chunks.get_mut(entity) {
                chunk.set_block(local.x as usize, local.y as usize, local.z as usize, block);
            }
        } else if block != BlockType::Air {
            // 3. Lazy Spawn：純空氣 Chunk 第一次放入非空氣方塊時，動態建立實體
            let mut chunk = Chunk::new(chunk_pos);
            chunk.buffer = generator::ChunkBuffer { blocks: entry.buffer.blocks };
            chunk.set_block(local.x as usize, local.y as usize, local.z as usize, block);
            chunk.is_dirty = true;

            let new_entity = commands.spawn((
                chunk,
                SpatialBundle {
                    transform: Transform::from_xyz(
                        (chunk_pos.x * CHUNK_SIZE) as f32,
                        (chunk_pos.y * CHUNK_SIZE) as f32,
                        (chunk_pos.z * CHUNK_SIZE) as f32,
                    ),
                    ..default()
                },
                Aabb::from_min_max(Vec3::ZERO, Vec3::splat(CHUNK_SIZE as f32)),
            )).id();

            // 回填 entity 到 entry
            if let Some(entry2) = self.chunks.get_mut(&chunk_pos) {
                entry2.entity = Some(new_entity);
            }
        }

        // 4. 邊界鄰居 Dirty 傳播（Remesh Propagation）
        // 若修改的方塊位於區塊邊界，相鄰 Chunk 的 Face Culling 結果也會改變，
        // 必須強制令相鄰 Chunk 也重新網格化。
        let boundary_neighbors: &[(i32, IVec3)] = &[
            (local.x,          IVec3::new(-1,  0,  0)),  // x == 0  → 左鄰居
            (CHUNK_SIZE - 1 - local.x, IVec3::new( 1,  0,  0)),  // x == 31 → 右鄰居
            (local.y,          IVec3::new( 0, -1,  0)),  // y == 0  → 下鄰居
            (CHUNK_SIZE - 1 - local.y, IVec3::new( 0,  1,  0)),  // y == 31 → 上鄰居
            (local.z,          IVec3::new( 0,  0, -1)),  // z == 0  → 後鄰居
            (CHUNK_SIZE - 1 - local.z, IVec3::new( 0,  0,  1)),  // z == 31 → 前鄰居
        ];

        for &(dist_to_edge, offset) in boundary_neighbors {
            if dist_to_edge == 0 {
                let neighbor_chunk_pos = chunk_pos + offset;
                // 查詢鄰居是否有視覺實體，若有則標記 Dirty
                if let Some(neighbor_entry) = self.chunks.get(&neighbor_chunk_pos) {
                    if let Some(neighbor_entity) = neighbor_entry.entity {
                        if let Ok((_, mut neighbor_chunk)) = q_chunks.get_mut(neighbor_entity) {
                            neighbor_chunk.is_dirty = true;
                        }
                    }
                }
            }
        }

        // 5. Light update hook (Runtime block destruction)
        if block == BlockType::Air {
            let max_surface_y = match self.world_type {
                WorldType::Flat => 4,
                WorldType::PerlinHills => {
                    let fbm = Fbm::<Perlin>::new(self.seed);
                    let noise = TerrainNoise(fbm);
                    let generator = generator::TerrainGenerator { noise_provider: noise };
                    generator.get_max_surface_y(pos.x, pos.z)
                },
                WorldType::FloatingIslands => -1,
            };

            let mut start_light = 0;
            if pos.y > max_surface_y {
                start_light = 15;
            } else {
                let neighbors = [
                    pos + IVec3::X, pos - IVec3::X,
                    pos + IVec3::Y, pos - IVec3::Y,
                    pos + IVec3::Z, pos - IVec3::Z,
                ];
                let mut max_adj = 0;
                for &npos in &neighbors {
                    let l = self.get_light_global(npos);
                    if l > max_adj { max_adj = l; }
                }
                if max_adj > 0 {
                    start_light = max_adj - 1;
                }
            }

            self.set_light_global(pos, start_light, q_chunks);
            let mut queue = std::collections::VecDeque::new();
            queue.push_back(pos);
            crate::world::lighting::propagate_sky_light_global(self, q_chunks, queue);
        }
    }


    /// 已加載的區塊總數（資料層）
    pub fn chunk_data_count(&self) -> usize {
        self.chunks.len()
    }

    /// 真正有渲染實體的區塊數
    pub fn chunk_entity_count(&self) -> usize {
        self.chunks.values().filter(|e| e.entity.is_some()).count()
    }
}

fn setup_world(mut commands: Commands) {
    if let Err(e) = std::fs::create_dir_all("saves") {
        error!("無法建立存檔資料夾: {}", e);
    }

    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            illuminance: 10000.0,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(10.0, 20.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });
}

fn update_chunks(
    mut commands: Commands,
    mut world_manager: ResMut<WorldManager>,
    q_player: Query<&Transform, With<crate::player::Player>>,
    q_chunks: Query<(Entity, &Chunk)>,
) {
    let Ok(player_tf) = q_player.get_single() else { return; };

    let player_pos_global = player_tf.translation.as_ivec3();
    let (player_chunk_pos, _) = WorldManager::global_to_chunk_pos(player_pos_global);

    // ── 1. 載入需要顯示的區塊（3D 動態螺旋加載） ──────────────────────────────
    let mut potential_chunks = Vec::new();
    for dx in -RENDER_DISTANCE..=RENDER_DISTANCE {
        for cy in 0..crate::utils::math::WORLD_CHUNKS_Y {
            for dz in -RENDER_DISTANCE..=RENDER_DISTANCE {
                let target = IVec3::new(player_chunk_pos.x + dx, cy, player_chunk_pos.z + dz);
                if !world_manager.chunks.contains_key(&target) && !world_manager.loading_chunks.contains(&target) {
                    potential_chunks.push(target);
                }
            }
        }
    }
    
    // 3D 距離環形排序 (距離玩家越近的區塊優先加載)
    potential_chunks.sort_by_key(|pos| {
        let diff = *pos - player_chunk_pos;
        diff.x * diff.x + diff.y * diff.y + diff.z * diff.z
    });

    let task_pool = AsyncComputeTaskPool::get();
    let world_type = world_manager.world_type;
    let seed = world_manager.seed;

    // 每幀限流 (Throttling)：最多派發 4 個加載任務，防止塞爆背景通道掉幀
    for pos in potential_chunks.into_iter().take(4) {
        world_manager.loading_chunks.insert(pos);
        
        let task = task_pool.spawn(async move {
            let mut max_surface_y_map = [0i32; 1024];
            match world_type {
                WorldType::Flat => {
                    for i in 0..1024 { max_surface_y_map[i] = 4; }
                },
                WorldType::PerlinHills => {
                    let fbm = Fbm::<Perlin>::new(seed);
                    let noise = TerrainNoise(fbm);
                    let generator = generator::TerrainGenerator { noise_provider: noise };
                    for bz in 0..32 {
                        for bx in 0..32 {
                            let gx = pos.x * 32 + bx as i32;
                            let gz = pos.z * 32 + bz as i32;
                            max_surface_y_map[bx + bz * 32] = generator.get_max_surface_y(gx, gz);
                        }
                    }
                },
                WorldType::FloatingIslands => {
                    for i in 0..1024 { max_surface_y_map[i] = -1; }
                },
            }

            let (chunk_buffer, non_air_count) = if let Some(data) = storage::load_chunk_from_disk(pos) {
                let count = data.buffer.blocks.iter().filter(|&&b| b != BlockType::Air).count() as u16;
                (data.buffer, count)
            } else {
                match world_type {
                    WorldType::Flat => {
                        let mut chunk = Chunk::new(pos);
                        gen::flat::generate(&mut chunk);
                        (chunk.buffer, chunk.non_air_count)
                    },
                    WorldType::PerlinHills => {
                        let fbm = Fbm::<Perlin>::new(seed);
                        let noise = TerrainNoise(fbm);
                        let generator = generator::TerrainGenerator { noise_provider: noise };
                        generator.generate_chunk_data(pos)
                    },
                    WorldType::FloatingIslands => (generator::ChunkBuffer::default(), 0),
                }
            };
            let mut light_buffer = ChunkLightBuffer::default();
            lighting::init_sunlight(pos, &chunk_buffer, &mut light_buffer, &max_surface_y_map);
            lighting::propagate_sky_light(&chunk_buffer, &mut light_buffer);

            // 回傳完整 ChunkData, ChunkLightBuffer 與 non_air_count
            (pos, ChunkData { buffer: chunk_buffer }, light_buffer, non_air_count, Box::new(max_surface_y_map))
        });

        commands.spawn(GeneratingChunk(task));
    }

    // ── 2. 卸載過遠的區塊（直接遍歷 HashMap，無需 Query） ─────────────────
    let unload_distance = RENDER_DISTANCE + 1;
    let mut to_remove: Vec<IVec3> = Vec::new();

    for (&chunk_pos, entry) in world_manager.chunks.iter() {
        let dx = (chunk_pos.x - player_chunk_pos.x).abs();
        let dz = (chunk_pos.z - player_chunk_pos.z).abs();

        if dx > unload_distance || dz > unload_distance || chunk_pos.y < 0 || chunk_pos.y >= crate::utils::math::WORLD_CHUNKS_Y {
            to_remove.push(chunk_pos);

            // 1. 存檔分流（僅對修改過的區塊）
            if entry.is_modified {
                if let Some(entity) = entry.entity {
                    // 有 ECS 實體：從 Chunk 組件取得最新 buffer 並存檔
                    if let Ok((_, chunk)) = q_chunks.get(entity) {
                        storage::save_chunk_to_disk(chunk_pos, ChunkData {
                            buffer: generator::ChunkBuffer { blocks: chunk.buffer.blocks },
                        });
                    }
                } else {
                    // 無 ECS 實體（純空氣資料層）：直接用 ChunkEntry.buffer 觸發存檔清理
                    storage::save_chunk_to_disk(chunk_pos, ChunkData {
                        buffer: generator::ChunkBuffer { blocks: entry.buffer.blocks },
                    });
                }
            }
            
            // 2. 無條件強制 ECS 實體物理銷毀
            if let Some(entity) = entry.entity {
                commands.entity(entity).despawn_recursive();
            }
        }

    }

    for pos in to_remove {
        world_manager.chunks.remove(&pos);
    }
}

// 主執行緒輪詢系統：接收非同步生成的 ChunkData，並真正置入世界
fn poll_loading_chunks(
    mut commands: Commands,
    mut world_manager: ResMut<WorldManager>,
    mut q_tasks: Query<(Entity, &mut GeneratingChunk)>,
    mut q_chunks: Query<(Entity, &mut Chunk)>,
) {
    for (entity, mut task) in &mut q_tasks {
        if let Some((chunk_pos, chunk_data, light_buffer, non_air_count, max_surface_y_map)) = future::block_on(future::poll_once(&mut task.0)) {
            // 從追蹤名單移除
            world_manager.loading_chunks.remove(&chunk_pos);
            world_manager.heightmap_cache.insert(IVec2::new(chunk_pos.x, chunk_pos.z), max_surface_y_map);

            let is_pure_air = non_air_count == 0;

            if is_pure_air {
                let entry = ChunkEntry {
                    buffer:      chunk_data.buffer,
                    light_buffer: light_buffer.clone(),
                    entity:      None,
                    is_modified: false,
                };
                world_manager.chunks.insert(chunk_pos, entry);
            } else {
                let mut chunk = Chunk::new(chunk_pos);
                chunk.buffer = generator::ChunkBuffer { blocks: chunk_data.buffer.blocks };
                chunk.light_buffer = light_buffer.clone();
                
                // 動態校準 non_air_count 防禦邊界
                chunk.non_air_count = non_air_count;

                chunk.is_dirty = true;
                chunk.is_modified = false;

                let chunk_entity = commands.spawn((
                    chunk,
                    SpatialBundle {
                        transform: Transform::from_xyz(
                            (chunk_pos.x * CHUNK_SIZE) as f32,
                            (chunk_pos.y * CHUNK_SIZE) as f32,
                            (chunk_pos.z * CHUNK_SIZE) as f32,
                        ),
                        ..default()
                    },
                    Aabb::from_min_max(Vec3::ZERO, Vec3::splat(CHUNK_SIZE as f32)),
                )).id();

                let entry = ChunkEntry {
                    buffer:      generator::ChunkBuffer { blocks: chunk_data.buffer.blocks }, // ✅ 保存完整資料與 3D 陣列供鄰居全域查詢
                    light_buffer: light_buffer.clone(),
                    entity:      Some(chunk_entity),
                    is_modified: false,
                };
                world_manager.chunks.insert(chunk_pos, entry);
            }

            // 新舊交界處 Remesh 連動 (只針對早已存在的實體鄰居)
            let offsets = [
                IVec3::new(-1,  0,  0), IVec3::new( 1,  0,  0),
                IVec3::new( 0, -1,  0), IVec3::new( 0,  1,  0),
                IVec3::new( 0,  0, -1), IVec3::new( 0,  0,  1),
            ];
            for offset in offsets {
                let neighbor_pos = chunk_pos + offset;
                if let Some(neighbor_entry) = world_manager.chunks.get(&neighbor_pos) {
                    if let Some(neighbor_entity) = neighbor_entry.entity {
                        if let Ok((_, mut neighbor_chunk)) = q_chunks.get_mut(neighbor_entity) {
                            neighbor_chunk.is_dirty = true;
                        }
                    }
                }
            }

            // --- 邊界光照縫合 (Chunk Boundary Light Stitching) ---
            let mut lighting_queue = std::collections::VecDeque::new();
            for offset in offsets {
                let neighbor_pos = chunk_pos + offset;
                if world_manager.chunks.contains_key(&neighbor_pos) {
                    let (dx, dy, dz) = (offset.x, offset.y, offset.z);
                    let mut start_x = 0; let mut end_x = 32;
                    let mut start_y = 0; let mut end_y = 32;
                    let mut start_z = 0; let mut end_z = 32;

                    if dx == 1 { start_x = 31; end_x = 32; }
                    if dx == -1 { start_x = 0; end_x = 1; }
                    if dy == 1 { start_y = 31; end_y = 32; }
                    if dy == -1 { start_y = 0; end_y = 1; }
                    if dz == 1 { start_z = 31; end_z = 32; }
                    if dz == -1 { start_z = 0; end_z = 1; }

                    for y in start_y..end_y {
                        for z in start_z..end_z {
                            for x in start_x..end_x {
                                let global_pos = IVec3::new(chunk_pos.x * 32 + x, chunk_pos.y * 32 + y, chunk_pos.z * 32 + z);
                                let n_global_pos = global_pos + offset;

                                let this_block = world_manager.get_block_global(global_pos);
                                let n_block   = world_manager.get_block_global(n_global_pos);

                                let light   = world_manager.get_light_global(global_pos);
                                let n_light = world_manager.get_light_global(n_global_pos);

                                // Push: new chunk has more light → spill into neighbour
                                if light > 1 && n_light < light - 1 && n_block == BlockType::Air {
                                    lighting_queue.push_back(global_pos);
                                // Pull: neighbour has more light → pull into new chunk
                                } else if n_light > 1 && light < n_light - 1 && this_block == BlockType::Air {
                                    lighting_queue.push_back(n_global_pos);
                                }
                            }
                        }
                    }
                }
            }
            if !lighting_queue.is_empty() {
                crate::world::lighting::propagate_sky_light_global(&mut world_manager, &mut q_chunks, lighting_queue);
            }

            // 任務完成，銷毀 Task 實體
            commands.entity(entity).despawn();
        }
    }
}
