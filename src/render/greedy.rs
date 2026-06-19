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

pub const ATTRIBUTE_FLOW_VECTOR: MeshVertexAttribute =
    MeshVertexAttribute::new("Mesh_Flow_Vector", 987654, VertexFormat::Float32x2);

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
    y_offset_down: u8,
}

// ── Mesh accumulator ─────────────────────────────────────────────────────────

type MeshData = (
    Vec<u32>,      // packed vertex data (x, y, z, face_id, tex_id)
    Vec<u32>,      // triangle indices
    Vec<[f32; 2]>, // flow vector
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
    y_offset_down: u8,
    _quad_w:    i32,
    _quad_h:    i32,
    rev:        bool,
    d:          usize,  // face axis
) {
    let base = bucket.0.len() as u32;

    let normal_val = normal_vec[d];
    let face_id = (d * 2) + if normal_val > 0.0 { 0 } else { 1 };
    
    for v in [v1, v2, v3, v4].iter() {
        let x = v[0] as u32;
        let y = v[1] as u32;
        let z = v[2] as u32;

        let packed: u32 = (x & 0x3F) 
                        | ((y & 0x3F) << 6) 
                        | ((z & 0x3F) << 12) 
                        | ((face_id as u32 & 0x07) << 18) 
                        | ((tex_layer & 0x0F) << 21) 
                        | (((y_offset_down as u32) & 0x07) << 25)
                        | (((sky_light as u32) & 0x0F) << 28);
                        
        bucket.0.push(packed);
        bucket.2.push([0.0, 0.0]);
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
    let (packed, idx, flow) = data;
    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
    mesh.insert_attribute(ATTRIBUTE_PACKED_DATA, packed);
    mesh.insert_attribute(ATTRIBUTE_FLOW_VECTOR, flow);
    mesh.insert_indices(Indices::U32(idx));
    meshes.add(mesh)
}

// ── ECS system ────────────────────────────────────────────────────────────────

pub fn mesh_dirty_chunks(
    mut commands:   Commands,
    mut q_chunks:   Query<(Entity, &mut Chunk)>,
    mut meshes:     ResMut<Assets<Mesh>>,
    mut world_manager:  ResMut<WorldManager>,
    game_textures:  Option<Res<GameTextures>>,
) {
    let Some(gt) = game_textures else { return; };
    if !gt.ready { return; }

    // Sync pure-data layer dirty flags to ECS layer
    let drained_dirty: Vec<IVec3> = world_manager.dirty_chunks_for_meshing.drain().collect();
    for chunk_pos in drained_dirty {
        if let Some(entry) = world_manager.chunks.get(&chunk_pos) {
            if let Some(entity) = entry.entity {
                if let Ok((_, mut chunk)) = q_chunks.get_mut(entity) {
                    chunk.is_dirty = true;
                }
            }
        }
    }

    let mut dirty_chunks = Vec::new();
    for (entity, chunk) in q_chunks.iter() {
        if chunk.is_dirty {
            dirty_chunks.push((entity, chunk.position));
        }
    }

    let mut meshes_to_apply = Vec::new();
    for (entity, chunk_pos) in dirty_chunks {
        let mut solid_data = empty_mesh();
        let mut fluid_data = empty_mesh();
        generate_greedy_mesh(entity, chunk_pos, &world_manager, false, &mut solid_data);
        generate_greedy_mesh(entity, chunk_pos, &world_manager, true, &mut fluid_data);
        meshes_to_apply.push((entity, solid_data, fluid_data));
    }

    for (entity, solid_data, fluid_data) in meshes_to_apply {
        commands.entity(entity).despawn_descendants();

        if !solid_data.0.is_empty() {
            let child = commands.spawn(MaterialMeshBundle {
                mesh:      finalize_mesh(solid_data, &mut meshes),
                material:  gt.material.clone(),
                transform: Transform::default(),
                ..default()
            }).id();
            commands.entity(entity).add_child(child);
        }

        if !fluid_data.0.is_empty() {
            let child = commands.spawn(MaterialMeshBundle {
                mesh:      finalize_mesh(fluid_data, &mut meshes),
                material:  gt.fluid_material.clone(),
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
    _entity: Entity,
    chunk_pos: IVec3,
    world: &WorldManager,
    is_fluid: bool,
    out:   &mut MeshData,
) {
    if is_fluid {
        generate_fluid_mesh(chunk_pos, world, out);
        return;
    }

    let chunk_size_i = CHUNK_SIZE as i32;

    for d in 0..3usize {
        let (u, v) = match d {
            0 => (1, 2),
            1 => (0, 2),
            _ => (0, 1),
        };

        let mut q = [0i32; 3];
        q[d] = 1;

        let mask_len = (CHUNK_SIZE * CHUNK_SIZE) as usize;
        let mut mask = vec![None::<FaceInfo>; mask_len];

        for slice in -1..chunk_size_i {
            let mut n = 0usize;
            for j in 0..chunk_size_i {
                for i in 0..chunk_size_i {
                    let mut x = [0i32; 3];
                    x[d] = slice;
                    x[u] = i;
                    x[v] = j;

                    let gp0 = chunk_pos * chunk_size_i + IVec3::new(x[0], x[1], x[2]);
                    let gp1 = chunk_pos * chunk_size_i + IVec3::new(x[0] + q[0], x[1] + q[1], x[2] + q[2]);
                    
                    let b0 = world.get_block_global(gp0);
                    let b1 = world.get_block_global(gp1);

                    mask[n] = match (b0.is_solid(), b1.is_solid()) {
                        (true, false) if slice >= 0 => {
                            Some(FaceInfo {
                                block:     b0,
                                normal:    1,
                                tex_layer: get_texture_layer(b0, d, 1),
                                sky_light: world.get_light_global(gp1),
                                y_offset_down: 0,
                            })
                        },
                        (false, true) if slice < chunk_size_i - 1 => {
                            Some(FaceInfo {
                                block:     b1,
                                normal:    -1,
                                tex_layer: get_texture_layer(b1, d, -1),
                                sky_light: world.get_light_global(gp0),
                                y_offset_down: 0,
                            })
                        },
                        _ => None,
                    };
                    n += 1;
                }
            }

            let face_coord = slice + 1;
            let mut n = 0usize;
            for j in 0..CHUNK_SIZE {
                let mut i = 0i32;
                while i < CHUNK_SIZE {
                    if let Some(face) = mask[n] {
                        let mut w = 1i32;
                        while i + w < CHUNK_SIZE && mask[n + w as usize] == Some(face) {
                            w += 1;
                        }
                        let mut h = 1i32;
                        'outer: while j + h < CHUNK_SIZE {
                            for k in 0..w {
                                if mask[n + (h * CHUNK_SIZE + k) as usize] != Some(face) {
                                    break 'outer;
                                }
                            }
                            h += 1;
                        }

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

                        let mut rev = face.normal < 0;
                        if d == 1 { rev = !rev; }

                        push_quad(
                            out,
                            v1, v2, v3, v4,
                            normal_vec,
                            [1.0, 1.0, 1.0, 1.0],
                            face.tex_layer,
                            face.sky_light,
                            face.y_offset_down,
                            w, h,
                            rev,
                            d,
                        );

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

fn push_fluid_quad(
    out: &mut MeshData,
    x: i32, _y: i32, z: i32,
    face_id: u8,
    y_coords: [i32; 4],
    offsets: [u8; 4],
    sky_light: u8,
    flip_diagonal: bool,
    flow: [f32; 2],
) {
    let (v1, v2, v3, v4) = match face_id {
        0 => ( [x+1, y_coords[0], z+1], [x+1, y_coords[1], z], [x+1, y_coords[2], z], [x+1, y_coords[3], z+1] ), // +X
        1 => ( [x, y_coords[0], z], [x, y_coords[1], z+1], [x, y_coords[2], z+1], [x, y_coords[3], z] ),         // -X
        2 => ( [x, y_coords[0], z+1], [x+1, y_coords[1], z+1], [x+1, y_coords[2], z], [x, y_coords[3], z] ),     // +Y
        3 => ( [x, y_coords[0], z], [x+1, y_coords[1], z], [x+1, y_coords[2], z+1], [x, y_coords[3], z+1] ),     // -Y
        4 => ( [x+1, y_coords[0], z+1], [x, y_coords[1], z+1], [x, y_coords[2], z+1], [x+1, y_coords[3], z+1] ), // +Z
        5 => ( [x, y_coords[0], z], [x+1, y_coords[1], z], [x+1, y_coords[2], z], [x, y_coords[3], z] ),         // -Z
        _ => unreachable!(),
    };

    let tex_layer = 4u32; // water_still
    let base = out.0.len() as u32;

    for (i, v) in [v1, v2, v3, v4].iter().enumerate() {
        let vx = v[0] as u32;
        let vy = v[1] as u32;
        let vz = v[2] as u32;
        let y_offset_down = offsets[i] as u32;

        let packed: u32 = (vx & 0x3F) 
                        | ((vy & 0x3F) << 6) 
                        | ((vz & 0x3F) << 12) 
                        | ((face_id as u32 & 0x07) << 18) 
                        | ((tex_layer & 0x0F) << 21) 
                        | ((y_offset_down & 0x07) << 25)
                        | (((sky_light as u32) & 0x0F) << 28);
                        
        out.0.push(packed);
        out.2.push(flow);
    }

    let indices = match face_id {
        0 => [0, 1, 2, 0, 2, 3], // +X: 法線 +X (向外)
        1 => [0, 1, 2, 0, 2, 3], // -X: 法線 -X (向外)
        2 => if flip_diagonal { [1, 2, 3, 1, 3, 0] } else { [0, 1, 2, 0, 2, 3] }, // +Y: 法線 +Y (向外)
        3 => [0, 1, 2, 0, 2, 3], // -Y: 法線 -Y (向外)
        4 => [0, 2, 1, 0, 3, 2], // +Z: 翻轉後兩個索引，強迫法線 +Z (向外)
        5 => [0, 2, 1, 0, 3, 2], // -Z: 翻轉後兩個索引，強迫法線 -Z (向外)
        _ => unreachable!(),
    };

    out.1.extend_from_slice(&[
        base + indices[0], base + indices[1], base + indices[2],
        base + indices[3], base + indices[4], base + indices[5],
    ]);
}

fn generate_fluid_mesh(
    chunk_pos: IVec3,
    world: &WorldManager,
    out: &mut MeshData,
) {
    let chunk_size_i = CHUNK_SIZE as i32;
    let base_gp = chunk_pos * chunk_size_i;

    for y in 0..chunk_size_i {
        for z in 0..chunk_size_i {
            for x in 0..chunk_size_i {
                let gp = base_gp + IVec3::new(x, y, z);
                let b0 = world.get_block_global(gp);
                let f0 = world.get_fluid_global(gp).min(8);
                
                if !(b0 == BlockType::Air && f0 > 0) {
                    continue;
                }

                let get_corner_height = |cx: i32, cz: i32| -> u8 {
                    let mut max_f = 0;
                    for (dx, dz) in [(0,0), (-1,0), (0,-1), (-1,-1)] {
                        let ngp = base_gp + IVec3::new(cx + dx, y, cz + dz);
                        let f = world.get_fluid_global(ngp).min(8);
                        let b_above = world.get_block_global(ngp + IVec3::Y);
                        let f_above = world.get_fluid_global(ngp + IVec3::Y).min(8);
                        
                        if (b_above == BlockType::Air && f_above > 0) || f == 8 {
                            return 8;
                        }
                        if f > max_f {
                            max_f = f;
                        }
                    }
                    max_f
                };

                let nw_h = get_corner_height(x, z);
                let ne_h = get_corner_height(x + 1, z);
                let sw_h = get_corner_height(x, z + 1);
                let se_h = get_corner_height(x + 1, z + 1);

                // 流體最低渲染高度屏障 (Min Height Clamp)
                // 確保最外圍水位為 1 時，仍保留至少 1/8 的厚度，避免頂面與地面發生 Z-Fighting
                let mut nw_off = (8 - nw_h).min(7);
                let mut ne_off = (8 - ne_h).min(7);
                let mut sw_off = (8 - sw_h).min(7);
                let mut se_off = (8 - se_h).min(7);

                // 【下落全滿特權】：只有自身是滿水位，且上方有水灌入，且水平四周存在幾何缺口時，才判定為真正的瀑布柱！
                let is_waterfall_column = f0 == 8 
                    && world.get_fluid_global(gp + IVec3::Y) > 0 
                    && (world.get_fluid_global(gp + IVec3::X) == 0 
                        || world.get_fluid_global(gp - IVec3::X) == 0 
                        || world.get_fluid_global(gp + IVec3::Z) == 0 
                        || world.get_fluid_global(gp - IVec3::Z) == 0);

                if is_waterfall_column {
                    nw_off = 0;
                    ne_off = 0;
                    sw_off = 0;
                    se_off = 0;
                }

                let check_face = |ngp: IVec3, is_top_bottom: bool| -> bool {
                    let nb = world.get_block_global(ngp);
                    let nf = world.get_fluid_global(ngp).min(8);
                    if nb.is_solid() {
                        return false;
                    }
                    if is_top_bottom {
                        let n_is_water = nb == BlockType::Air && nf > 0;
                        !n_is_water
                    } else {
                        nf == 0
                    }
                };

                let h_nw = nw_off as i32;
                let h_se = se_off as i32;
                let h_ne = ne_off as i32;
                let h_sw = sw_off as i32;
                
                let diff_a = (h_nw - h_se).abs();
                let diff_b = (h_ne - h_sw).abs();
                let flip_diagonal = diff_a > diff_b;

                let nf_px = world.get_fluid_global(gp + IVec3::X).min(8);
                let nf_nx = world.get_fluid_global(gp - IVec3::X).min(8);
                let nf_pz = world.get_fluid_global(gp + IVec3::Z).min(8);
                let nf_nz = world.get_fluid_global(gp - IVec3::Z).min(8);

                let flow_x = (nf_px as f32) - (nf_nx as f32);
                let flow_z = (nf_pz as f32) - (nf_nz as f32);
                let mut flow_vec = bevy::math::Vec2::new(flow_x, flow_z);
                
                if flow_vec.length_squared() > 0.001 {
                    flow_vec = flow_vec.normalize();
                } else {
                    flow_vec = bevy::math::Vec2::ZERO;
                }
                let top_flow = [flow_vec.x, flow_vec.y];
                // For side faces, we use positive V = down.
                // world_uv for side faces uses -y for V.
                // If we want it to flow down (negative Y direction),
                // V should decrease? Wait.
                // world_uv = vec2(z, -y).
                // We want to sample higher Y (smaller -y).
                // So we want V to DECREASE over time.
                // animated_uv = world_uv + time * flow.
                // So flow for V should be NEGATIVE!
                let side_flow = [0.0, -1.0];

                let get_side_anchors = |nf: u8| -> (i32, u8) {
                    if nf == 0 {
                        (y, 0)
                    } else {
                        (y + 1, 8 - nf)
                    }
                };

                let (y_bot_px, off_px) = get_side_anchors(nf_px);
                let (y_bot_nx, off_nx) = get_side_anchors(nf_nx);
                let (y_bot_pz, off_pz) = get_side_anchors(nf_pz);
                let (y_bot_nz, off_nz) = get_side_anchors(nf_nz);

                if check_face(gp + IVec3::X, false) {
                    push_fluid_quad(out, x, y, z, 0, [y_bot_px, y_bot_px, y+1, y+1], [off_px, off_px, ne_off, se_off], world.get_light_global(gp + IVec3::X), false, side_flow);
                }
                if check_face(gp - IVec3::X, false) {
                    push_fluid_quad(out, x, y, z, 1, [y_bot_nx, y_bot_nx, y+1, y+1], [off_nx, off_nx, sw_off, nw_off], world.get_light_global(gp - IVec3::X), false, side_flow);
                }
                if check_face(gp + IVec3::Y, true) {
                    push_fluid_quad(out, x, y, z, 2, [y+1, y+1, y+1, y+1], [sw_off, se_off, ne_off, nw_off], world.get_light_global(gp + IVec3::Y), flip_diagonal, top_flow);
                }
                if check_face(gp - IVec3::Y, true) {
                    push_fluid_quad(out, x, y, z, 3, [y+1, y+1, y+1, y+1], [7, 7, 7, 7], world.get_light_global(gp - IVec3::Y), false, side_flow);
                }
                if check_face(gp + IVec3::Z, false) {
                    push_fluid_quad(out, x, y, z, 4, [y_bot_pz, y_bot_pz, y+1, y+1], [off_pz, off_pz, sw_off, se_off], world.get_light_global(gp + IVec3::Z), false, side_flow);
                }
                if check_face(gp - IVec3::Z, false) {
                    push_fluid_quad(out, x, y, z, 5, [y_bot_nz, y_bot_nz, y+1, y+1], [off_nz, off_nz, ne_off, nw_off], world.get_light_global(gp - IVec3::Z), false, side_flow);
                }
            }
        }
    }
}
