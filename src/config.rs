use bevy::prelude::*;

pub const MAX_FLUID_LEVEL: u8 = 8;
pub const MAX_LIGHT_LEVEL: u8 = 15;

#[derive(Resource, Clone)]
pub struct EngineConfig {
    pub fluid_scroll_speed: f32, // GPU 材質流速
    pub fluid_tick_speed: f32,   // CPU BFS 蔓延心跳 (秒)
    pub min_ambient_light: f32,  // 基礎環境光 (0.02)
    pub render_distance: u32,    // 渲染半徑 (8)
    pub max_mesh_uploads_per_frame: usize, // 每影格最大 GPU 上傳區塊數 (推薦預設 2 或 3)
    pub smooth_lighting: bool,   // 平滑光照開關
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
        }
    }
}
