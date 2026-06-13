pub mod greedy;
pub mod textures;

use bevy::prelude::*;
use textures::TexturePlugin;

pub struct RenderPlugin;

impl Plugin for RenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(TexturePlugin)
           .add_systems(Update, greedy::mesh_dirty_chunks);
    }
}
