use bevy::prelude::{Vec3, IVec3};
use crate::world::voxel::BlockType;
use crate::world::WorldManager;

pub const CHUNK_SIZE: i32 = 32;
pub const WORLD_CHUNKS_Y: i32 = 8;
pub const WORLD_MAX_Y: i32 = CHUNK_SIZE * WORLD_CHUNKS_Y;

#[inline(always)]
pub fn voxel_pos_to_index(x: usize, y: usize, z: usize) -> usize {
    x + (y * (CHUNK_SIZE as usize)) + (z * (CHUNK_SIZE as usize) * (CHUNK_SIZE as usize))
}

#[inline(always)]
pub fn in_bounds(x: usize, y: usize, z: usize) -> bool {
    x < (CHUNK_SIZE as usize) && y < (CHUNK_SIZE as usize) && z < (CHUNK_SIZE as usize)
}

pub struct Aabb {
    pub min: Vec3,
    pub max: Vec3,
}

impl Aabb {
    pub fn new(min: Vec3, max: Vec3) -> Self {
        Self { min, max }
    }

    pub fn intersects(&self, other: &Self) -> bool {
        self.min.x < other.max.x && self.max.x > other.min.x &&
        self.min.y < other.max.y && self.max.y > other.min.y &&
        self.min.z < other.max.z && self.max.z > other.min.z
    }
}

/// 3D DDA 射線檢測命中結果
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RaycastHit {
    pub block_pos: IVec3,
    pub normal: IVec3,
    pub distance: f32,
    pub adjacent_pos: IVec3,
    pub block_type: BlockType,
}

/// 射線與軸向包圍盒 (AABB) 的精準求交演算法 (Slab Method)
pub fn ray_box_intersection(start: Vec3, dir: Vec3, box_min: Vec3, box_max: Vec3) -> Option<(f32, IVec3)> {
    let mut t_min = 0.0f32;
    let mut t_max = f32::INFINITY;
    let mut hit_normal = IVec3::ZERO;

    for i in 0..3 {
        let (origin_coord, dir_coord, min_coord, max_coord, axis_norm) = match i {
            0 => (start.x, dir.x, box_min.x, box_max.x, IVec3::X),
            1 => (start.y, dir.y, box_min.y, box_max.y, IVec3::Y),
            _ => (start.z, dir.z, box_min.z, box_max.z, IVec3::Z),
        };

        if dir_coord.abs() < 1e-8 {
            if origin_coord < min_coord || origin_coord > max_coord {
                return None;
            }
        } else {
            let inv_d = 1.0 / dir_coord;
            let mut t1 = (min_coord - origin_coord) * inv_d;
            let mut t2 = (max_coord - origin_coord) * inv_d;
            let mut norm = -axis_norm;

            if t1 > t2 {
                std::mem::swap(&mut t1, &mut t2);
                norm = axis_norm;
            }

            if t1 > t_min {
                t_min = t1;
                hit_normal = norm;
            }
            t_max = t_max.min(t2);

            if t_min > t_max {
                return None;
            }
        }
    }

    if t_max < 0.0 {
        return None;
    }

    Some((t_min.max(0.0), hit_normal))
}

/// 工業級 Amanatides & Woo (1987) 3D 快速體素遍歷演算法 (Fast Voxel Traversal / 3D DDA)
///
/// 徹底取代舊有低效且具穿透誤差的固定步長 (0.05m) 射線步進，
/// 具備嚴格的整數格邊界跳轉、微型方塊 (火把等) AABB 精準求交，並直接產出擊中表面法線與相鄰放置點。
pub fn raycast_voxel(
    world: &WorldManager,
    start: Vec3,
    dir: Vec3,
    max_dist: f32,
) -> Option<RaycastHit> {
    if dir.length_squared() < 0.0001 || max_dist <= 0.0 {
        return None;
    }
    let dir = dir.normalize();

    let mut current = IVec3::new(
        start.x.floor() as i32,
        start.y.floor() as i32,
        start.z.floor() as i32,
    );

    let step_x = if dir.x > 0.0 { 1 } else if dir.x < 0.0 { -1 } else { 0 };
    let step_y = if dir.y > 0.0 { 1 } else if dir.y < 0.0 { -1 } else { 0 };
    let step_z = if dir.z > 0.0 { 1 } else if dir.z < 0.0 { -1 } else { 0 };

    let delta_x = if dir.x != 0.0 { (1.0 / dir.x).abs() } else { f32::INFINITY };
    let delta_y = if dir.y != 0.0 { (1.0 / dir.y).abs() } else { f32::INFINITY };
    let delta_z = if dir.z != 0.0 { (1.0 / dir.z).abs() } else { f32::INFINITY };

    let mut max_x = if step_x > 0 {
        ((current.x + 1) as f32 - start.x) * delta_x
    } else if step_x < 0 {
        (start.x - current.x as f32) * delta_x
    } else {
        f32::INFINITY
    };

    let mut max_y = if step_y > 0 {
        ((current.y + 1) as f32 - start.y) * delta_y
    } else if step_y < 0 {
        (start.y - current.y as f32) * delta_y
    } else {
        f32::INFINITY
    };

    let mut max_z = if step_z > 0 {
        ((current.z + 1) as f32 - start.z) * delta_z
    } else if step_z < 0 {
        (start.z - current.z as f32) * delta_z
    } else {
        f32::INFINITY
    };

    let mut normal = IVec3::ZERO;
    let mut current_dist = 0.0;

    while current_dist <= max_dist {
        if current.y >= 0 && current.y < WORLD_MAX_Y {
            let block = world.get_block_global(current);
            if block.is_solid() || block.is_torch() {
                let (aabb_min, aabb_max) = block.get_aabb_offsets();
                let b_min = current.as_vec3() + Vec3::from_array(aabb_min);
                let b_max = current.as_vec3() + Vec3::from_array(aabb_max);

                if let Some((t_hit, hit_norm)) = ray_box_intersection(start, dir, b_min, b_max) {
                    if t_hit <= max_dist {
                        let final_normal = if normal == IVec3::ZERO { hit_norm } else { normal };
                        return Some(RaycastHit {
                            block_pos: current,
                            normal: final_normal,
                            distance: t_hit,
                            adjacent_pos: current + final_normal,
                            block_type: block,
                        });
                    }
                }
            }
        }

        // 3D DDA 軸向推進步進
        if max_x < max_y {
            if max_x < max_z {
                current_dist = max_x;
                max_x += delta_x;
                current.x += step_x;
                normal = IVec3::new(-step_x, 0, 0);
            } else {
                current_dist = max_z;
                max_z += delta_z;
                current.z += step_z;
                normal = IVec3::new(0, 0, -step_z);
            }
        } else {
            if max_y < max_z {
                current_dist = max_y;
                max_y += delta_y;
                current.y += step_y;
                normal = IVec3::new(0, -step_y, 0);
            } else {
                current_dist = max_z;
                max_z += delta_z;
                current.z += step_z;
                normal = IVec3::new(0, 0, -step_z);
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dda_raycast_orthogonal_hit() {
        let mut world = WorldManager::default();
        let ground_pos = IVec3::new(0, 5, 0);
        // Place stone at (0, 5, 0)
        let chunk_pos = IVec3::new(0, 0, 0);
        world.chunks.insert(chunk_pos, crate::world::ChunkEntry {
            buffer: std::sync::Arc::new(crate::world::generator::ChunkBuffer { blocks: [BlockType::Air; 32768] }),
            light_buffer: std::sync::Arc::new(crate::world::ChunkLightBuffer::default()),
            fluid_buffer: None,
            entity: None,
            is_modified: false,
            is_lighting_ready: true,
        });
        let ecs_world = bevy::prelude::World::new();
        let mut queue = bevy::ecs::world::CommandQueue::default();
        let mut commands = bevy::prelude::Commands::new(&mut queue, &ecs_world);
        world.set_block_global(ground_pos, BlockType::Stone, &mut commands);

        // Ray from (0.5, 7.0, 0.5) looking straight down (0, -1, 0)
        let start = Vec3::new(0.5, 7.0, 0.5);
        let dir = Vec3::new(0.0, -1.0, 0.0);
        let hit = raycast_voxel(&world, start, dir, 5.0).expect("Should hit stone");

        assert_eq!(hit.block_pos, ground_pos);
        assert_eq!(hit.normal, IVec3::Y, "Hitting top of stone should return +Y normal");
        assert_eq!(hit.adjacent_pos, IVec3::new(0, 6, 0), "Adjacent placement pos should be (0, 6, 0)");
        assert_eq!(hit.block_type, BlockType::Stone);
        assert!((hit.distance - 1.0).abs() < 1e-4, "Distance from 7.0 to 6.0 should be 1.0");
    }

    #[test]
    fn test_dda_raycast_sub_block_torch_aabb() {
        let mut world = WorldManager::default();
        let chunk_pos = IVec3::new(0, 0, 0);
        world.chunks.insert(chunk_pos, crate::world::ChunkEntry {
            buffer: std::sync::Arc::new(crate::world::generator::ChunkBuffer { blocks: [BlockType::Air; 32768] }),
            light_buffer: std::sync::Arc::new(crate::world::ChunkLightBuffer::default()),
            fluid_buffer: None,
            entity: None,
            is_modified: false,
            is_lighting_ready: true,
        });
        let ecs_world = bevy::prelude::World::new();
        let mut queue = bevy::ecs::world::CommandQueue::default();
        let mut commands = bevy::prelude::Commands::new(&mut queue, &ecs_world);

        let torch_pos = IVec3::new(2, 5, 0);
        let stone_pos = IVec3::new(4, 5, 0);
        world.set_block_global(torch_pos, BlockType::Torch, &mut commands);
        world.set_block_global(stone_pos, BlockType::Stone, &mut commands);

        // 1. 射線高度 y = 5.3 (穿過火把 AABB: y ∈ [5.0, 5.7])
        // 應精準命中火把
        let ray1_start = Vec3::new(0.5, 5.3, 0.5);
        let ray_dir = Vec3::new(1.0, 0.0, 0.0);
        let hit1 = raycast_voxel(&world, ray1_start, ray_dir, 10.0).expect("Should hit torch");
        assert_eq!(hit1.block_pos, torch_pos, "Ray at y=5.3 must hit torch");
        assert_eq!(hit1.block_type, BlockType::Torch);
        assert_eq!(hit1.normal, IVec3::NEG_X);

        // 2. 射線高度 y = 5.8 (高於火把高度 0.7，即 y > 5.7，火把上方的空曠處)
        // 射線必須精準穿透火把所處體素的空氣部分，命中後方的石頭！
        let ray2_start = Vec3::new(0.5, 5.8, 0.5);
        let hit2 = raycast_voxel(&world, ray2_start, ray_dir, 10.0).expect("Should hit stone behind torch");
        assert_eq!(hit2.block_pos, stone_pos, "Ray at y=5.8 must pass above torch and hit stone");
        assert_eq!(hit2.block_type, BlockType::Stone);
        assert_eq!(hit2.normal, IVec3::NEG_X);
    }
}
