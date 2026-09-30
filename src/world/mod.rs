pub mod voxel;
pub mod chunk;
pub mod storage;
pub mod gen;
pub mod generator;
pub mod lighting;
pub mod fluid;
pub mod systems;
pub mod registry;
use bevy::prelude::*;
use bevy::utils::{HashMap, HashSet};
use bevy::tasks::Task;
use bevy::render::primitives::Aabb;

pub use chunk::{Chunk, ChunkData, ChunkLightBuffer};
pub use voxel::BlockType;
pub use registry::BlockRegistry;
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

#[derive(Resource)]
pub struct DayNightCycle {
    pub time: f32, // 0.0 to 24.0
    pub time_rate: f32, // hours per real second
    pub sky_factor: f32,
}

impl Default for DayNightCycle {
    fn default() -> Self {
        Self {
            time: 12.0, // Start at noon
            time_rate: 0.1, // 0.1 in-game hour per real second
            sky_factor: 1.0,
        }
    }
}

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WorldManager>()
            .init_resource::<BlockRegistry>()
            .init_resource::<DayNightCycle>()
            .insert_resource(systems::FluidTickTimer(Timer::from_seconds(0.1, TimerMode::Repeating)))
            .add_systems(Startup, systems::setup_world)
            .add_systems(
                Update,
                (
                    systems::update_chunks,
                    systems::poll_loading_chunks,
                    systems::fluid_tick_system,
                    systems::update_day_night_cycle,
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
            
            // 🚀 正確觸發周遭 Chunk 重新烘焙
            for &(axis, loc_val) in [(0usize, local.x), (1usize, local.y), (2usize, local.z)].iter() {
                if loc_val == 0 {
                    let mut dir = IVec3::ZERO;
                    dir[axis] = -1;
                    self.dirty_chunks_for_meshing.insert(chunk_pos + dir);
                } else if loc_val == crate::utils::math::CHUNK_SIZE - 1 {
                    let mut dir = IVec3::ZERO;
                    dir[axis] = 1;
                    self.dirty_chunks_for_meshing.insert(chunk_pos + dir);
                }
            }
        }
    }

    /// 相容舊簽名的全域方塊查詢（供 greedy.rs 等使用），實際上直接查 palette
    #[allow(dead_code)]
    pub fn get_block_global_mut(&self, pos: IVec3) -> BlockType {
        self.get_block_global(pos)
    }

    pub fn get_sky_light_global(&self, pos: IVec3) -> u8 {
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

    pub fn get_block_light_global(&self, pos: IVec3) -> u8 {
        if pos.y < 0 || pos.y >= crate::utils::math::WORLD_MAX_Y { return 0; }
        let (chunk_pos, local) = Self::global_to_chunk_pos(pos);
        if let Some(entry) = self.chunks.get(&chunk_pos) {
            let idx = crate::utils::math::voxel_pos_to_index(local.x as usize, local.y as usize, local.z as usize);
            return entry.light_buffer.get_block_light(idx);
        }
        0
    }

    pub fn set_sky_light_global(&mut self, pos: IVec3, light: u8) {
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

    pub fn set_block_light_global(&mut self, pos: IVec3, light: u8) {
        if pos.y < 0 || pos.y >= crate::utils::math::WORLD_MAX_Y { return; }
        let (chunk_pos, local) = Self::global_to_chunk_pos(pos);
        if let Some(entry) = self.chunks.get_mut(&chunk_pos) {
            let idx = crate::utils::math::voxel_pos_to_index(local.x as usize, local.y as usize, local.z as usize);
            entry.light_buffer.set_block_light(idx, light);
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

        // 1. 取得舊方塊並進行無變更早期退出判定
        let idx = crate::utils::math::voxel_pos_to_index(local.x as usize, local.y as usize, local.z as usize);
        let old_block = entry.buffer.blocks[idx];
        if old_block == block {
            return;
        }

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

        // 5. 核心雙軌光照更新 (Core Dual-Light Propagation & Removal)

        // 5a. 若被摧毀/取代的舊方塊是發光體 (例如 Torch / TorchWall)，無條件移除其發散的光源
        let old_emitted = old_block.emitted_light();
        if old_emitted > 0 {
            let old_light = self.get_block_light_global(pos);
            self.set_block_light_global(pos, 0);
            let mut block_remove_queue = std::collections::VecDeque::new();
            let mut block_prop_queue = std::collections::VecDeque::new();
            block_remove_queue.push_back((pos, old_light.max(old_emitted)));
            crate::world::lighting::remove_block_light_global(self, block_remove_queue, &mut block_prop_queue);
            crate::world::lighting::propagate_block_light_global(self, block_prop_queue);
        }

        // 5b. 若新放置的方塊是不透明固體 (Opaque)，剛性阻斷天空光與方塊光
        if block.is_opaque() {
            let old_sky_light = self.get_sky_light_global(pos);
            if old_sky_light > 0 {
                self.set_sky_light_global(pos, 0);
                let mut sky_remove_queue = std::collections::VecDeque::new();
                let mut sky_prop_queue = std::collections::VecDeque::new();
                sky_remove_queue.push_back((pos, old_sky_light));
                crate::world::lighting::remove_sky_light_global(self, sky_remove_queue, &mut sky_prop_queue);
                crate::world::lighting::propagate_sky_light_global(self, sky_prop_queue);
            }

            let old_block_light = self.get_block_light_global(pos);
            if old_block_light > 0 {
                self.set_block_light_global(pos, 0);
                let mut block_remove_queue = std::collections::VecDeque::new();
                let mut block_prop_queue = std::collections::VecDeque::new();
                block_remove_queue.push_back((pos, old_block_light));
                crate::world::lighting::remove_block_light_global(self, block_remove_queue, &mut block_prop_queue);
                crate::world::lighting::propagate_block_light_global(self, block_prop_queue);
            }
        }

        // 5c. 若舊方塊為不透明固體，且新方塊為透光體 (如挖開泥土變空氣、放玻璃)，允許外界光線湧入
        if old_block.is_opaque() && !block.is_opaque() {
            // 天空光湧入
            let top_light = self.get_sky_light_global(pos + IVec3::Y);
            if top_light == 15 {
                self.set_sky_light_global(pos, 15);
                self.dirty_chunks_for_meshing.insert(chunk_pos);
                let mut sky_prop_queue = std::collections::VecDeque::new();
                sky_prop_queue.push_back(pos);
                crate::world::lighting::propagate_sky_light_global(self, sky_prop_queue);
            } else {
                let neighbors = [
                    pos + IVec3::X, pos - IVec3::X,
                    pos + IVec3::Y, pos - IVec3::Y,
                    pos + IVec3::Z, pos - IVec3::Z,
                ];
                let mut max_neighbor_light = 0;
                for &npos in &neighbors {
                    let l = self.get_sky_light_global(npos);
                    if l > max_neighbor_light {
                        max_neighbor_light = l;
                    }
                }
                if max_neighbor_light > 1 {
                    self.set_sky_light_global(pos, max_neighbor_light - 1);
                    let mut sky_prop_queue = std::collections::VecDeque::new();
                    sky_prop_queue.push_back(pos);
                    crate::world::lighting::propagate_sky_light_global(self, sky_prop_queue);
                }
            }

            // 周遭既有方塊光湧入
            let neighbors = [
                pos + IVec3::X, pos - IVec3::X,
                pos + IVec3::Y, pos - IVec3::Y,
                pos + IVec3::Z, pos - IVec3::Z,
            ];
            let mut max_neighbor_block_light = 0;
            for &npos in &neighbors {
                let l = self.get_block_light_global(npos);
                if l > max_neighbor_block_light {
                    max_neighbor_block_light = l;
                }
            }
            if max_neighbor_block_light > 1 {
                self.set_block_light_global(pos, max_neighbor_block_light - 1);
                let mut block_prop_queue = std::collections::VecDeque::new();
                block_prop_queue.push_back(pos);
                crate::world::lighting::propagate_block_light_global(self, block_prop_queue);
            }
        }

        // 5d. 若新方塊具備發光能力 (例如放置 Torch)，向外擴散方塊光
        let new_emitted = block.emitted_light();
        if new_emitted > 0 {
            let current_light = self.get_block_light_global(pos);
            if new_emitted > current_light {
                self.set_block_light_global(pos, new_emitted);
                let mut block_prop_queue = std::collections::VecDeque::new();
                block_prop_queue.push_back(pos);
                crate::world::lighting::propagate_block_light_global(self, block_prop_queue);
            }
        }

        // 🚀 資料層鐵血解鎖：所有受光照影響的區塊，無條件解鎖光照狀態以供即時烘焙
        let dirty_snapshot: Vec<IVec3> = self.dirty_chunks_for_meshing.iter().copied().collect();
        for &dirty_cp in &dirty_snapshot {
            if let Some(entry) = self.chunks.get_mut(&dirty_cp) {
                entry.is_lighting_ready = true;
            }
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

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::world::CommandQueue;

    #[test]
    fn test_torch_placement_and_destruction_lighting() {
        let mut world_manager = WorldManager::default();
        let ecs_world = bevy::prelude::World::new();
        let mut queue = CommandQueue::default();
        let mut commands = Commands::new(&mut queue, &ecs_world);

        // Pre-create chunk at (0, 0, 0)
        let chunk_pos = IVec3::new(0, 0, 0);
        world_manager.chunks.insert(chunk_pos, ChunkEntry {
            buffer: generator::ChunkBuffer { blocks: [BlockType::Air; 32768] },
            light_buffer: ChunkLightBuffer::default(),
            fluid_buffer: None,
            entity: None,
            is_modified: false,
            is_lighting_ready: true,
        });

        let torch_pos = IVec3::new(10, 10, 10);
        // Place torch
        world_manager.set_block_global(torch_pos, BlockType::Torch, &mut commands);

        // Verify torch position has light 15, adjacent has light 14
        assert_eq!(world_manager.get_block_light_global(torch_pos), 15);
        assert_eq!(world_manager.get_block_light_global(torch_pos + IVec3::X), 14);
        assert_eq!(world_manager.get_block_light_global(torch_pos + IVec3::Y), 14);
        assert_eq!(world_manager.get_block_light_global(torch_pos + IVec3::new(2, 0, 0)), 13);

        // Destroy torch (replace with Air)
        world_manager.set_block_global(torch_pos, BlockType::Air, &mut commands);

        // Verify that light is completely removed (no light retention!)
        assert_eq!(world_manager.get_block_light_global(torch_pos), 0, "Torch location must be 0 after destruction");
        assert_eq!(world_manager.get_block_light_global(torch_pos + IVec3::X), 0, "Adjacent block must be 0 after torch destruction");
        assert_eq!(world_manager.get_block_light_global(torch_pos + IVec3::Y), 0);
        assert_eq!(world_manager.get_block_light_global(torch_pos + IVec3::new(2, 0, 0)), 0);
    }

    #[test]
    fn test_sunlight_penetrates_glass_and_torches() {
        let chunk_pos = IVec3::new(0, 0, 0);
        let mut blocks = generator::ChunkBuffer { blocks: [BlockType::Air; 32768] };
        let mut light_buf = ChunkLightBuffer::default();
        let heightmap = [5i32; 1024]; // max surface height is 5 (ground is at <= 5)

        // Place glass at y=10 (col bx=0, bz=0)
        let idx_glass = 0 + 10 * 32 + 0 * 1024;
        blocks.blocks[idx_glass] = BlockType::Glass;

        // Place torch at y=8
        let idx_torch = 0 + 8 * 32 + 0 * 1024;
        blocks.blocks[idx_torch] = BlockType::Torch;

        lighting::init_sunlight(chunk_pos, &blocks, &mut light_buf, &heightmap);

        // y=10 (Glass) and y=8 (Torch) should both receive direct sunlight 15 because they are non-opaque!
        assert_eq!(light_buf.get_sky_light(idx_glass), 15, "Glass must allow direct sunlight penetration");
        assert_eq!(light_buf.get_sky_light(idx_torch), 15, "Torch must allow direct sunlight penetration");
    }
}


