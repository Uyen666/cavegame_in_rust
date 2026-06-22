use bevy::prelude::*;

pub const MAX_FLUID_LEVEL: u8 = 8;
#[allow(dead_code)]
pub const MAX_LIGHT_LEVEL: u8 = 15;

#[derive(Clone)]
pub struct PhysicsConfig {
    pub gravity: f32,                  // 預設 25.0, 陸地重力
    pub land_jump_impulse: f32,        // 預設 8.5, 陸地跳躍衝量
    pub water_gravity_multiplier: f32, // 預設 0.25, 水中重力削弱比
    pub water_damping: f32,            // 預設 4.0, 水體阻尼係數
    pub water_buoyancy: f32,           // 預設 15.0, 水下空白鍵上升力
}

impl Default for PhysicsConfig {
    fn default() -> Self {
        Self {
            gravity: 25.0,
            land_jump_impulse: 8.5,
            water_gravity_multiplier: 0.25,
            water_damping: 6.0,
            water_buoyancy: 15.0,
        }
    }
}

#[derive(Resource, Clone)]
pub struct EngineConfig {
    pub fluid_scroll_speed: f32, // GPU 材質流速
    pub fluid_tick_speed: f32,   // CPU BFS 蔓延心跳 (秒)
    pub min_ambient_light: f32,  // 基礎環境光 (0.02)
    pub render_distance: u32,    // 渲染半徑 (8)
    pub max_mesh_uploads_per_frame: usize, // 每影格最大 GPU 上傳區塊數 (推薦預設 2 或 3)
    pub smooth_lighting: bool,   // 平滑光照開關
    pub physics: PhysicsConfig,  // 動態物理參數註冊表
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            fluid_scroll_speed: 0.12,
            fluid_tick_speed: 0.1,
            min_ambient_light: 0.02,
            render_distance: 8,
            max_mesh_uploads_per_frame: 3,
            smooth_lighting: true,
            physics: PhysicsConfig::default(),
        }
    }
}
