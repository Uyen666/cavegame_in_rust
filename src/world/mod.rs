pub mod voxel;
pub mod chunk;
pub mod storage;
pub mod gen;
pub mod generator;
pub mod lighting;
pub mod fluid;
pub mod systems;
use bevy::prelude::*;
use bevy::utils::{HashMap, HashSet};
use bevy::tasks::Task;
use bevy::render::primitives::Aabb;

pub use chunk::{Chunk, ChunkData, ChunkLightBuffer};
pub use voxel::BlockType;
use crate::utils::math::CHUNK_SIZE;
use noise::{NoiseFn, Perlin, Fbm};

pub struct TerrainNoise(pub Fbm<Perlin>);

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
            .insert_resource(systems::FluidTickTimer(Timer::from_seconds(0.1, TimerMode::Repeating)))
            .add_systems(Startup, systems::setup_world)
            .add_systems(
                Update,
                (
                    systems::update_chunks,
                    systems::poll_loading_chunks,
                    systems::fluid_tick_system,
                ).run_if(in_state(crate::GameState::InGame))
            );
    }
}

#[derive(Component)]
pub struct GeneratingChunk(pub Task<(IVec3, ChunkData, ChunkLightBuffer, u16, Box<[i32; 1024]>)>);



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
#[derive(Clone)]
pub struct ChunkEntry {
    pub buffer: generator::ChunkBuffer,
    pub light_buffer: ChunkLightBuffer,
    pub fluid_buffer: Option<Box<[u8; 32768]>>,
    pub entity:  Option<Entity>,
    pub is_modified: bool,
    pub is_lighting_ready: bool,
}

#[derive(Resource, Clone)]
pub struct WorldManager {
    pub chunks: HashMap<IVec3, ChunkEntry>,
    pub loading_chunks: HashSet<IVec3>,
    pub vacuum_chunks: HashSet<IVec3>, // 已確認為純空氣的區塊，不需重複加載
    pub world_type: WorldType,
    pub seed: u32,
    pub heightmap_cache: HashMap<IVec2, Box<[i32; 1024]>>,
    pub dirty_chunks_for_meshing: std::collections::HashSet<IVec3>,
    pub fluid_queue: std::collections::VecDeque<IVec3>,
}

impl Default for WorldManager {
    fn default() -> Self {
        Self {
            chunks: HashMap::default(),
            loading_chunks: HashSet::default(),
            vacuum_chunks: HashSet::default(),
            world_type: WorldType::PerlinHills,
            seed: 12345,
            heightmap_cache: HashMap::default(),
            dirty_chunks_for_meshing: std::collections::HashSet::new(),
            fluid_queue: std::collections::VecDeque::new(),
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

    pub fn get_chunk_ref(&self, pos: IVec3) -> Option<&ChunkEntry> {
        self.chunks.get(&pos)
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
        
        // 🚀 核心物理防線：當目標區塊在記憶體中不存在（Sparse 空氣或純石頭）時
        if chunk_pos.y >= 2 {
            // 🚀 高空（Y >= 64）：隱含背景是絕對空曠的空氣！物理完全通行！
            BlockType::Air
        } else {
            // 🚀 地底（Y < 64）：隱含背景是實心石頭！
            BlockType::Stone
        }
    }

    pub fn get_fluid_global(&self, pos: IVec3) -> u8 {
        if pos.y < 0 || pos.y >= crate::utils::math::WORLD_MAX_Y {
            return 0;
        }
        let (chunk_pos, local) = Self::global_to_chunk_pos(pos);
        if let Some(entry) = self.chunks.get(&chunk_pos) {
            if let Some(fluid_buf) = &entry.fluid_buffer {
                let idx = crate::utils::math::voxel_pos_to_index(local.x as usize, local.y as usize, local.z as usize);
                return fluid_buf[idx];
            }
        }
        
        0
    }

    pub fn set_fluid_global(&mut self, pos: IVec3, val: u8) {
        if pos.y < 0 || pos.y >= crate::utils::math::WORLD_MAX_Y { return; }
        let (chunk_pos, local) = Self::global_to_chunk_pos(pos);
        
        if !self.chunks.contains_key(&chunk_pos) {
            if val == 0 {
                return; // 空氣/無流體操作直接忽略
            }
            let mut new_entry = ChunkEntry {
                buffer: crate::world::generator::ChunkBuffer { blocks: [BlockType::Air; 32768] },
                light_buffer: ChunkLightBuffer::default(),
                fluid_buffer: None,
                entity: None,
                is_modified: true,
                is_lighting_ready: false,
            };
            if chunk_pos.y < 2 {
                // 🚀 地底深層：預設填滿石頭，天空光照保持為 0（死黑溶洞）
                new_entry.buffer.blocks = [BlockType::Stone; 32768];
            } else {
                // 🚀 高空世界：預設為空氣，天空光照必須強制填滿大自然的天空光！
                new_entry.light_buffer.light_data.fill(0xF0); 
            }
            self.chunks.insert(chunk_pos, new_entry);
        }
        
        // 🛡️ 铁律：只對已在 HashMap 中的區塊操作流體，絕不學生區塊
        if let Some(entry) = self.chunks.get_mut(&chunk_pos) {
            let fluid_buf = entry.fluid_buffer.get_or_insert(Box::new([0; 32768]));
            let idx = crate::utils::math::voxel_pos_to_index(local.x as usize, local.y as usize, local.z as usize);
            fluid_buf[idx] = val;
            entry.is_modified = true;
            self.dirty_chunks_for_meshing.insert(chunk_pos);
        }
    }

    /// 相容舊簽名的全域方塊查詢（供 greedy.rs 等使用），實際上直接查 palette
    #[allow(dead_code)]
    pub fn get_block_global_mut(&self, pos: IVec3) -> BlockType {
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

    pub fn set_light_global(&mut self, pos: IVec3, light: u8) {
        if pos.y < 0 || pos.y >= crate::utils::math::WORLD_MAX_Y { return; }
        let (chunk_pos, local) = Self::global_to_chunk_pos(pos);
        if let Some(entry) = self.chunks.get_mut(&chunk_pos) {
            let idx = crate::utils::math::voxel_pos_to_index(local.x as usize, local.y as usize, local.z as usize);
            entry.light_buffer.set_sky_light(idx, light);
            self.dirty_chunks_for_meshing.insert(chunk_pos);
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
                if self.chunks.contains_key(&neighbor_chunk_pos) {
                    self.dirty_chunks_for_meshing.insert(neighbor_chunk_pos);
                }
            }
        }
    }

    pub fn set_block_global(
        &mut self,
        pos: IVec3,
        block: BlockType,
        commands: &mut Commands,
    ) {
        if pos.y < 0 || pos.y >= crate::utils::math::WORLD_MAX_Y { return; }
        let (chunk_pos, local) = Self::global_to_chunk_pos(pos);
        
        let mut is_revived = false;
        if !self.chunks.contains_key(&chunk_pos) {
            if block == BlockType::Air {
                return; // 純粹的空氣操作或無效寫入，絕對不准在 HashMap 裡 insert 新區塊
            }
            
            // 🚀 動態復活限制：只有玩家真正主動放置非空氣方塊時，才觸發復活機制
            let mut new_entry = ChunkEntry {
                buffer: crate::world::generator::ChunkBuffer { blocks: [BlockType::Air; 32768] },
                light_buffer: ChunkLightBuffer::default(),
                fluid_buffer: None,
                entity: None,
                is_modified: true,
                is_lighting_ready: false,
            };
            if chunk_pos.y < 2 {
                // 🚀 如果是地底區塊，背景預設必須是石頭，否則會出現巨型灰色交界平面！
                new_entry.buffer.blocks = [BlockType::Stone; 32768];
            } else {
                // 🚀 高空世界：預設為空氣，天空光照必須強制填滿大自然的天空光！
                new_entry.light_buffer.light_data.fill(0xF0);
            }
            self.chunks.insert(chunk_pos, new_entry);
            is_revived = true;
        }
        
        // 🛡️ 防御線：取得區塊（經過上方復活邏輯後，必定能取得，除非意外）
        let Some(entry) = self.chunks.get_mut(&chunk_pos) else { return };

        // 1. 同步資料層 buffer
        let idx = crate::utils::math::voxel_pos_to_index(local.x as usize, local.y as usize, local.z as usize);
        entry.buffer.blocks[idx] = block;
        entry.is_modified = true;
        self.dirty_chunks_for_meshing.insert(chunk_pos);

        // 2. 若該 Chunk 已有實體，標記為髒污（已在上方 insert 到 dirty_chunks）
        if entry.entity.is_some() {
            // ECS Chunk 實體的同步將由系統統一處理
        } else if block != BlockType::Air {
            // 3. Lazy Spawn：純空氣 Chunk 第一次放入非空氣方塊時，動態建立實體
            let mut chunk = Chunk::new(chunk_pos);
            chunk.buffer = generator::ChunkBuffer { blocks: entry.buffer.blocks };
            chunk.light_buffer = entry.light_buffer.clone();
            chunk.non_air_count = chunk.buffer.blocks.iter().filter(|&&b| b != BlockType::Air).count() as u16;
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
        let boundary_neighbors: &[(i32, IVec3)] = &[
            (local.x,          IVec3::new(-1,  0,  0)),
            (CHUNK_SIZE - 1 - local.x, IVec3::new( 1,  0,  0)),
            (local.y,          IVec3::new( 0, -1,  0)),
            (CHUNK_SIZE - 1 - local.y, IVec3::new( 0,  1,  0)),
            (local.z,          IVec3::new( 0,  0, -1)),
            (CHUNK_SIZE - 1 - local.z, IVec3::new( 0,  0,  1)),
        ];

        for &(dist_to_edge, offset) in boundary_neighbors {
            if dist_to_edge == 0 || is_revived {
                let neighbor_chunk_pos = chunk_pos + offset;
                if self.chunks.contains_key(&neighbor_chunk_pos) {
                    self.dirty_chunks_for_meshing.insert(neighbor_chunk_pos);
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
                let top_light = self.get_light_global(pos + IVec3::Y);
                if top_light == 15 {
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
            }

            self.set_light_global(pos, start_light);
            let mut queue = std::collections::VecDeque::new();
            queue.push_back(pos);
            crate::world::lighting::propagate_sky_light_global(self, queue);
        } else {
            // 🚀 正統光照阻斷泛洪更新 (Light Removal BFS)
            let old_light = self.get_light_global(pos);
            if old_light > 0 {
                self.set_light_global(pos, 0);
                let mut remove_queue = std::collections::VecDeque::new();
                remove_queue.push_back((pos, old_light));
                let mut propagate_queue = std::collections::VecDeque::new();
                
                // 1. 消除被阻斷的光源
                crate::world::lighting::remove_sky_light_global(self, remove_queue, &mut propagate_queue);
                
                // 2. 從周圍未受影響的亮處重新蔓延光照
                crate::world::lighting::propagate_sky_light_global(self, propagate_queue);
            }
        }

        // 🚀 資料層鐵血解鎖：動態放置/破壞方塊後，無條件解鎖光照狀態
        if let Some(entry) = self.chunks.get_mut(&chunk_pos) {
            entry.is_lighting_ready = true;
        }
    }


    /// 已加載的區塊總數（資料層）
    pub fn chunk_data_count(&self) -> usize {
        self.chunks.len()
    }

    pub fn chunk_entity_count(&self) -> usize {
        self.chunks.values().filter(|e| e.entity.is_some()).count()
    }

    pub fn is_chunk_lighting_ready(&self, chunk_pos: IVec3) -> bool {
        self.chunks.get(&chunk_pos).map(|e| e.is_lighting_ready).unwrap_or(false)
    }
}


