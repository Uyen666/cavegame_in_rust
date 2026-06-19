use bevy::prelude::*;
use bevy::render::mesh::{Indices, MeshVertexAttribute};
use bevy::render::render_asset::RenderAssetUsages;
use bevy::render::render_resource::{PrimitiveTopology, VertexFormat};
use crate::world::{Chunk, BlockType, WorldManager};
use crate::utils::math::CHUNK_SIZE;
use super::textures::GameTextures;

// ── Custom Attribute ─────────────────────────────────────────────────────────

// 將 x(6), y(6), z(6), face_id(3), tex_id(11) 壓縮進一個 u32 (總計 32 bits)
pub const ATTRIBUTE_PACKED_DATA: MeshVertexAttribute =
    MeshVertexAttribute::new("Vertex_Packed_Data", 99887, VertexFormat::Uint32);

// ── Deterministic texture layer mapping ─────────────────────────────────────
// MUST match the order in texture_array.rs TEXTURES:
//   Layer 0: stone
//   Layer 1: dirt
//   Layer 2: grass_block_top
//   Layer 3: grass_block_side

fn get_texture_layer(block: BlockType, d: usize, normal: i32) -> u32 {
    match block {
        BlockType::Stone => 0,
        BlockType::Dirt  => 1,
        BlockType::Grass => match (d, normal) {
            (1,  1) => 2, // Y+ → top    (grass_block_top)
            (1, -1) => 1, // Y- → bottom (dirt)
            _       => 3, // X±/Z± → side (grass_block_side)
        },
        _ => 0,
    }
}

// ── Face mask ────────────────────────────────────────────────────────────────
//
// `tex_layer` is stored in the mask so that:
//   1. Greedy merge only fuses cells with the EXACT SAME material layer
//      (prevents top ↔ side confusion even if block type is the same)
//   2. The layer is computed once at mask-build time, no recomputation later

#[derive(Clone, Copy, PartialEq, Eq)]
struct FaceInfo {
    block:     BlockType,
    normal:    i32,
    tex_layer: u32,   // ← Key addition: makes merge direction-aware
    sky_light: u8,
}

// ── Mesh accumulator ─────────────────────────────────────────────────────────

type MeshData = (
    Vec<u32>,      // packed vertex data (x, y, z, face_id, tex_id)
    Vec<u32>,      // triangle indices
);

fn empty_mesh() -> MeshData { Default::default() }

/// Push a single greedy quad into the mesh accumulator.
///
/// Vertex order (quad corners in world space):
///   v1 = origin
///   v2 = origin + du  (width step)
///   v3 = origin + du + dv
///   v4 = origin + dv  (height step)
///
/// Winding proof (all axes, for POSITIVE normals with rev=false):
///   d=0: (v2-v1)×(v3-v1) = (0,w,0)×(0,w,h) = (+X,0,0)  ✓
///   d=1: (v2-v1)×(v3-v1) = (0,0,w)×(h,0,w) = (0,+Y,0)  ✓
///   d=2: (v2-v1)×(v3-v1) = (w,0,0)×(w,h,0) = (0,0,+Z)  ✓
/// → rev = (normal < 0)  for ALL axes (unified rule)
///
/// UV mapping per axis (ensures texture is right-side-up on all faces):
///   d=0  X-face  (u=Y vert, v=Z horiz):
///         v1(Y-low,Z-low)  v2(Y-high,Z-low)  v3(Y-high,Z-hi)  v4(Y-low,Z-hi)
///     UV: [0,w]            [0,0]             [h,0]            [h,w]
///         U → Z-horiz, V → Y-vert inverted (V=0=top, V=w=bottom)
///
///   d=1  Y-face  (u=Z, v=X) — top/bottom, no vertical concept needed:
///     UV: [0,0] [w,0] [w,h] [0,h]   (no change)
///
///   d=2  Z-face  (u=X horiz, v=Y vert):
///         v1(X-low,Y-low)  v2(X-hi,Y-low)  v3(X-hi,Y-hi)  v4(X-low,Y-hi)
///     UV: [0,h]            [w,h]            [w,0]          [0,0]
///         U → X-horiz, V → Y-vert inverted (V=0=top, V=h=bottom)
fn push_quad(
    bucket:    &mut MeshData,
    v1: [f32; 3], v2: [f32; 3], v3: [f32; 3], v4: [f32; 3],
    normal_vec: [f32; 3],
    _color:     [f32; 4],
    tex_layer:  u32,
    sky_light:  u8,
    _quad_w:    i32,
    _quad_h:    i32,
    rev:        bool,
    d:          usize,  // face axis
) {
    let base = bucket.0.len() as u32;

    // 計算 face_id (0: +X, 1: -X, 2: +Y, 3: -Y, 4: +Z, 5: -Z)
    // normal_vec 非 0 即 1 或 -1，藉由 d * 2 + (if pos then 0 else 1) 來映射
    let normal_val = normal_vec[d];
    let face_id = (d * 2) + if normal_val > 0.0 { 0 } else { 1 };
    
    // 將 4 個頂點分別打包
    for v in [v1, v2, v3, v4].iter() {
        let x = v[0] as u32;
        let y = v[1] as u32;
        let z = v[2] as u32;
        
        let packed: u32 = (x & 0x3F) 
                        | ((y & 0x3F) << 6) 
                        | ((z & 0x3F) << 12) 
                        | ((face_id as u32 & 0x07) << 18) 
                        | ((tex_layer & 0x7F) << 21) 
                        | (((sky_light as u32) & 0x0F) << 28);
                        
        bucket.0.push(packed);
    }

    // Triangle indices — CCW for rev=false, CW (reversed) for rev=true
    if rev {
        bucket.1.extend_from_slice(&[
            base, base + 2, base + 1,
            base, base + 3, base + 2,
        ]);
    } else {
        bucket.1.extend_from_slice(&[
            base, base + 1, base + 2,
            base, base + 2, base + 3,
        ]);
    }
}

// ── Finalise mesh ─────────────────────────────────────────────────────────────

fn finalize_mesh(data: MeshData, meshes: &mut Assets<Mesh>) -> Handle<Mesh> {
    let (packed, idx) = data;
    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
    mesh.insert_attribute(ATTRIBUTE_PACKED_DATA, packed);
    mesh.insert_indices(Indices::U32(idx));
    meshes.add(mesh)
}

// ── ECS system ────────────────────────────────────────────────────────────────

pub fn mesh_dirty_chunks(
    mut commands:   Commands,
    mut q_chunks:   Query<(Entity, &mut Chunk)>,
    mut meshes:     ResMut<Assets<Mesh>>,
    world_manager:  Res<WorldManager>,
    game_textures:  Option<Res<GameTextures>>,
) {
    let Some(gt) = game_textures else { return; };
    if !gt.ready { return; }

    let mut dirty_chunks = Vec::new();
    for (entity, chunk) in q_chunks.iter() {
        if chunk.is_dirty {
            dirty_chunks.push((entity, chunk.position));
        }
    }

    let mut meshes_to_apply = Vec::new();
    for (entity, chunk_pos) in dirty_chunks {
        let mut data = empty_mesh();
        generate_greedy_mesh(entity, chunk_pos, &world_manager, &q_chunks, &mut data);
        meshes_to_apply.push((entity, data));
    }

    for (entity, data) in meshes_to_apply {
        commands.entity(entity).despawn_descendants();

        if !data.0.is_empty() {
            let child = commands.spawn(MaterialMeshBundle {
                mesh:      finalize_mesh(data, &mut meshes),
                material:  gt.material.clone(),
                transform: Transform::default(),
                ..default()
            }).id();
            commands.entity(entity).add_child(child);
        }

        if let Ok((_, mut chunk)) = q_chunks.get_mut(entity) {
            chunk.is_dirty = false;
        }
    }
}

// ── Core greedy mesher ────────────────────────────────────────────────────────
//
// Axis layout
//   d  = axis being sliced (0=X, 1=Y, 2=Z)
//   u  = (d+1)%3  — "width"  direction within the face
//   v  = (d+2)%3  — "height" direction within the face
//
// Face detection rule (mask)
//   b0 = block at current voxel, b1 = block one step in +d
//   Face appears only when exactly ONE of the two is solid.
//   Two different solid types share a hidden internal boundary → no face.
//
// Merge rule (greedy)
//   Two adjacent mask cells merge into one quad only when their FaceInfo
//   is IDENTICAL — meaning same block, same outward normal, AND same tex_layer.
//   Storing tex_layer in FaceInfo makes it impossible to merge a grass top
//   (tex 2) with a grass side (tex 3) even if the struct fields happen to
//   collide in a future block type redesign.

fn generate_greedy_mesh(
    entity: Entity,
    chunk_pos: IVec3,
    world: &WorldManager,
    q_chunks: &Query<(Entity, &mut Chunk)>,
    out:   &mut MeshData,
) {
    // 🚀 優化 1：在進入 10 萬次迴圈前，先一次性獲取當前區塊的唯讀引用
    let current_chunk = &q_chunks.get(entity).unwrap().1;
    let current_entry = world.chunks.get(&chunk_pos).unwrap();

    // 強制將 CHUNK_SIZE 轉為 i32 以便與含有 -1 的 slice 安全迭代
    let chunk_size_i = CHUNK_SIZE as i32;

    for d in 0..3usize {
        // 快取局部性校準 (Cache Locality Calibration)
        // 記憶體步長: X=1, Y=32, Z=1024
        // u 軸映射到最內層迴圈 (i)，必須分配給步長最小的軸！
        let (u, v) = match d {
            0 => (1, 2), // d=X, u=Y(32), v=Z(1024) -> 最佳
            1 => (0, 2), // d=Y, u=X(1),  v=Z(1024) -> 交換 u,v 拯救快取！(原為 u=Z, v=X)
            _ => (0, 1), // d=Z, u=X(1),  v=Y(32)   -> 最佳
        };

        let mut q = [0i32; 3];
        q[d] = 1;

        let mask_len = (CHUNK_SIZE * CHUNK_SIZE) as usize;
        let mut mask = vec![None::<FaceInfo>; mask_len];

        // 🚀 優化 2：顯式轉換邊界型別，防止編譯器抗議 Range 混合
        for slice in -1..chunk_size_i {
            // ── Build mask ─────────────────────────────────────────────────
            let mut n = 0usize;
            for j in 0..chunk_size_i {
                for i in 0..chunk_size_i {
                    let mut x = [0i32; 3];
                    x[d] = slice;
                    x[u] = i;
                    x[v] = j;

                    // b0 = block at x[d]=slice (this voxel)
                    // b1 = block at x[d]=slice+1 (the voxel one step in +d)
                    let b0 = {
                        let lx = [x[0], x[1], x[2]];
                        if lx[0] >= 0 && lx[0] < chunk_size_i
                            && lx[1] >= 0 && lx[1] < chunk_size_i
                            && lx[2] >= 0 && lx[2] < chunk_size_i
                        {
                            // 🚀 優化 3：直接讀取變數，並將 i32 安全轉型為 usize 接入一維發電機
                            current_chunk.get_block(lx[0] as usize, lx[1] as usize, lx[2] as usize)
                        } else {
                            let gp = chunk_pos * chunk_size_i + IVec3::new(lx[0], lx[1], lx[2]);
                            world.get_block_global_mut(gp, q_chunks)
                        }
                    };

                    let b1 = {
                        let lx = [x[0] + q[0], x[1] + q[1], x[2] + q[2]];
                        if lx[0] >= 0 && lx[0] < chunk_size_i
                            && lx[1] >= 0 && lx[1] < chunk_size_i
                            && lx[2] >= 0 && lx[2] < chunk_size_i
                        {
                            // 🚀 優化 3：同上，本地安全高速讀取
                            current_chunk.get_block(lx[0] as usize, lx[1] as usize, lx[2] as usize)
                        } else {
                            let gp = chunk_pos * chunk_size_i + IVec3::new(lx[0], lx[1], lx[2]);
                            world.get_block_global_mut(gp, q_chunks)
                        }
                    };

                    // 實心→空氣: 繪製正向面 (normal +d)
                    // 空氣→實心: 繪製反向面 (normal -d)
                    mask[n] = match (b0.is_solid(), b1.is_solid()) {
                        (true, false) if slice >= 0 => {
                            let lx = x[0] + q[0];
                            let ly = x[1] + q[1];
                            let lz = x[2] + q[2];
                            let sl = if lx >= 0 && lx < chunk_size_i && ly >= 0 && ly < chunk_size_i && lz >= 0 && lz < chunk_size_i {
                                let idx = lx as usize + (ly as usize) * 32 + (lz as usize) * 1024;
                                current_entry.light_buffer.get_sky_light(idx)
                            } else {
                                let gp = chunk_pos * chunk_size_i + IVec3::new(lx, ly, lz);
                                world.get_light_global(gp)
                            };
                            Some(FaceInfo {
                                block:     b0,
                                normal:    1,
                                tex_layer: get_texture_layer(b0, d, 1),
                                sky_light: sl,
                            })
                        },
                        (false, true) if slice < chunk_size_i - 1 => {
                            let lx = x[0];
                            let ly = x[1];
                            let lz = x[2];
                            let sl = if lx >= 0 && lx < chunk_size_i && ly >= 0 && ly < chunk_size_i && lz >= 0 && lz < chunk_size_i {
                                let idx = lx as usize + (ly as usize) * 32 + (lz as usize) * 1024;
                                current_entry.light_buffer.get_sky_light(idx)
                            } else {
                                let gp = chunk_pos * chunk_size_i + IVec3::new(lx, ly, lz);
                                world.get_light_global(gp)
                            };
                            Some(FaceInfo {
                                block:     b1,
                                normal:    -1,
                                tex_layer: get_texture_layer(b1, d, -1),
                                sky_light: sl,
                            })
                        },
                        _ => None,
                    };
                    n += 1;
                }
            }

            // ── Greedy merge + emit quads ──────────────────────────────────
            let face_coord = slice + 1; // vertex plane is one step ahead

            let mut n = 0usize;
            for j in 0..CHUNK_SIZE {
                let mut i = 0i32;
                while i < CHUNK_SIZE {
                    if let Some(face) = mask[n] {
                        // Expand width (u direction)
                        let mut w = 1i32;
                        while i + w < CHUNK_SIZE && mask[n + w as usize] == Some(face) {
                            w += 1;
                        }

                        // Expand height (v direction)
                        let mut h = 1i32;
                        'outer: while j + h < CHUNK_SIZE {
                            for k in 0..w {
                                if mask[n + (h * CHUNK_SIZE + k) as usize] != Some(face) {
                                    break 'outer;
                                }
                            }
                            h += 1;
                        }

                        // Build quad geometry
                        let mut x    = [0i32; 3];
                        x[d] = face_coord;
                        x[u] = i;
                        x[v] = j;

                        let mut du = [0i32; 3]; du[u] = w;
                        let mut dv = [0i32; 3]; dv[v] = h;

                        let v1 = [ x[0]             as f32,  x[1]             as f32,  x[2]             as f32];
                        let v2 = [(x[0]+du[0])       as f32, (x[1]+du[1])       as f32, (x[2]+du[2])       as f32];
                        let v3 = [(x[0]+du[0]+dv[0]) as f32, (x[1]+du[1]+dv[1]) as f32, (x[2]+du[2]+dv[2]) as f32];
                        let v4 = [(x[0]+dv[0])       as f32, (x[1]+dv[1])       as f32, (x[2]+dv[2])       as f32];

                        let normal_vec = match d {
                            0 => [face.normal as f32, 0.0, 0.0],
                            1 => [0.0, face.normal as f32, 0.0],
                            _ => [0.0, 0.0, face.normal as f32],
                        };

                        // Winding: proved correct for all d when rev = (normal < 0)
                        let mut rev = face.normal < 0;
                        // 因為 d=1 時我們強制交換了 u(X) 與 v(Z)，外積方向翻轉，必須修正 Winding！
                        if d == 1 {
                            rev = !rev;
                        }

                        push_quad(
                            out,
                            v1, v2, v3, v4,
                            normal_vec,
                            [1.0, 1.0, 1.0, 1.0],
                            face.tex_layer,
                            face.sky_light,
                            w, h,
                            rev,
                            d,  // pass axis for correct UV layout
                        );

                        // Clear merged region from mask
                        for l in 0..h {
                            for k in 0..w {
                                mask[n + (l * CHUNK_SIZE + k) as usize] = None;
                            }
                        }

                        i += w;
                        n += w as usize;
                    } else {
                        i += 1;
                        n += 1;
                    }
                }
            }
        }
    }
}
