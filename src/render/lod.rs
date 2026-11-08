#![allow(dead_code)]
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderRef, ShaderType};
use bevy::tasks::{AsyncComputeTaskPool, Task};
use bevy::utils::{HashMap, HashSet};
use futures_lite::future;
use noise::{Fbm, Perlin};

use crate::world::generator::{TerrainGenerator, SEA_LEVEL};
use crate::world::{TerrainNoise, WorldManager, WorldType};

/// 🚀 Voxy 風格微體素階梯 (Voxelized Stepped LOD) 地平線系統
/// 核心設計哲學：
/// 1. 幾何本質純體素化 (Voxels All The Way Down)：
///    - 徹底廢除 2.5D 平滑斜坡三角形！每個 8x8 米柱體由一個水平頂面（Top Quad）與垂直側壁（Side Quads）構成。
///    - 遠山徹底保留經典 Minecraft 體素階梯輪廓，輪廓與近處方塊 100% 統一，徹底消除黏土感與饅頭山！
/// 2. 水下海底連續延伸 + 半透明遠景海水：
///    - LOD 地形網格完整生成海平面以下的海底沙灘與海床岩石，不強行抹平。
///    - 水面單獨生成一層半透明網格（AlphaMode::Blend），透出水下海床，消除刀切色塊斷崖！
/// 3. 遠景樹冠微結構 (Canopy Voxels)：
///    - 採樣樹木分佈，在丘陵森林疊加 5 米高的深綠色樹冠方塊，遠山覆蓋茂密植被，杜絕光頭山。
/// 4. 體素與遠景大氣迷霧全域同步：
///    - 在 380m~1800m 平滑過渡，消滅近景交接邊界的色調突變。

pub const LOD_REGION_SIZE: i32 = 256; // 8 區塊 * 32 格 = 256 米
pub const LOD_STEP: i32 = 8;          // 採樣步長 8 米 (每個微體素階梯 8x8 米)
pub const LOD_CELLS: usize = (LOD_REGION_SIZE / LOD_STEP) as usize; // 32 個微體素列
pub const LOD_MAX_RADIUS: i32 = 8;    // 半徑 8 個區域 = 2,048 米 (64 區塊)
pub const LOD_FOG_START: f32 = 380.0; // 大氣迷霧起始距離 (在方塊邊界 480m 前溫和起步，消除視覺突變)
pub const LOD_FOG_END: f32 = 1800.0;  // 大氣迷霧結束完全融於天空之距離

#[allow(dead_code)]
#[derive(Clone, Default, ShaderType, Debug)]
pub struct LODEnvironmentUniform {
    pub near_voxel_radius: f32, // 近景體素防護罩半徑 (在此半徑內的 LOD 像素強制 Discard 剔除)
    pub fog_start: f32,
    pub fog_end: f32,
    pub sky_factor: f32,        // 晝夜天空光強度係數 [0.05 ~ 1.0] (與近景方塊感官光照曲線 100% 同步)
    pub fog_color: Vec4,
}

/// 🏔️ 體素階梯地表材質 (不透明 / AlphaMask，寫入深度緩衝)
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

/// 🌊 遠景大洋水面材質 (半透明混合 / AlphaMode::Blend，透出水下海床沙石)
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct LODWaterMaterial {
    #[uniform(0)]
    pub env: LODEnvironmentUniform,
}

impl Material for LODWaterMaterial {
    fn vertex_shader() -> ShaderRef {
        "shaders/lod.wgsl".into()
    }

    fn fragment_shader() -> ShaderRef {
        "shaders/lod.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Blend
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
pub struct GeneratingLOD(pub Task<(IVec2, Mesh, Option<Mesh>)>);

#[derive(Resource)]
pub struct LODManager {
    pub loaded_regions: HashMap<IVec2, Entity>,
    pub loading_regions: HashSet<IVec2>,
    pub terrain_material: Handle<LODMaterial>,
    pub water_material: Handle<LODWaterMaterial>,
    pub timer: f32,
    pub last_player_region: IVec2,
    pub initialized: bool,
}

pub struct LODPlugin;

impl Plugin for LODPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            MaterialPlugin::<LODMaterial> {
                prepass_enabled: false,
                ..default()
            },
            MaterialPlugin::<LODWaterMaterial> {
                prepass_enabled: false,
                ..default()
            },
        ))
        .add_systems(Startup, setup_lod)
        .add_systems(Update, (update_lod_regions, poll_lod_tasks));
    }
}

pub fn setup_lod(
    mut commands: Commands,
    mut terrain_materials: ResMut<Assets<LODMaterial>>,
    mut water_materials: ResMut<Assets<LODWaterMaterial>>,
    config: Res<crate::config::EngineConfig>,
) {
    let voxel_radius = (config.render_distance as f32 * 32.0) - 16.0;
    let env = LODEnvironmentUniform {
        near_voxel_radius: voxel_radius,
        fog_start: LOD_FOG_START,
        fog_end: LOD_FOG_END,
        sky_factor: 1.0,
        fog_color: Vec4::new(0.5, 0.8, 1.0, 1.0),
    };

    let terrain_material = terrain_materials.add(LODMaterial { env: env.clone() });
    let water_material = water_materials.add(LODWaterMaterial { env });

    commands.insert_resource(LODManager {
        loaded_regions: HashMap::new(),
        loading_regions: HashSet::new(),
        terrain_material,
        water_material,
        timer: 0.0,
        last_player_region: IVec2::splat(i32::MAX),
        initialized: false,
    });
}

/// 🚀 Voxy 風格微體素階梯網格生成算法：
/// 1. 返回 (地形微體素階梯網格, 遠海水面網格選項)
/// 2. 每個 8x8 米柱體由 1 個水平頂面 (Top Quad) 與相鄰高差的垂直側壁 (Side Quads) 構成。
/// 3. 若有低於海平面之區域，地形網格忠實生成水下海床，並在水面生成半透明水體網格。
pub fn generate_lod_region_mesh(
    rx: i32,
    rz: i32,
    seed: u32,
    world_type: WorldType,
) -> (Mesh, Option<Mesh>) {
    let base_gx = rx * LOD_REGION_SIZE;
    let base_gz = rz * LOD_REGION_SIZE;
    let sun_dir = Vec3::new(0.4, 0.8, 0.3).normalize();

    let mut positions = Vec::with_capacity(LOD_CELLS * LOD_CELLS * 8);
    let mut normals = Vec::with_capacity(LOD_CELLS * LOD_CELLS * 8);
    let mut colors = Vec::with_capacity(LOD_CELLS * LOD_CELLS * 8);
    let mut indices = Vec::with_capacity(LOD_CELLS * LOD_CELLS * 12);

    let mut water_positions = Vec::new();
    let mut water_normals = Vec::new();
    let mut water_colors = Vec::new();
    let mut water_indices = Vec::new();

    match world_type {
        WorldType::Flat => {
            for cz in 0..LOD_CELLS {
                let z0 = (cz as i32 * LOD_STEP) as f32;
                let z1 = ((cz as i32 + 1) * LOD_STEP) as f32;
                for cx in 0..LOD_CELLS {
                    let x0 = (cx as i32 * LOD_STEP) as f32;
                    let x1 = ((cx as i32 + 1) * LOD_STEP) as f32;

                    let base_idx = positions.len() as u32;
                    positions.push([x0, 4.0, z0]);
                    positions.push([x0, 4.0, z1]);
                    positions.push([x1, 4.0, z1]);
                    positions.push([x1, 4.0, z0]);

                    for _ in 0..4 {
                        normals.push([0.0, 1.0, 0.0]);
                        colors.push([0.30, 0.55, 0.22, 1.0]);
                    }

                    indices.extend_from_slice(&[
                        base_idx, base_idx + 1, base_idx + 2,
                        base_idx, base_idx + 2, base_idx + 3,
                    ]);
                }
            }
        }
        WorldType::PerlinHills => {
            let fbm = Fbm::<Perlin>::new(seed);
            let noise = TerrainNoise(fbm);
            let generator = TerrainGenerator { noise_provider: noise };

            // 建立 34x34 快取查找表，涵蓋 cx: -1..=32, cz: -1..=32
            // 包含鄰接區域邊界高度，以便無縫縫合跨區域垂直側壁
            const TABLE_DIM: usize = LOD_CELLS + 2; // 34
            let mut height_table = [0i32; TABLE_DIM * TABLE_DIM];
            let mut foliage_table = [false; TABLE_DIM * TABLE_DIM];

            for svz in 0..TABLE_DIM {
                let cell_z = svz as i32 - 1;
                let sample_z = base_gz + cell_z * LOD_STEP + LOD_STEP / 2;
                for svx in 0..TABLE_DIM {
                    let cell_x = svx as i32 - 1;
                    let sample_x = base_gx + cell_x * LOD_STEP + LOD_STEP / 2;

                    let y = generator.get_max_surface_y(sample_x, sample_z);

                    // 🌲 檢測樹木分佈微結構
                    let mut h = (sample_x as u32).wrapping_mul(374761393).wrapping_add((sample_z as u32).wrapping_mul(668265263));
                    h = (h ^ (h >> 13)).wrapping_mul(1274126177);
                    h ^= h >> 16;
                    let has_tree = (h % 1000 < 30) && (y > SEA_LEVEL + 4) && (y <= 500);

                    let idx = svz * TABLE_DIM + svx;
                    height_table[idx] = if has_tree { y + 5 } else { y };
                    foliage_table[idx] = has_tree;
                }
            }

            for cz in 0..LOD_CELLS {
                let z0 = (cz as i32 * LOD_STEP) as f32;
                let z1 = ((cz as i32 + 1) * LOD_STEP) as f32;

                for cx in 0..LOD_CELLS {
                    let x0 = (cx as i32 * LOD_STEP) as f32;
                    let x1 = ((cx as i32 + 1) * LOD_STEP) as f32;

                    let table_idx = (cz + 1) * TABLE_DIM + (cx + 1);
                    let curr_y = height_table[table_idx];
                    let is_tree = foliage_table[table_idx];

                    let y_f32 = curr_y as f32;

                    // ── 1. 水平微體素頂面 (Top Quad, normal = +Y) ──
                    let base_idx = positions.len() as u32;
                    positions.push([x0, y_f32, z0]);
                    positions.push([x0, y_f32, z1]);
                    positions.push([x1, y_f32, z1]);
                    positions.push([x1, y_f32, z0]);

                    let n_dot_l = Vec3::Y.dot(sun_dir).max(0.0);
                    let top_diffuse = 0.65 + 0.35 * n_dot_l;

                    let top_base_rgb = if curr_y < SEA_LEVEL {
                        Vec3::new(0.34, 0.33, 0.30) // 🌊 深海海底沙礫
                    } else if curr_y <= SEA_LEVEL + 3 {
                        Vec3::new(0.76, 0.72, 0.50) // 🏖️ 金色濱海沙灘
                    } else if curr_y > 580 {
                        Vec3::new(0.90, 0.92, 0.96) // ❄️ 高山積雪
                    } else if curr_y > 520 {
                        Vec3::new(0.48, 0.48, 0.50) // 🪨 風化花崗岩
                    } else if is_tree {
                        Vec3::new(0.18, 0.36, 0.14) // 🌲 森林樹冠頂部
                    } else {
                        Vec3::new(0.26, 0.48, 0.18) // 🌿 溫帶常綠草坪
                    };

                    let top_shaded = top_base_rgb * top_diffuse;
                    for _ in 0..4 {
                        normals.push([0.0, 1.0, 0.0]);
                        colors.push([top_shaded.x, top_shaded.y, top_shaded.z, 1.0]);
                    }
                    indices.extend_from_slice(&[
                        base_idx, base_idx + 1, base_idx + 2,
                        base_idx, base_idx + 2, base_idx + 3,
                    ]);

                    // ── 2. 四向垂直側壁階梯 (Side Quads) ──
                    let side_base_rgb = if curr_y > 520 {
                        Vec3::new(0.42, 0.42, 0.44) // 岩壁
                    } else if curr_y < SEA_LEVEL {
                        Vec3::new(0.26, 0.26, 0.28) // 海底岩壁
                    } else if is_tree {
                        Vec3::new(0.22, 0.32, 0.16) // 樹冠側壁
                    } else {
                        Vec3::new(0.42, 0.30, 0.18) // 泥土棕色峭壁
                    };

                    // 西側 (-X)
                    let y_west = height_table[table_idx - 1];
                    if curr_y > y_west {
                        let y_low = y_west as f32;
                        let s_idx = positions.len() as u32;
                        positions.push([x0, y_low, z1]);
                        positions.push([x0, y_f32, z1]);
                        positions.push([x0, y_f32, z0]);
                        positions.push([x0, y_low, z0]);

                        let norm = Vec3::new(-1.0, 0.0, 0.0);
                        let diff = 0.48 + 0.52 * norm.dot(sun_dir).max(0.0);
                        let shaded = side_base_rgb * diff;
                        for _ in 0..4 {
                            normals.push([-1.0, 0.0, 0.0]);
                            colors.push([shaded.x, shaded.y, shaded.z, 1.0]);
                        }
                        indices.extend_from_slice(&[
                            s_idx, s_idx + 1, s_idx + 2,
                            s_idx, s_idx + 2, s_idx + 3,
                        ]);
                    }

                    // 東側 (+X)
                    let y_east = height_table[table_idx + 1];
                    if curr_y > y_east {
                        let y_low = y_east as f32;
                        let s_idx = positions.len() as u32;
                        positions.push([x1, y_low, z0]);
                        positions.push([x1, y_f32, z0]);
                        positions.push([x1, y_f32, z1]);
                        positions.push([x1, y_low, z1]);

                        let norm = Vec3::new(1.0, 0.0, 0.0);
                        let diff = 0.48 + 0.52 * norm.dot(sun_dir).max(0.0);
                        let shaded = side_base_rgb * diff;
                        for _ in 0..4 {
                            normals.push([1.0, 0.0, 0.0]);
                            colors.push([shaded.x, shaded.y, shaded.z, 1.0]);
                        }
                        indices.extend_from_slice(&[
                            s_idx, s_idx + 1, s_idx + 2,
                            s_idx, s_idx + 2, s_idx + 3,
                        ]);
                    }

                    // 北側 (-Z)
                    let y_north = height_table[table_idx - TABLE_DIM];
                    if curr_y > y_north {
                        let y_low = y_north as f32;
                        let s_idx = positions.len() as u32;
                        positions.push([x0, y_low, z0]);
                        positions.push([x0, y_f32, z0]);
                        positions.push([x1, y_f32, z0]);
                        positions.push([x1, y_low, z0]);

                        let norm = Vec3::new(0.0, 0.0, -1.0);
                        let diff = 0.48 + 0.52 * norm.dot(sun_dir).max(0.0);
                        let shaded = side_base_rgb * diff;
                        for _ in 0..4 {
                            normals.push([0.0, 0.0, -1.0]);
                            colors.push([shaded.x, shaded.y, shaded.z, 1.0]);
                        }
                        indices.extend_from_slice(&[
                            s_idx, s_idx + 1, s_idx + 2,
                            s_idx, s_idx + 2, s_idx + 3,
                        ]);
                    }

                    // 南側 (+Z)
                    let y_south = height_table[table_idx + TABLE_DIM];
                    if curr_y > y_south {
                        let y_low = y_south as f32;
                        let s_idx = positions.len() as u32;
                        positions.push([x1, y_low, z1]);
                        positions.push([x1, y_f32, z1]);
                        positions.push([x0, y_f32, z1]);
                        positions.push([x0, y_low, z1]);

                        let norm = Vec3::new(0.0, 0.0, 1.0);
                        let diff = 0.48 + 0.52 * norm.dot(sun_dir).max(0.0);
                        let shaded = side_base_rgb * diff;
                        for _ in 0..4 {
                            normals.push([0.0, 0.0, 1.0]);
                            colors.push([shaded.x, shaded.y, shaded.z, 1.0]);
                        }
                        indices.extend_from_slice(&[
                            s_idx, s_idx + 1, s_idx + 2,
                            s_idx, s_idx + 2, s_idx + 3,
                        ]);
                    }

                    // ── 3. 半透明大洋水面頂面 (Water Quad, Y = SEA_LEVEL) ──
                    if curr_y < SEA_LEVEL {
                        let w_idx = water_positions.len() as u32;
                        let sea_y = SEA_LEVEL as f32;
                        water_positions.push([x0, sea_y, z0]);
                        water_positions.push([x0, sea_y, z1]);
                        water_positions.push([x1, sea_y, z1]);
                        water_positions.push([x1, sea_y, z0]);

                        let water_color = [0.15, 0.35, 0.75, 0.60];
                        for _ in 0..4 {
                            water_normals.push([0.0, 1.0, 0.0]);
                            water_colors.push(water_color);
                        }
                        water_indices.extend_from_slice(&[
                            w_idx, w_idx + 1, w_idx + 2,
                            w_idx, w_idx + 2, w_idx + 3,
                        ]);
                    }
                }
            }
        }
        WorldType::FloatingIslands => {}
    }

    let mut terrain_mesh = Mesh::new(
        bevy::render::mesh::PrimitiveTopology::TriangleList,
        bevy::render::render_asset::RenderAssetUsages::default(),
    );
    terrain_mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    terrain_mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    terrain_mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    terrain_mesh.insert_indices(bevy::render::mesh::Indices::U32(indices));

    let water_mesh = if !water_positions.is_empty() {
        let mut wm = Mesh::new(
            bevy::render::mesh::PrimitiveTopology::TriangleList,
            bevy::render::render_asset::RenderAssetUsages::default(),
        );
        wm.insert_attribute(Mesh::ATTRIBUTE_POSITION, water_positions);
        wm.insert_attribute(Mesh::ATTRIBUTE_NORMAL, water_normals);
        wm.insert_attribute(Mesh::ATTRIBUTE_COLOR, water_colors);
        wm.insert_indices(bevy::render::mesh::Indices::U32(water_indices));
        Some(wm)
    } else {
        None
    };

    (terrain_mesh, water_mesh)
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

    let voxel_radius = (config.render_distance as f32 * 32.0) - 16.0;

    // 1. 淘汰超出最大地平線半徑 (LOD_MAX_RADIUS + 1) 或完全被近景體素覆蓋之區域
    let max_radius_sq = (LOD_MAX_RADIUS + 1) * (LOD_MAX_RADIUS + 1);
    let mut to_despawn = Vec::new();

    for (&rpos, &entity) in lod_manager.loaded_regions.iter() {
        let dx = rpos.x - player_rx;
        let dz = rpos.y - player_rz;
        let dist_sq = dx * dx + dz * dz;

        let r_center_x = (rpos.x * LOD_REGION_SIZE + LOD_REGION_SIZE / 2) as f32;
        let r_center_z = (rpos.y * LOD_REGION_SIZE + LOD_REGION_SIZE / 2) as f32;
        let center_dist = ((r_center_x - player_pos.x).powi(2) + (r_center_z - player_pos.z).powi(2)).sqrt();
        let region_half_diag = (LOD_REGION_SIZE as f32 / 2.0) * 1.4142;

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

    candidates.sort_by_key(|(_, d)| *d);

    let seed = world_manager.seed;
    let world_type = world_manager.world_type;
    let task_pool = AsyncComputeTaskPool::get();

    for (rpos, _) in candidates.into_iter().take(4) {
        lod_manager.loading_regions.insert(rpos);

        let task = task_pool.spawn(async move {
            let (terrain_mesh, water_mesh) = generate_lod_region_mesh(rpos.x, rpos.y, seed, world_type);
            (rpos, terrain_mesh, water_mesh)
        });

        commands.spawn(GeneratingLOD(task));
    }
}

/// 🚀 輪詢異步遠景網格任務並實例化 Entity (包含海床與半透明遠洋水體)
pub fn poll_lod_tasks(
    mut commands: Commands,
    mut lod_manager: ResMut<LODManager>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut q_tasks: Query<(Entity, &mut GeneratingLOD)>,
) {
    for (entity, mut task) in &mut q_tasks {
        if let Some((rpos, terrain_mesh, water_mesh)) = future::block_on(future::poll_once(&mut task.0)) {
            lod_manager.loading_regions.remove(&rpos);

            let terrain_mesh_handle = meshes.add(terrain_mesh);
            let terrain_material_handle = lod_manager.terrain_material.clone();

            let region_entity = commands.spawn((
                LODRegionChunk {
                    rx: rpos.x,
                    rz: rpos.y,
                },
                terrain_mesh_handle,
                terrain_material_handle,
                SpatialBundle {
                    transform: Transform::from_xyz(
                        (rpos.x * LOD_REGION_SIZE) as f32,
                        0.0,
                        (rpos.y * LOD_REGION_SIZE) as f32,
                    ),
                    ..default()
                },
            )).id();

            if let Some(wmesh) = water_mesh {
                let water_mesh_handle = meshes.add(wmesh);
                let water_material_handle = lod_manager.water_material.clone();
                commands.entity(region_entity).with_children(|parent| {
                    parent.spawn((
                        water_mesh_handle,
                        water_material_handle,
                        SpatialBundle::default(),
                    ));
                });
            }

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
        let (mesh, water) = generate_lod_region_mesh(0, 0, 1337, WorldType::Flat);
        assert!(water.is_none(), "Flat world has no water below sea level");

        let positions = mesh.attribute(Mesh::ATTRIBUTE_POSITION).unwrap();
        // 32 * 32 cells * 4 vertices = 4096
        assert_eq!(positions.len(), LOD_CELLS * LOD_CELLS * 4);
        assert_eq!(positions.len(), 4096);

        let indices = mesh.indices().unwrap();
        let expected_tris = LOD_CELLS * LOD_CELLS * 2;
        assert_eq!(indices.len(), expected_tris * 3);
    }

    #[test]
    fn test_lod_region_coordinates_and_sea_level_anchor() {
        let (terrain_mesh, water_mesh) = generate_lod_region_mesh(2, -3, 42, WorldType::PerlinHills);
        let positions = terrain_mesh.attribute(Mesh::ATTRIBUTE_POSITION).unwrap();
        let colors = terrain_mesh.attribute(Mesh::ATTRIBUTE_COLOR).unwrap();

        assert!(positions.len() >= LOD_CELLS * LOD_CELLS * 4);
        assert_eq!(positions.len(), colors.len());

        match positions {
            bevy::render::mesh::VertexAttributeValues::Float32x3(pts) => {
                for p in pts {
                    assert!(p[1] >= 0.0, "Vertex height cannot be negative: {}", p[1]);
                    assert!(p[1] <= 1024.0, "Vertex height exceeded max world limit: {}", p[1]);
                }
            }
            _ => panic!("Expected Float32x3 position attributes"),
        }

        if let Some(wmesh) = water_mesh {
            let w_positions = wmesh.attribute(Mesh::ATTRIBUTE_POSITION).unwrap();
            match w_positions {
                bevy::render::mesh::VertexAttributeValues::Float32x3(pts) => {
                    for p in pts {
                        assert_eq!(p[1], SEA_LEVEL as f32, "Water height must be exactly SEA_LEVEL");
                    }
                }
                _ => panic!("Expected Float32x3 water position attributes"),
            }
        }
    }
}
