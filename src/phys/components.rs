use bevy::prelude::*;
use crate::utils::math::Aabb;

#[derive(Component, Default)]
pub struct RigidBody {
    pub gravity_scale: f32,
    pub safewalk: bool, // 🚀 潛行邊緣防跌落安全鎖
    pub is_kinematic: bool, // 旁觀者模式繞過標記
    pub is_colliding_horizontally: bool,
}

#[derive(Component)]
pub struct AabbCollider {
    pub min_offset: Vec3,
    pub max_offset: Vec3,
}

impl Default for AabbCollider {
    fn default() -> Self {
        Self::from_dimensions(0.6, 1.8) // 預設跟 Player 一樣
    }
}

impl AabbCollider {
    #[allow(dead_code)]
    pub fn new(min_offset: Vec3, max_offset: Vec3) -> Self {
        Self { min_offset, max_offset }
    }
    
    pub fn from_dimensions(width: f32, height: f32) -> Self {
        let half_width = width / 2.0;
        Self {
            min_offset: Vec3::new(-half_width, 0.0, -half_width),
            max_offset: Vec3::new(half_width, height, half_width),
        }
    }

    pub fn get_global_aabb(&self, pos: Vec3) -> Aabb {
        Aabb::new(pos + self.min_offset, pos + self.max_offset)
    }
}

#[derive(Component, Deref, DerefMut, Default)]
pub struct Velocity(pub Vec3);

#[derive(Component, Default)]
pub struct GroundSensor {
    pub on_ground: bool,
    pub was_on_ground: bool,
}

#[derive(Component, Default)]
pub struct FluidSensor {
    pub in_fluid: bool,
    pub head_in_fluid: bool,
}
