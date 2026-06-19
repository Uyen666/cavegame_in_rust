use bevy::prelude::*;

#[derive(Resource)]
pub struct EngineConfig {
    pub fluid_scroll_speed: f32,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            fluid_scroll_speed: 0.12,
        }
    }
}
