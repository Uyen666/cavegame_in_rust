pub mod greedy;
pub mod textures;
pub mod texture_array;
pub mod material;
pub mod particles;

use bevy::prelude::*;
use textures::TexturePlugin;
use material::VoxelMaterial;

pub struct RenderPlugin;

impl Plugin for RenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(TexturePlugin)
           .add_plugins(particles::ParticlesPlugin)
           .add_plugins(MaterialPlugin::<VoxelMaterial> {
               prepass_enabled: false,
               ..default()
           })
           .add_systems(Update, (
               greedy::mesh_dirty_chunks,
               greedy::poll_mesh_tasks
           ));
    }
}


