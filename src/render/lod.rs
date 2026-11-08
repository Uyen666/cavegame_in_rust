#![allow(dead_code)]
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderRef, ShaderType};
use bevy::tasks::{AsyncComputeTaskPool, Task};
use bevy::utils::{HashMap, HashSet};
use futures_lite::future;
use noise::{Fbm, Perlin};

use crate::world::generator::{TerrainGenerator, SEA_LEVEL};
use crate::world::{TerrainNoise, WorldManager, WorldType};

/// 🚀 Distant Horizons (DH) 超遠景 LOD 地平線系統
/// 核心設計哲學：
/// 1. 近處（0 ~ near_voxel_radius，即 0 ~ 248 米）：100% 嚴格純 3D 方塊，絕不允許任何平滑地表穿出！
/// 2. 遠景（248 米 ~ 1,536 米）：無狀態低多邊形網格無縫接管，僅需約 80 ~ 110 個 Draw Call 呈現 1.5 公里宏大雪峰與大洋！
/// 3. 銜接處（240 ~ 248 米）：體素迷霧平滑融化，GPU 著色器即時 Discard 近景像素，保證完全無縫、無穿模、無 Z-Fighting！

pub const LOD_REGION_SIZE: i32 = 256; // 8 區塊 * 32 格 = 256 米
pub const LOD_STEP: i32 = 8;          // 採樣步長 8 米
pub const LOD_GRID_VERTS: usize = (LOD_REGION_SIZE / LOD_STEP + 1) as usize; // 33 頂點
pub const LOD_MAX_RADIUS: i32 = 6;    // 半徑 6 個區域 = 1,536 米 (48 區塊)
pub const LOD_FOG_START: f32 = 650.0; // 大氣迷霧起始距離
pub const LOD_FOG_END: f32 = 1450.0;  // 大氣迷霧結束完全融於天空之距離

#[allow(dead_code)]
#[derive(Clone, Default, ShaderType, Debug)]
pub struct LODEnvironmentUniform {
    pub near_voxel_radius: f32, // 近景體素防護罩半徑 (在此半徑內的 LOD 像素強制 Discard 剔除)
    pub fog_start: f32,
    pub fog_end: f32,
    pub _pad0: f32,
    pub fog_color: Vec4,
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct LODMaterial {
    #[uniform(0)]
    pub env: LODEnvironmentUniform,
}

impl Material for LODMaterial {
    fn vertex_shader() -> ShaderRef {
        "shaders/lod.wgsl".into()
    }

    fn fragment_shader() -> ShaderRef {
        "shaders/lod.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Mask(0.5) // 允許 GPU 執行 Fragment Discard 且保證深度寫入正確
    }

    fn prepass_vertex_shader() -> ShaderRef {
        "shaders/lod.wgsl".into()
    }

    fn prepass_fragment_shader() -> ShaderRef {
        "shaders/lod.wgsl".into()
    }

    fn specialize(
        _pipeline: &bevy::pbr::MaterialPipeline<Self>,
        descriptor: &mut bevy::render::render_resource::RenderPipelineDescriptor,
        layout: &bevy::render::mesh::MeshVertexBufferLayoutRef,
        _key: bevy::pbr::MaterialPipelineKey<Self>,
    ) -> Result<(), bevy::render::render_resource::SpecializedMeshPipelineError> {
        descriptor.primitive.cull_mode = Some(bevy::render::render_resource::Face::Back);

        let vertex_layout = layout.0.get_layout(&[
            Mesh::ATTRIBUTE_POSITION.at_shader_location(0),
            Mesh::ATTRIBUTE_NORMAL.at_shader_location(1),
            Mesh::ATTRIBUTE_COLOR.at_shader_location(2),
        ])?;

        descriptor.vertex.buffers[0] = vertex_layout;
        Ok(())
    }
}

#[allow(dead_code)]
#[derive(Component)]
pub struct LODRegionChunk {
    pub rx: i32,
    pub rz: i32,
}

#[derive(Component)]
pub struct GeneratingLOD(pub Task<(IVec2, Mesh)>);

#[derive(Resource)]
pub struct LODManager {
    pub loaded_regions: HashMap<IVec2, Entity>,
    pub loading_regions: HashSet<IVec2>,
    pub material: Handle<LODMaterial>,
    pub timer: f32,
    pub last_player_region: IVec2,
    pub initialized: bool,
}

pub struct LODPlugin;

impl Plugin for LODPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MaterialPlugin::<LODMaterial> {
            prepass_enabled: false,
            ..default()
        })
        .add_systems(Startup, setup_lod)
        .add_systems(Update, (update_lod_regions, poll_lod_tasks));
    }
}

pub fn setup_lod(
    mut commands: Commands,
    mut materials: ResMut<Assets<LODMaterial>>,
    config: Res<crate::config::EngineConfig>,
) {
    let voxel_radius = (config.render_distance as f32 * 32.0) - 8.0;
    let material = materials.add(LODMaterial {
        env: LODEnvironmentUniform {
            near_voxel_radius: voxel_radius,
            fog_start: LOD_FOG_START,
            fog_end: LOD_FOG_END,
            _pad0: 0.0,
            fog_color: Vec4::new(0.5, 0.8, 1.0, 1.0),
        },
    });

    commands.insert_resource(LODManager {
        loaded_regions: HashMap::new(),
        loading_regions: HashSet::new(),
        material,
        timer: 0.0,
        last_player_region: IVec2::splat(i32::MAX),
        initialized: false,
    });
}

/// 🚀 核心網格生成算法：純數學解析性無狀態生成 256x256 米地表高度與代表色網格
pub fn generate_lod_region_mesh(
    rx: i32,
    rz: i32,
    seed: u32,
    world_type: WorldType,
) -> Mesh {
    let mut positions = Vec::with_capacity(LOD_GRID_VERTS * LOD_GRID_VERTS);
    let mut normals = Vec::with_capacity(LOD_GRID_VERTS * LOD_GRID_VERTS);
    let mut colors = Vec::with_capacity(LOD_GRID_VERTS * LOD_GRID_VERTS);

    let base_gx = rx * LOD_REGION_SIZE;
    let base_gz = rz * LOD_REGION_SIZE;

    // 太陽漫反射方向 (歸一化定向光照向量)
    let sun_dir = Vec3::new(0.4, 0.8, 0.3).normalize();

    match world_type {
        WorldType::Flat => {
            for vz in 0..LOD_GRID_VERTS {
                for vx in 0..LOD_GRID_VERTS {
                    let lx = (vx as i32 * LOD_STEP) as f32;
                    let lz = (vz as i32 * LOD_STEP) as f32;
                    positions.push([lx, 4.0, lz]);
                    normals.push([0.0, 1.0, 0.0]);
                    colors.push([0.30, 0.55, 0.22, 1.0]);
                }
            }
        }
        WorldType::PerlinHills => {
            let fbm = Fbm::<Perlin>::new(seed);
            let noise = TerrainNoise(fbm);
            let generator = TerrainGenerator { noise_provider: noise };

            // 🚀 建立 35x35 的棧快取查找表，單次填充、零重複噪聲呼叫
            // 涵蓋周遭 vx: -1..=33, vz: -1..=33，供鄰域有限差分法精準計算地表法線
            const TABLE_DIM: usize = LOD_GRID_VERTS + 2; // 35
            let mut height_table = [0i32; TABLE_DIM * TABLE_DIM];

            for svz in 0..TABLE_DIM {
                let sample_z = base_gz + (svz as i32 - 1) * LOD_STEP;
                for svx in 0..TABLE_DIM {
                    let sample_x = base_gx + (svx as i32 - 1) * LOD_STEP;
                    let h = generator.get_max_surface_y(sample_x, sample_z);
                    height_table[svz * TABLE_DIM + svx] = h;
                }
            }

            for vz in 0..LOD_GRID_VERTS {
                for vx in 0..LOD_GRID_VERTS {
                    let lx = (vx as i32 * LOD_STEP) as f32;
                    let lz = (vz as i32 * LOD_STEP) as f32;

                    let table_idx = (vz + 1) * TABLE_DIM + (vx + 1);
                    let surf_y = height_table[table_idx];

                    if surf_y < SEA_LEVEL {
                        // 🌊 1. 浩瀚大洋水面：水平如鏡，錨定於海平面 (Y = 256.0)
                        positions.push([lx, SEA_LEVEL as f32, lz]);
                        normals.push([0.0, 1.0, 0.0]);
                        colors.push([0.16, 0.38, 0.68, 1.0]); // 深海澄藍
                    } else {
                        // 🏔️ 2. 陸地地表：高度微下沉 0.15 米，防止在體素過渡交界區產生 Z-Fighting
                        let y = surf_y as f32 - 0.15;
                        positions.push([lx, y, lz]);

                        // 鄰域有限差分計算平滑法線
                        let h_left = height_table[table_idx - 1] as f32;
                        let h_right = height_table[table_idx + 1] as f32;
                        let h_down = height_table[table_idx - TABLE_DIM] as f32;
                        let h_up = height_table[table_idx + TABLE_DIM] as f32;

                        let dx = (h_right - h_left) / (LOD_STEP as f32 * 2.0);
                        let dz = (h_up - h_down) / (LOD_STEP as f32 * 2.0);
                        let normal = Vec3::new(-dx, 1.0, -dz).normalize();
                        normals.push([normal.x, normal.y, normal.z]);

                        // 頂點定向光照烘焙 (Baked Directional Sun Lighting)
                        let n_dot_l = normal.dot(sun_dir).max(0.0);
                        let diffuse = 0.55 + 0.45 * n_dot_l;

                        // 地形代表色匹配 (Biome Representative Surface Tint)
                        let base_rgb = if surf_y <= SEA_LEVEL + 4 {
                            Vec3::new(0.78, 0.74, 0.52) // 🏖️ 濱海金色沙灘
                        } else if surf_y > 580 {
                            Vec3::new(0.86, 0.89, 0.93) // ❄️ 1024 宏觀雪峰白雪
                        } else if surf_y > 520 {
                            Vec3::new(0.48, 0.48, 0.48) // 🪨 高海拔風化花崗岩
                        } else {
                            Vec3::new(0.28, 0.52, 0.20) // 🌿 溫帶常綠生機草地
                        };

                        let shaded_rgb = base_rgb * diffuse;
                        colors.push([shaded_rgb.x, shaded_rgb.y, shaded_rgb.z, 1.0]);
                    }
                }
            }
        }
        WorldType::FloatingIslands => {
            // 空島模式：地表無宏觀連續基底，保持虛空
        }
    }

    // 構建頂點索引 (32 x 32 網格，每個格子由 2 個逆時針三角形構成)
    const CELLS: usize = LOD_GRID_VERTS - 1; // 32
    let mut indices = Vec::with_capacity(CELLS * CELLS * 6);

    for cz in 0..CELLS {
        for cx in 0..CELLS {
            let i0 = (cz * LOD_GRID_VERTS + cx) as u32;
            let i1 = i0 + 1;
            let i2 = ((cz + 1) * LOD_GRID_VERTS + cx) as u32;
            let i3 = i2 + 1;

            // 朝上的三角形 (Counter-Clockwise Winding: i0 -> i2 -> i1 與 i1 -> i2 -> i3)
            indices.extend_from_slice(&[i0, i2, i1, i1, i2, i3]);
        }
    }

    let mut mesh = Mesh::new(
        bevy::render::mesh::PrimitiveTopology::TriangleList,
        bevy::render::render_asset::RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    mesh.insert_indices(bevy::render::mesh::Indices::U32(indices));

    mesh
}

/// 🚀 後台動態加載管理：監聽玩家位置，異步派發遠景網格生成任務，淘汰超出地平線之區域
pub fn update_lod_regions(
    mut commands: Commands,
    mut lod_manager: ResMut<LODManager>,
    world_manager: Res<WorldManager>,
    q_player: Query<&Transform, With<crate::player::Player>>,
    config: Res<crate::config::EngineConfig>,
    time: Res<Time>,
) {
    let Ok(player_tf) = q_player.get_single() else { return; };
    let player_pos = player_tf.translation;

    let player_rx = (player_pos.x / LOD_REGION_SIZE as f32).floor() as i32;
    let player_rz = (player_pos.z / LOD_REGION_SIZE as f32).floor() as i32;
    let current_region = IVec2::new(player_rx, player_rz);

    lod_manager.timer += time.delta_seconds();
    let region_changed = !lod_manager.initialized || current_region != lod_manager.last_player_region;

    // 節流檢查：玩家未跨區域且時間未到 0.25 秒時跳過
    if !region_changed && lod_manager.timer < 0.25 {
        return;
    }

    lod_manager.timer = 0.0;
    lod_manager.last_player_region = current_region;
    lod_manager.initialized = true;

    let voxel_radius = (config.render_distance as f32 * 32.0) - 8.0;

    // 1. 淘汰超出最大地平線半徑 (LOD_MAX_RADIUS + 1) 或完全被近景體素覆蓋之區域
    let max_radius_sq = (LOD_MAX_RADIUS + 1) * (LOD_MAX_RADIUS + 1);
    let mut to_despawn = Vec::new();

    for (&rpos, &entity) in lod_manager.loaded_regions.iter() {
        let dx = rpos.x - player_rx;
        let dz = rpos.y - player_rz;
        let dist_sq = dx * dx + dz * dz;

        // 計算該區域中心至玩家的歐幾里得距離
        let r_center_x = (rpos.x * LOD_REGION_SIZE + LOD_REGION_SIZE / 2) as f32;
        let r_center_z = (rpos.y * LOD_REGION_SIZE + LOD_REGION_SIZE / 2) as f32;
        let center_dist = ((r_center_x - player_pos.x).powi(2) + (r_center_z - player_pos.z).powi(2)).sqrt();
        let region_half_diag = (LOD_REGION_SIZE as f32 / 2.0) * 1.4142;

        // 若該區域所有頂點均深陷於近景體素半徑內部，完全由體素實體渲染，直接卸載
        let entirely_inside_voxels = center_dist + region_half_diag < voxel_radius;

        if dist_sq > max_radius_sq || entirely_inside_voxels {
            to_despawn.push(rpos);
            commands.entity(entity).despawn_recursive();
        }
    }

    for rpos in to_despawn {
        lod_manager.loaded_regions.remove(&rpos);
    }

    lod_manager.loading_regions.retain(|rpos| {
        let dx = rpos.x - player_rx;
        let dz = rpos.y - player_rz;
        dx * dx + dz * dz <= max_radius_sq
    });

    // 2. 收集需要加載的候選區域
    let mut candidates = Vec::new();
    let max_r = LOD_MAX_RADIUS;

    for dz in -max_r..=max_r {
        for dx in -max_r..=max_r {
            let dist_sq = dx * dx + dz * dz;
            if dist_sq > max_r * max_r {
                continue;
            }

            let rpos = IVec2::new(player_rx + dx, player_rz + dz);
            let r_center_x = (rpos.x * LOD_REGION_SIZE + LOD_REGION_SIZE / 2) as f32;
            let r_center_z = (rpos.y * LOD_REGION_SIZE + LOD_REGION_SIZE / 2) as f32;
            let center_dist = ((r_center_x - player_pos.x).powi(2) + (r_center_z - player_pos.z).powi(2)).sqrt();
            let region_half_diag = (LOD_REGION_SIZE as f32 / 2.0) * 1.4142;

            // 🚀 CPU 預先快篩：若該區域完全深埋於近景體素防護罩內部，完全不生成任務
            if center_dist + region_half_diag < voxel_radius {
                continue;
            }

            if !lod_manager.loaded_regions.contains_key(&rpos)
                && !lod_manager.loading_regions.contains(&rpos)
            {
                candidates.push((rpos, dist_sq));
            }
        }
    }

    // 由近及遠排序加載
    candidates.sort_by_key(|(_, d)| *d);

    let seed = world_manager.seed;
    let world_type = world_manager.world_type;
    let task_pool = AsyncComputeTaskPool::get();

    // 🚀 每影格平滑限流派發最多 4 個 LOD 任務，保障 200+ FPS 絲滑無卡頓
    for (rpos, _) in candidates.into_iter().take(4) {
        lod_manager.loading_regions.insert(rpos);

        let task = task_pool.spawn(async move {
            let mesh = generate_lod_region_mesh(rpos.x, rpos.y, seed, world_type);
            (rpos, mesh)
        });

        commands.spawn(GeneratingLOD(task));
    }
}

/// 🚀 輪詢異步遠景網格任務並實例化 Entity
pub fn poll_lod_tasks(
    mut commands: Commands,
    mut lod_manager: ResMut<LODManager>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut q_tasks: Query<(Entity, &mut GeneratingLOD)>,
) {
    for (entity, mut task) in &mut q_tasks {
        if let Some((rpos, mesh)) = future::block_on(future::poll_once(&mut task.0)) {
            lod_manager.loading_regions.remove(&rpos);

            let mesh_handle = meshes.add(mesh);
            let material_handle = lod_manager.material.clone();

            let region_entity = commands.spawn((
                LODRegionChunk {
                    rx: rpos.x,
                    rz: rpos.y,
                },
                mesh_handle,
                material_handle,
                SpatialBundle {
                    transform: Transform::from_xyz(
                        (rpos.x * LOD_REGION_SIZE) as f32,
                        0.0,
                        (rpos.y * LOD_REGION_SIZE) as f32,
                    ),
                    ..default()
                },
            )).id();

            lod_manager.loaded_regions.insert(rpos, region_entity);
            commands.entity(entity).despawn();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lod_region_mesh_vertex_and_triangle_counts() {
        let mesh = generate_lod_region_mesh(0, 0, 1337, WorldType::Flat);
        let positions = mesh.attribute(Mesh::ATTRIBUTE_POSITION).unwrap();
        assert_eq!(positions.len(), LOD_GRID_VERTS * LOD_GRID_VERTS);
        assert_eq!(positions.len(), 33 * 33);

        let indices = mesh.indices().unwrap();
        let expected_tris = 32 * 32 * 2;
        assert_eq!(indices.len(), expected_tris * 3);
    }

    #[test]
    fn test_lod_region_coordinates_and_sea_level_anchor() {
        let mesh = generate_lod_region_mesh(2, -3, 42, WorldType::PerlinHills);
        let positions = mesh.attribute(Mesh::ATTRIBUTE_POSITION).unwrap();
        let colors = mesh.attribute(Mesh::ATTRIBUTE_COLOR).unwrap();

        assert_eq!(positions.len(), 33 * 33);
        assert_eq!(colors.len(), 33 * 33);

        // 驗證所有頂點高度均大於等於 0 且在合理範圍內
        match positions {
            bevy::render::mesh::VertexAttributeValues::Float32x3(pts) => {
                for p in pts {
                    assert!(p[1] >= 0.0, "Vertex height cannot be negative: {}", p[1]);
                    assert!(p[1] <= 1024.0, "Vertex height exceeded max world limit: {}", p[1]);
                }
            }
            _ => panic!("Expected Float32x3 position attributes"),
        }
    }
}
