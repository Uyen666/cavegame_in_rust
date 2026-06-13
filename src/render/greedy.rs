use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use bevy::render::render_asset::RenderAssetUsages;
use crate::world::{Chunk, BlockType, WorldManager};
use crate::utils::math::CHUNK_SIZE;
use super::textures::GameTextures;

#[derive(Clone, Copy, PartialEq, Eq)]
struct MaskState {
    block: BlockType,
    normal: i32,
}

type MeshData = (
    Vec<[f32; 3]>, // positions
    Vec<[f32; 3]>, // normals
    Vec<[f32; 4]>, // colors
    Vec<[f32; 2]>, // UVs
    Vec<u32>,      // indices
);

fn empty_mesh() -> MeshData { Default::default() }

/// 把一個四邊形（Greedy Quad）推進 mesh bucket。
/// UV 座標範圍 (0,0) ~ (quad_w, quad_h)，搭配 Repeat 採樣器，
/// 每個整數單位 = 一格方塊 = 一個紋理 tile，完全不拉伸。
fn push_quad(
    bucket: &mut MeshData,
    v1: [f32; 3], v2: [f32; 3], v3: [f32; 3], v4: [f32; 3],
    normal_vec: [f32; 3],
    color: [f32; 4],
    quad_w: i32,
    quad_h: i32,
    rev: bool,
) {
    let start = bucket.0.len() as u32;
    bucket.0.extend_from_slice(&[v1, v2, v3, v4]);
    bucket.1.extend_from_slice(&[normal_vec; 4]);
    bucket.2.extend_from_slice(&[color; 4]);

    let (w, h) = (quad_w as f32, quad_h as f32);
    // UV 按照 v1→v2→v3→v4 的頂點順序排列
    bucket.3.extend_from_slice(&[
        [0.0, 0.0],
        [w,   0.0],
        [w,   h  ],
        [0.0, h  ],
    ]);

    if rev {
        bucket.4.extend_from_slice(&[
            start, start + 2, start + 1,
            start, start + 3, start + 2,
        ]);
    } else {
        bucket.4.extend_from_slice(&[
            start, start + 1, start + 2,
            start, start + 2, start + 3,
        ]);
    }
}

fn finalize_mesh(data: MeshData, meshes: &mut Assets<Mesh>) -> Handle<Mesh> {
    let (pos, nrm, col, uv, idx) = data;
    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, pos);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL,   nrm);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR,    col);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0,     uv);
    mesh.insert_indices(Indices::U32(idx));
    meshes.add(mesh)
}

pub fn mesh_dirty_chunks(
    mut commands: Commands,
    mut q_chunks: Query<(Entity, &mut Chunk)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    world_manager: Res<WorldManager>,
    game_textures: Option<Res<GameTextures>>,
) {
    let Some(gt) = game_textures else { return; };
    if !gt.ready { return; }

    // 共用材質（每幀 lazy-init 一次即可）
    let mut grass_mat: Option<Handle<StandardMaterial>> = None;
    let mut stone_mat: Option<Handle<StandardMaterial>> = None;

    for (entity, mut chunk) in q_chunks.iter_mut() {
        if !chunk.is_dirty { continue; }

        let mut grass_mesh = empty_mesh();
        let mut stone_mesh = empty_mesh();

        generate_greedy_mesh(&chunk, &world_manager, &mut grass_mesh, &mut stone_mesh);

        commands.entity(entity).despawn_descendants();

        // --- 草地（有紋理）---
        if !grass_mesh.0.is_empty() {
            let mat = grass_mat.get_or_insert_with(|| {
                materials.add(StandardMaterial {
                    base_color: Color::WHITE,
                    base_color_texture: Some(gt.grass.clone()),
                    cull_mode: None,
                    unlit: false,
                    alpha_mode: AlphaMode::Opaque,
                    ..default()
                })
            });
            let child = commands.spawn(PbrBundle {
                mesh: finalize_mesh(grass_mesh, &mut meshes),
                material: mat.clone(),
                transform: Transform::default(),
                ..default()
            }).id();
            commands.entity(entity).add_child(child);
        }

        // --- 石頭（石頭紋理）---
        if !stone_mesh.0.is_empty() {
            let mat = stone_mat.get_or_insert_with(|| {
                materials.add(StandardMaterial {
                    base_color: Color::WHITE,
                    base_color_texture: Some(gt.stone.clone()),
                    cull_mode: None,
                    unlit: false,
                    alpha_mode: AlphaMode::Opaque,
                    ..default()
                })
            });
            let child = commands.spawn(PbrBundle {
                mesh: finalize_mesh(stone_mesh, &mut meshes),
                material: mat.clone(),
                transform: Transform::default(),
                ..default()
            }).id();
            commands.entity(entity).add_child(child);
        }

        chunk.is_dirty = false;
    }
}

fn generate_greedy_mesh(
    chunk: &Chunk,
    _world: &WorldManager,
    out_grass: &mut MeshData,
    out_stone: &mut MeshData,
) {
    for d in 0..3usize {
        let u = (d + 1) % 3;
        let v = (d + 2) % 3;

        let mut x = [0i32; 3];
        let mut q = [0i32; 3];
        q[d] = 1;

        let mut mask = vec![None::<MaskState>; (CHUNK_SIZE * CHUNK_SIZE) as usize];

        for slice in -1..CHUNK_SIZE {
            x[d] = slice;

            // 建立 mask（哪些格子需要生成面）
            let mut n = 0;
            for j in 0..CHUNK_SIZE {
                for i in 0..CHUNK_SIZE {
                    x[v] = j; x[u] = i;

                    let b0 = if x[d] >= 0 {
                        chunk.get_block(x[0], x[1], x[2])
                    } else {
                        BlockType::Air
                    };
                    let b1 = if x[d] < CHUNK_SIZE - 1 {
                        chunk.get_block(x[0]+q[0], x[1]+q[1], x[2]+q[2])
                    } else {
                        BlockType::Air
                    };

                    mask[n] = if b0.is_solid() == b1.is_solid() && b0 == b1 {
                        None
                    } else if b0.is_solid() {
                        Some(MaskState { block: b0, normal: 1 })
                    } else if b1.is_solid() {
                        Some(MaskState { block: b1, normal: -1 })
                    } else {
                        None
                    };
                    n += 1;
                }
            }

            // Greedy 合併 + 生成 quad
            x[d] += 1;
            let mut n = 0;
            for j in 0..CHUNK_SIZE {
                let mut i = 0;
                while i < CHUNK_SIZE {
                    if let Some(ms) = mask[n] {
                        // 往 u 方向貪婪擴展寬度
                        let mut w = 1;
                        while i + w < CHUNK_SIZE && mask[n + w as usize] == Some(ms) {
                            w += 1;
                        }

                        // 往 v 方向貪婪擴展高度
                        let mut h = 1;
                        'outer: while j + h < CHUNK_SIZE {
                            for k in 0..w {
                                if mask[n + (h * CHUNK_SIZE + k) as usize] != Some(ms) {
                                    break 'outer;
                                }
                            }
                            h += 1;
                        }

                        x[u] = i; x[v] = j;
                        let mut du = [0i32; 3]; du[u] = w;
                        let mut dv = [0i32; 3]; dv[v] = h;

                        let v1 = [x[0] as f32,           x[1] as f32,           x[2] as f32];
                        let v2 = [(x[0]+du[0]) as f32,   (x[1]+du[1]) as f32,   (x[2]+du[2]) as f32];
                        let v3 = [(x[0]+du[0]+dv[0]) as f32, (x[1]+du[1]+dv[1]) as f32, (x[2]+du[2]+dv[2]) as f32];
                        let v4 = [(x[0]+dv[0]) as f32,   (x[1]+dv[1]) as f32,   (x[2]+dv[2]) as f32];

                        let normal_vec = match d {
                            0 => [ms.normal as f32, 0.0, 0.0],
                            1 => [0.0, ms.normal as f32, 0.0],
                            2 => [0.0, 0.0, ms.normal as f32],
                            _ => [0.0, 0.0, 0.0],
                        };

                        // 頂點顏色：都設為白色，由紋理來提供顏色
                        let color = [1.0, 1.0, 1.0, 1.0];

                        // 捲繞方向
                        let rev = {
                            let r = ms.normal > 0;
                            if d == 0 || d == 2 { !r } else { r }
                        };

                        // 選對應的 bucket
                        match ms.block {
                            BlockType::Grass | BlockType::Dirt =>
                                push_quad(out_grass, v1, v2, v3, v4, normal_vec, color, w, h, rev),
                            _ =>
                                push_quad(out_stone, v1, v2, v3, v4, normal_vec, color, w, h, rev),
                        }

                        // 清除已處理的 mask 格子
                        for l in 0..h {
                            for k in 0..w {
                                mask[n + (l * CHUNK_SIZE + k) as usize] = None;
                            }
                        }

                        i += w; n += w as usize;
                    } else {
                        i += 1; n += 1;
                    }
                }
            }
        }
    }
}
