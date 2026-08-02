use bevy::prelude::*;
use bevy::tasks::AsyncComputeTaskPool;
use futures_lite::future;
use bevy::render::primitives::Aabb;
use noise::{Perlin, Fbm};

use crate::world::{
    WorldManager, WorldType, ChunkEntry, ChunkData, ChunkLightBuffer, Chunk, BlockType, GeneratingChunk, TerrainNoise
};
use crate::world::generator::{TerrainGenerator, ChunkBuffer};
use crate::utils::math::CHUNK_SIZE;

#[derive(Resource)]
pub struct FluidTickTimer(pub Timer);

pub fn update_day_night_cycle(
    time: Res<Time>,
    mut cycle: ResMut<crate::world::DayNightCycle>,
    mut materials: ResMut<Assets<crate::render::material::VoxelMaterial>>,
) {
    cycle.time = (cycle.time + cycle.time_rate * time.delta_seconds()) % 24.0;
    
    // Mapping: 6am (0.0) -> 12pm (1.0) -> 18pm (0.0) -> night
    let phase = (cycle.time - 6.0) / 24.0 * std::f32::consts::TAU;
    let base_factor = phase.sin();
    let sky_factor = base_factor.max(0.05); // Slight moonlight ambient
    cycle.sky_factor = sky_factor;

    for (_, mat) in materials.iter_mut() {
        mat.env.sky_factor = sky_factor;
    }
}

pub fn fluid_tick_system(
    time: Res<Time>,
    mut timer: ResMut<FluidTickTimer>,
    mut world_manager: ResMut<WorldManager>,
    config: Res<crate::config::EngineConfig>,
) {
    let tick_speed = config.fluid_tick_speed;
    timer.0.set_duration(std::time::Duration::from_secs_f32(tick_speed));
    if !timer.0.tick(time.delta()).just_finished() {
        return;
    }

    let current_len = world_manager.fluid_queue.len();
    if current_len == 0 {
        return;
    }

    let mut pushed_this_tick = std::collections::HashSet::new();

    for _ in 0..current_len {
        let Some(pos) = world_manager.fluid_queue.pop_front() else { break; };
        
        if pos.y < 0 || pos.y >= crate::utils::math::WORLD_MAX_Y {
            continue;
        }

        let block = world_manager.get_block_global(pos);
        if block.is_solid() {
            if world_manager.get_fluid_global(pos) > 0 {
                world_manager.set_fluid_global(pos, 0);
            }
            continue;
        }

        let current_raw = world_manager.get_fluid_global(pos);
        let is_source = (current_raw & 0x80) != 0;

        let mut target_level = 0;
        
        if is_source {
            target_level = crate::config::MAX_FLUID_LEVEL;
        } else {
            let above_pos = pos + IVec3::Y;
            if above_pos.y < crate::utils::math::WORLD_MAX_Y {
                let fluid_above_raw = world_manager.get_fluid_global(above_pos);
                let fluid_above = fluid_above_raw & 0x0F;
                
                let block_below = world_manager.get_block_global(pos + IVec3::NEG_Y);
                let is_suspended = !block_below.is_solid();

                if fluid_above > 0 && is_suspended {
                    target_level = crate::config::MAX_FLUID_LEVEL;
                } else {
                    let mut max_n = 0;

                    if fluid_above > 0 && !is_suspended {
                        max_n = crate::config::MAX_FLUID_LEVEL;
                    }

                    for dir in [IVec3::X, IVec3::NEG_X, IVec3::Z, IVec3::NEG_Z] {
                        let npos = pos + dir;
                        
                        let neighbor_block = world_manager.get_block_global(npos);
                        if neighbor_block.is_solid() {
                            continue;
                        }

                        let f_raw = world_manager.get_fluid_global(npos);
                        let f_level = f_raw & 0x0F;
                        let n_is_source = (f_raw & 0x80) != 0;
                        
                        if f_level > 0 {
                            let n_block_directly_below = world_manager.get_block_global(npos + IVec3::NEG_Y);
                            let allow_horizontal_spread = n_is_source || n_block_directly_below.is_solid();
                            
                            if allow_horizontal_spread {
                                let mut min_dist = 999;
                                let npos_fluid_level = world_manager.get_fluid_global(npos) & 0x0F;

                                for inner_dir in [IVec3::X, IVec3::NEG_X, IVec3::Z, IVec3::NEG_Z] {
                                    let target_scan_pos = npos + inner_dir;
                                    let target_fluid_level = world_manager.get_fluid_global(target_scan_pos) & 0x0F;

                                    let dist = if target_fluid_level >= npos_fluid_level {
                                        999
                                    } else {
                                        crate::world::fluid::get_distance_to_drop(&world_manager, npos, inner_dir)
                                    };

                                    if dist < min_dist {
                                        min_dist = dist;
                                    }
                                }

                                let flow_dir = pos - npos;
                                
                                let dist_to_pos = if (world_manager.get_fluid_global(npos + flow_dir) & 0x0F) >= npos_fluid_level {
                                    999
                                } else {
                                    crate::world::fluid::get_distance_to_drop(&world_manager, npos, flow_dir)
                                };

                                let b_curr_pos = world_manager.get_block_global(pos);
                                
                                let allow_flow_here = min_dist == 999 
                                    || dist_to_pos == min_dist 
                                    || (min_dist != 1 && b_curr_pos == BlockType::Air);

                                if allow_flow_here {
                                    if npos_fluid_level > max_n { max_n = npos_fluid_level; }
                                }
                            }
                        }
                    }
                    if max_n > 1 {
                        target_level = max_n - 1;
                    } else {
                        target_level = 0;
                    }
                }
            }
        }

        let target_raw = if is_source {
            target_level | 0x80
        } else {
            target_level
        };

        if current_raw != target_raw {
            world_manager.set_fluid_global(pos, target_raw);
            for dir in [IVec3::Y, IVec3::NEG_Y, IVec3::X, IVec3::NEG_X, IVec3::Z, IVec3::NEG_Z] {
                let npos = pos + dir;
                if npos.y >= 0 && npos.y < crate::utils::math::WORLD_MAX_Y {
                    let neighbor_block = world_manager.get_block_global(npos);
                    let neighbor_fluid = world_manager.get_fluid_global(npos) & 0x0F;
                    if neighbor_block == BlockType::Air || neighbor_fluid > 0 {
                        if pushed_this_tick.insert(npos) {
                            world_manager.fluid_queue.push_back(npos);
                        }
                    }
                }
            }
        }
    }
}

pub fn setup_world(mut commands: Commands) {
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

pub fn update_chunks(
    mut commands: Commands,
    mut world_manager: ResMut<WorldManager>,
    q_player: Query<&Transform, With<crate::player::Player>>,
    q_chunks: Query<(Entity, &Chunk)>,
    config: Res<crate::config::EngineConfig>,
) {
    let Ok(player_tf) = q_player.get_single() else { return; };

    let player_pos_global = player_tf.translation.as_ivec3();
    let (player_chunk_pos, _) = WorldManager::global_to_chunk_pos(player_pos_global);

    let mut potential_chunks = Vec::new();
    let render_dist = config.render_distance as i32;
    let cy_min = 0;
    let cy_max = crate::utils::math::WORLD_CHUNKS_Y - 1;
    for dx in -render_dist..=render_dist {
        for cy in cy_min..=cy_max {
            for dz in -render_dist..=render_dist {
                let target = IVec3::new(player_chunk_pos.x + dx, cy, player_chunk_pos.z + dz);
                if !world_manager.chunks.contains_key(&target)
                    && !world_manager.loading_chunks.contains(&target)
                    && !world_manager.vacuum_chunks.contains(&target)
                {
                    potential_chunks.push(target);
                }
            }
        }
    }
    
    potential_chunks.sort_by_key(|pos| {
        let diff = *pos - player_chunk_pos;
        let dist2d = diff.x * diff.x + diff.z * diff.z;
        (dist2d, diff.y.abs()) // 🚀 二維柱狀優先排序：優先加載同 XZ 水平距離的整根垂直柱子
    });

    let task_pool = AsyncComputeTaskPool::get();
    let world_type = world_manager.world_type;
    let seed = world_manager.seed;

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
                    let generator = TerrainGenerator { noise_provider: noise };
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

            let (chunk_buffer, non_air_count, fluid_buffer) = if let Some(data) = crate::world::storage::load_chunk_from_disk(pos) {
                let count = data.buffer.blocks.iter().filter(|&&b| b != BlockType::Air).count() as u16;
                let fb = data.fluid_buffer.map(|v| {
                    let mut b = Box::new([0u8; 32768]);
                    let len = v.len().min(32768);
                    b[..len].copy_from_slice(&v[..len]);
                    b
                });
                (data.buffer, count, fb)
            } else {
                match world_type {
                    WorldType::Flat => {
                        let mut chunk = Chunk::new(pos);
                        crate::world::gen::flat::generate(&mut chunk);
                        (chunk.buffer, chunk.non_air_count, None)
                    },
                    WorldType::PerlinHills => {
                        let fbm = Fbm::<Perlin>::new(seed);
                        let noise = TerrainNoise(fbm);
                        let generator = TerrainGenerator { noise_provider: noise };
                        let (b, c) = generator.generate_chunk_data(pos);
                        (b, c, None)
                    },
                    WorldType::FloatingIslands => (ChunkBuffer::default(), 0, None),
                }
            };
            let mut light_buffer = ChunkLightBuffer::default();
            crate::world::lighting::init_sunlight(pos, &chunk_buffer, &mut light_buffer, &max_surface_y_map);
            crate::world::lighting::propagate_sky_light(&chunk_buffer, &mut light_buffer);

            let chunk_data_fluid = fluid_buffer.as_ref().map(|b| b.to_vec());
            (pos, ChunkData { buffer: chunk_buffer, fluid_buffer: chunk_data_fluid }, light_buffer, non_air_count, Box::new(max_surface_y_map))
        });

        commands.spawn(GeneratingChunk(task));
    }

    let unload_distance = config.render_distance as i32 + 1;
    let mut to_remove: Vec<IVec3> = Vec::new();

    for (&chunk_pos, _) in world_manager.chunks.iter() {
        let dx = (chunk_pos.x - player_chunk_pos.x).abs();
        let dz = (chunk_pos.z - player_chunk_pos.z).abs();

        if dx > unload_distance || dz > unload_distance { // 🚀 柱狀視距裁切：徹底無視垂直 Y 軸，保護地底深淵
            to_remove.push(chunk_pos);
        }
    }

    for pos in to_remove {
        if let Some(entry) = world_manager.chunks.remove(&pos) {
            if entry.is_modified {
                if let Some(entity) = entry.entity {
                    if let Ok((_, chunk)) = q_chunks.get(entity) {
                        crate::world::storage::save_chunk_to_disk(pos, ChunkData {
                            buffer: ChunkBuffer { blocks: chunk.buffer.blocks },
                            fluid_buffer: entry.fluid_buffer.as_ref().map(|b| b.to_vec()),
                        });
                    }
                } else {
                    crate::world::storage::save_chunk_to_disk(pos, ChunkData {
                        buffer: ChunkBuffer { blocks: entry.buffer.blocks },
                        fluid_buffer: entry.fluid_buffer.as_ref().map(|b| b.to_vec()),
                    });
                }
            }
            
            if let Some(entity) = entry.entity {
                commands.entity(entity).despawn_recursive();
            }
        }
        world_manager.vacuum_chunks.remove(&pos);
    }
}

pub fn poll_loading_chunks(
    mut commands: Commands,
    mut world_manager: ResMut<WorldManager>,
    mut q_tasks: Query<(Entity, &mut GeneratingChunk)>,
    mut q_chunks: Query<(Entity, &mut Chunk)>,
) {
    for (entity, mut task) in &mut q_tasks {
        if let Some((chunk_pos, chunk_data, light_buffer, non_air_count, max_surface_y_map)) = future::block_on(future::poll_once(&mut task.0)) {
            world_manager.loading_chunks.remove(&chunk_pos);
            world_manager.heightmap_cache.insert(IVec2::new(chunk_pos.x, chunk_pos.z), max_surface_y_map);

            let is_empty_chunk = chunk_data.buffer.is_pure_air() && chunk_data.is_fluid_pure_vacuum();

            if is_empty_chunk {
                world_manager.vacuum_chunks.insert(chunk_pos);
                commands.entity(entity).despawn();
                continue;
            } else {
                let mut chunk = Chunk::new(chunk_pos);
                chunk.buffer = ChunkBuffer { blocks: chunk_data.buffer.blocks };
                chunk.light_buffer = light_buffer.clone();
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
                    buffer:      ChunkBuffer { blocks: chunk_data.buffer.blocks },
                    light_buffer: light_buffer.clone(),
                    fluid_buffer: chunk_data.fluid_buffer.as_ref().map(|v| {
                        let mut b = Box::new([0u8; 32768]);
                        let len = v.len().min(32768);
                        b[..len].copy_from_slice(&v[..len]);
                        b
                    }),
                    entity:      Some(chunk_entity),
                    is_modified: false,
                    is_lighting_ready: false,
                };
                world_manager.chunks.insert(chunk_pos, entry);
            }

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

                                let light   = world_manager.get_sky_light_global(global_pos);
                                let n_light = world_manager.get_sky_light_global(n_global_pos);

                                if light > 1 && n_light < light - 1 && n_block == BlockType::Air {
                                    lighting_queue.push_back(global_pos);
                                } else if n_light > 1 && light < n_light - 1 && this_block == BlockType::Air {
                                    lighting_queue.push_back(n_global_pos);
                                }
                            }
                        }
                    }
                }
            }
            if !lighting_queue.is_empty() {
                crate::world::lighting::propagate_sky_light_global(&mut world_manager, lighting_queue);
            }

            let mut chunks_to_ready = vec![chunk_pos];
            for offset in offsets {
                chunks_to_ready.push(chunk_pos + offset);
            }
            for pos in chunks_to_ready {
                if let Some(entry) = world_manager.chunks.get_mut(&pos) {
                    entry.is_lighting_ready = true;
                    // Directly dirty the ECS component for immediate redraw
                    if let Some(ent) = entry.entity {
                        if let Ok((_, mut c)) = q_chunks.get_mut(ent) {
                            c.is_dirty = true;
                        }
                    }
                }
            }

            commands.entity(entity).despawn();
        }
    }
}
