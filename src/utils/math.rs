pub const CHUNK_SIZE: i32 = 32;
pub const WORLD_CHUNKS_Y: i32 = 8;
pub const WORLD_MAX_Y: i32 = CHUNK_SIZE * WORLD_CHUNKS_Y;

#[inline]
pub fn voxel_pos_to_index(x: usize, y: usize, z: usize) -> usize {
    x + (y * (CHUNK_SIZE as usize)) + (z * (CHUNK_SIZE as usize) * (CHUNK_SIZE as usize))
}

#[inline]
pub fn in_bounds(x: usize, y: usize, z: usize) -> bool {
    x < (CHUNK_SIZE as usize) && y < (CHUNK_SIZE as usize) && z < (CHUNK_SIZE as usize)
}

use bevy::prelude::Vec3;

pub struct Aabb {
    pub min: Vec3,
    pub max: Vec3,
}

impl Aabb {
    pub fn new(min: Vec3, max: Vec3) -> Self {
        Self { min, max }
    }

    #[allow(dead_code)]
    pub fn expand_by_velocity(&self, velocity: Vec3) -> Self {
        let mut min = self.min;
        let mut max = self.max;
        if velocity.x > 0.0 { max.x += velocity.x; } else { min.x += velocity.x; }
        if velocity.y > 0.0 { max.y += velocity.y; } else { min.y += velocity.y; }
        if velocity.z > 0.0 { max.z += velocity.z; } else { min.z += velocity.z; }
        Self { min, max }
    }

    pub fn intersects(&self, other: &Self) -> bool {
        self.min.x < other.max.x && self.max.x > other.min.x &&
        self.min.y < other.max.y && self.max.y > other.min.y &&
        self.min.z < other.max.z && self.max.z > other.min.z
    }
}
