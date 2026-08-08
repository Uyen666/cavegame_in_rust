pub mod components;
pub mod systems;
pub mod swept;

use bevy::prelude::*;

pub struct PhysicsPlugin;

impl Plugin for PhysicsPlugin {
    fn build(&self, _app: &mut App) {
        // Systems are registered and chained explicitly in main.rs to ensure ordering
        // with player_move and other input systems.
    }
}
