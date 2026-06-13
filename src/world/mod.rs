pub mod voxel;
pub mod palette;
pub mod chunk;

use bevy::prelude::*;
use bevy::utils::HashMap;
use bevy::tasks::IoTaskPool;
use std::fs::File;
use std::io::{Read, Write};
use noise::{NoiseFn, Perlin};

pub use chunk::{Chunk, ChunkData};
pub use voxel::BlockType;
use crate::utils::math::CHUNK_SIZE;

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WorldManager>()
            .add_systems(Startup, setup_world)
            .add_systems(Update, update_chunks);
    }
}

const RENDER_DISTANCE: i32 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WorldType {
    Flat,
    #[default]
    PerlinHills,
    FloatingIslands,
}

#[derive(Resource)]
pub struct WorldManager {
    pub chunks: HashMap<IVec3, Entity>,
    pub world_type: WorldType,
    pub seed: u32,
}

impl Default for WorldManager {
    fn default() -> Self {
        Self {
            chunks: HashMap::default(),
            world_type: WorldType::PerlinHills,
            seed: 12345,
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

    pub fn get_block_global(&self, pos: IVec3, q_chunks: &Query<(Entity, &Chunk)>) -> BlockType {
        let (chunk_pos, local) = Self::global_to_chunk_pos(pos);
        if let Some(&entity) = self.chunks.get(&chunk_pos) {
            if let Ok((_, chunk)) = q_chunks.get(entity) {
                return chunk.get_block(local.x, local.y, local.z);
            }
        }
        BlockType::Air
    }

    pub fn get_block_global_mut(&self, pos: IVec3, q_chunks: &Query<(Entity, &mut Chunk)>) -> BlockType {
        let (chunk_pos, local) = Self::global_to_chunk_pos(pos);
        if let Some(&entity) = self.chunks.get(&chunk_pos) {
            if let Ok((_, chunk)) = q_chunks.get(entity) {
                return chunk.get_block(local.x, local.y, local.z);
            }
        }
        BlockType::Air
    }

    pub fn set_block_global(&self, pos: IVec3, block: BlockType, q_chunks: &mut Query<(Entity, &mut Chunk)>) {
        let (chunk_pos, local) = Self::global_to_chunk_pos(pos);
        if let Some(&entity) = self.chunks.get(&chunk_pos) {
            if let Ok((_, mut chunk)) = q_chunks.get_mut(entity) {
                chunk.set_block(local.x, local.y, local.z, block);
            }
        }
    }
}

fn setup_world(mut commands: Commands) {
    if let Err(e) = std::fs::create_dir_all("saves") {
        error!("無法建立存檔資料夾: {}", e);
    }

    // 移除固定的 Chunk，只保留全域光源（太陽）
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

fn load_chunk_from_disk(pos: IVec3) -> Option<ChunkData> {
    let path = format!("saves/chunk_{}_{}_{}.bin", pos.x, pos.y, pos.z);
    if let Ok(mut file) = File::open(&path) {
        let mut buffer = Vec::new();
        if file.read_to_end(&mut buffer).is_ok() {
            if let Ok(data) = bincode::deserialize(&buffer) {
                return Some(data);
            }
        }
    }
    None
}

fn save_chunk_to_disk(pos: IVec3, data: ChunkData) {
    let path = format!("saves/chunk_{}_{}_{}.bin", pos.x, pos.y, pos.z);
    IoTaskPool::get().spawn(async move {
        if let Ok(encoded) = bincode::serialize(&data) {
            if let Ok(mut file) = File::create(&path) {
                let _ = file.write_all(&encoded);
            }
        }
    }).detach();
}

fn spawn_chunk(commands: &mut Commands, chunk_pos: IVec3, world_type: WorldType, seed: u32) -> Entity {
    let mut chunk = Chunk::new(chunk_pos);

    if let Some(data) = load_chunk_from_disk(chunk_pos) {
        chunk.palette = data.palette;
        chunk.is_modified = false;
    } else {
        match world_type {
            WorldType::Flat => {
                // 平坦地形：y < 4 是石頭，y == 4 是草地
                for x in 0..CHUNK_SIZE {
                    for z in 0..CHUNK_SIZE {
                        for y in 0..5 {
                            let block = if y == 4 { BlockType::Grass } else { BlockType::Stone };
                            chunk.set_block(x, y, z, block);
                        }
                    }
                }
            }
            WorldType::PerlinHills => {
                let perlin = Perlin::new(seed);
                for x in 0..CHUNK_SIZE {
                    for z in 0..CHUNK_SIZE {
                        let global_x = chunk_pos.x * CHUNK_SIZE + x;
                        let global_z = chunk_pos.z * CHUNK_SIZE + z;
                        
                        let noise_val = perlin.get([global_x as f64 * 0.015, global_z as f64 * 0.015]);
                        let normalized_noise = (noise_val + 1.0) * 0.5; // -1~1 映射到 0~1
                        let height = 10 + (normalized_noise * 20.0) as i32;

                        for y in 0..=height {
                            let block = if y == height {
                                BlockType::Grass
                            } else if y >= height - 3 {
                                BlockType::Dirt
                            } else {
                                BlockType::Stone
                            };
                            chunk.set_block(x, y, z, block);
                        }
                    }
                }
            }
            WorldType::FloatingIslands => {}
        }
        chunk.is_modified = false;
    }

    commands.spawn((
        chunk,
        SpatialBundle {
            transform: Transform::from_xyz(
                (chunk_pos.x * CHUNK_SIZE) as f32,
                (chunk_pos.y * CHUNK_SIZE) as f32,
                (chunk_pos.z * CHUNK_SIZE) as f32,
            ),
            ..default()
        },
    )).id()
}

fn update_chunks(
    mut commands: Commands,
    mut world_manager: ResMut<WorldManager>,
    q_player: Query<&Transform, With<crate::player::Player>>,
    q_chunks: Query<&Chunk>,
) {
    let Ok(player_tf) = q_player.get_single() else { return; };
    
    // 玩家所在的方塊座標與 Chunk 座標
    let player_pos_global = player_tf.translation.as_ivec3();
    let (player_chunk_pos, _) = WorldManager::global_to_chunk_pos(player_pos_global);

    // 1. 載入需要顯示的區塊（九宮格/5x5）
    for dx in -RENDER_DISTANCE..=RENDER_DISTANCE {
        for dz in -RENDER_DISTANCE..=RENDER_DISTANCE {
            // 目前只生成 Y=0 的平坦世界層
            let target_chunk_pos = IVec3::new(player_chunk_pos.x + dx, 0, player_chunk_pos.z + dz);

            if !world_manager.chunks.contains_key(&target_chunk_pos) {
                let entity = spawn_chunk(&mut commands, target_chunk_pos, world_manager.world_type, world_manager.seed);
                world_manager.chunks.insert(target_chunk_pos, entity);
            }
        }
    }

    // 2. 卸載過遠的區塊
    let unload_distance = RENDER_DISTANCE + 1;
    let mut chunks_to_remove = Vec::new();

    for (&chunk_pos, &entity) in world_manager.chunks.iter() {
        let dx = (chunk_pos.x - player_chunk_pos.x).abs();
        let dz = (chunk_pos.z - player_chunk_pos.z).abs();

        if dx > unload_distance || dz > unload_distance {
            // 如果超過卸載距離，標記為需要移除
            chunks_to_remove.push(chunk_pos);
            
            if let Ok(chunk) = q_chunks.get(entity) {
                if chunk.is_modified {
                    save_chunk_to_disk(chunk_pos, ChunkData {
                        palette: chunk.palette.clone(),
                    });
                }
            }
            
            commands.entity(entity).despawn_recursive();
        }
    }

    // 從 HashMap 中正式刪除
    for chunk_pos in chunks_to_remove {
        world_manager.chunks.remove(&chunk_pos);
    }
}
