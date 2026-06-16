use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology, MeshVertexAttribute};
use bevy::render::render_asset::RenderAssetUsages;
use bevy::render::render_resource::VertexFormat;
use crate::world::{Chunk, BlockType, WorldManager};
use crate::utils::math::CHUNK_SIZE;
use super::textures::GameTextures;

pub const ATTRIBUTE_TEXTURE_INDEX: MeshVertexAttribute = MeshVertexAttribute::new(
    "Vertex_TextureIndex",
    99,
    VertexFormat::Uint32,
);

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
}

// ── Mesh accumulator ─────────────────────────────────────────────────────────

type MeshData = (
    Vec<[f32; 3]>, // positions
    Vec<[f32; 3]>, // normals
    Vec<[f32; 4]>, // colors
    Vec<[f32; 2]>, // UVs
    Vec<u32>,      // texture layer per vertex
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
    color:      [f32; 4],
    tex_layer:  u32,
    quad_w:     i32,
    quad_h:     i32,
    rev:        bool,
    d:          usize,  // face axis → drives UV layout
) {
    let base = bucket.0.len() as u32;

    bucket.0.extend_from_slice(&[v1, v2, v3, v4]);
    bucket.1.extend_from_slice(&[normal_vec; 4]);
    bucket.2.extend_from_slice(&[color; 4]);
    bucket.4.extend_from_slice(&[tex_layer; 4]);

    let (w, h) = (quad_w as f32, quad_h as f32);

    // Per-axis UV layout (see doc-comment above for derivation)
    let uvs: [[f32; 2]; 4] = match d {
        // X-face: U→Z(horiz=h), V→Y(vert=w) inverted
        0 => [[0.0, w  ], [0.0, 0.0], [h,   0.0], [h,   w  ]],
        // Y-face: standard, no vertical correction needed
        1 => [[0.0, 0.0], [w,   0.0], [w,   h  ], [0.0, h  ]],
        // Z-face: U→X(horiz=w), V→Y(vert=h) inverted
        _ => [[0.0, h  ], [w,   h  ], [w,   0.0], [0.0, 0.0]],
    };
    bucket.3.extend_from_slice(&uvs);

    // Triangle indices — CCW for rev=false, CW (reversed) for rev=true
    if rev {
        bucket.5.extend_from_slice(&[
            base, base + 2, base + 1,
            base, base + 3, base + 2,
        ]);
    } else {
        bucket.5.extend_from_slice(&[
            base, base + 1, base + 2,
            base, base + 2, base + 3,
        ]);
    }
}

// ── Finalise mesh ─────────────────────────────────────────────────────────────

fn finalize_mesh(data: MeshData, meshes: &mut Assets<Mesh>) -> Handle<Mesh> {
    let (pos, nrm, col, uv, tex_idx, idx) = data;
    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION,    pos);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL,      nrm);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR,       col);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0,        uv);
    mesh.insert_attribute(ATTRIBUTE_TEXTURE_INDEX,     tex_idx);
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
    for d in 0..3usize {
        let u = (d + 1) % 3;
        let v = (d + 2) % 3;

        let mut q = [0i32; 3];
        q[d] = 1;

        let mask_len = (CHUNK_SIZE * CHUNK_SIZE) as usize;
        let mut mask = vec![None::<FaceInfo>; mask_len];

        for slice in -1..CHUNK_SIZE {
            // ── Build mask ─────────────────────────────────────────────────
            let mut n = 0usize;
            for j in 0..CHUNK_SIZE {
                for i in 0..CHUNK_SIZE {
                    let mut x = [0i32; 3];
                    x[d] = slice;
                    x[u] = i;
                    x[v] = j;

                    // b0 = block at x[d]=slice (this voxel)
                    // b1 = block at x[d]=slice+1 (the voxel one step in +d)
                    let b0 = {
                        let lx = [x[0], x[1], x[2]];
                        if lx[0] >= 0 && lx[0] < CHUNK_SIZE
                            && lx[1] >= 0 && lx[1] < CHUNK_SIZE
                            && lx[2] >= 0 && lx[2] < CHUNK_SIZE
                        {
                            q_chunks.get(entity).unwrap().1.get_block(lx[0], lx[1], lx[2])
                        } else {
                            let gp = chunk_pos * CHUNK_SIZE + IVec3::new(lx[0], lx[1], lx[2]);
                            world.get_block_global_mut(gp, q_chunks)
                        }
                    };

                    let b1 = {
                        let lx = [x[0] + q[0], x[1] + q[1], x[2] + q[2]];
                        if lx[0] >= 0 && lx[0] < CHUNK_SIZE
                            && lx[1] >= 0 && lx[1] < CHUNK_SIZE
                            && lx[2] >= 0 && lx[2] < CHUNK_SIZE
                        {
                            q_chunks.get(entity).unwrap().1.get_block(lx[0], lx[1], lx[2])
                        } else {
                            let gp = chunk_pos * CHUNK_SIZE + IVec3::new(lx[0], lx[1], lx[2]);
                            world.get_block_global_mut(gp, q_chunks)
                        }
                    };

                    // 實心→空氣: 繪製正向面 (normal +d)
                    // 空氣→實心: 繪製反向面 (normal -d)
                    // 【修正】：嚴格實施「幾何擁有權短路」
                    // 只有當實心方塊真正屬於「當前區塊」的合法索引範圍 (0~31) 時，才允許產生網格面。
                    // 若實心方塊在隔壁區塊，則由隔壁區塊自己負責繪製，防止跨區界產生雙重疊加的幽靈面！
                    mask[n] = match (b0.is_solid(), b1.is_solid()) {
                        (true, false) if slice >= 0 => Some(FaceInfo {
                            block:     b0,
                            normal:    1,
                            tex_layer: get_texture_layer(b0, d, 1),
                        }),
                        (false, true) if slice < CHUNK_SIZE - 1 => Some(FaceInfo {
                            block:     b1,
                            normal:    -1,
                            tex_layer: get_texture_layer(b1, d, -1),
                        }),
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
                        let rev = face.normal < 0;

                        push_quad(
                            out,
                            v1, v2, v3, v4,
                            normal_vec,
                            [1.0, 1.0, 1.0, 1.0],
                            face.tex_layer,
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
