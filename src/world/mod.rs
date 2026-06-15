pub mod voxel;
pub mod palette;
pub mod chunk;
pub mod storage;
pub mod gen;

use bevy::prelude::*;
use bevy::utils::HashMap;

pub use chunk::{Chunk, ChunkData};
pub use voxel::BlockType;
use crate::utils::math::CHUNK_SIZE;

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WorldManager>()
            .add_systems(Startup, setup_world)
            .add_systems(
                Update,
                update_chunks.run_if(in_state(crate::GameState::InGame))
            );
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

fn spawn_chunk(commands: &mut Commands, chunk_pos: IVec3, world_type: WorldType, seed: u32) -> Entity {
    let mut chunk = Chunk::new(chunk_pos);

    if let Some(data) = storage::load_chunk_from_disk(chunk_pos) {
        chunk.palette = data.palette;
        chunk.is_modified = false;
    } else {
        match world_type {
            WorldType::Flat => {
                gen::flat::generate(&mut chunk);
            }
            WorldType::PerlinHills => {
                gen::perlin::generate(&mut chunk, chunk_pos, seed);
            }
            WorldType::FloatingIslands => {}
        }
        chunk.is_modified = false;
    }

    // 所有 Chunk 一律帶正確的世界偏移，方便子 Mesh 使用 Transform::default()
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

    // 1. 載入需要顯示的區塊（3D 動態加載）
    for dx in -RENDER_DISTANCE..=RENDER_DISTANCE {
        for cy in 0..4 {
            for dz in -RENDER_DISTANCE..=RENDER_DISTANCE {
                let target_chunk_pos = IVec3::new(player_chunk_pos.x + dx, cy, player_chunk_pos.z + dz);

                if !world_manager.chunks.contains_key(&target_chunk_pos) {
                    let entity = spawn_chunk(&mut commands, target_chunk_pos, world_manager.world_type, world_manager.seed);
                    world_manager.chunks.insert(target_chunk_pos, entity);
                }
            }
        }
    }

    // 2. 卸載過遠的區塊
    let unload_distance = RENDER_DISTANCE + 1;
    let mut chunks_to_remove = Vec::new();

    for (&chunk_pos, &entity) in world_manager.chunks.iter() {
        let dx = (chunk_pos.x - player_chunk_pos.x).abs();
        let dz = (chunk_pos.z - player_chunk_pos.z).abs();

        // 超過水平卸載距離，或是超出垂直邊界 (0..=3) 時強制卸載
        if dx > unload_distance || dz > unload_distance || chunk_pos.y < 0 || chunk_pos.y > 3 {
            // 如果超過卸載距離，標記為需要移除
            chunks_to_remove.push(chunk_pos);
            
            if let Ok(chunk) = q_chunks.get(entity) {
                if chunk.is_modified {
                    storage::save_chunk_to_disk(chunk_pos, ChunkData {
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
