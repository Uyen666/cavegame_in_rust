pub mod greedy;
pub mod textures;
pub mod texture_array;
pub mod material;

use bevy::prelude::*;
use textures::TexturePlugin;
use material::VoxelMaterial;

pub struct RenderPlugin;

impl Plugin for RenderPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(TexturePlugin)
           .add_plugins(MaterialPlugin::<VoxelMaterial> {
               prepass_enabled: false,
               ..default()
           })
           .add_systems(Update, (
               greedy::mesh_dirty_chunks,
               greedy::poll_mesh_tasks
           ))
           .add_systems(Update, update_dynamic_environment);
    }
}

fn update_dynamic_environment(
    time: Res<Time>,
    config: Res<crate::config::EngineConfig>,
    mut clear_color: ResMut<ClearColor>,
    mut materials: ResMut<Assets<VoxelMaterial>>,
    game_textures: Res<crate::render::textures::GameTextures>,
    world_manager: Res<crate::world::WorldManager>,
    q_player: Query<&Transform, With<crate::player::Player>>,
) {
    if !game_textures.ready { return; }

    let Ok(player_transform) = q_player.get_single() else { return; };
    let pos = player_transform.translation;
    let eye_pos = IVec3::new(pos.x.floor() as i32, (pos.y + 1.0).floor() as i32, pos.z.floor() as i32);
    
    let eye_light = world_manager.get_light_global(eye_pos);
    let light_factor = eye_light as f32 / crate::config::MAX_LIGHT_LEVEL as f32;
    
    let base_ambient = config.min_ambient_light;
    let tr = base_ambient + (0.5 - base_ambient) * light_factor;
    let tg = base_ambient + (0.8 - base_ambient) * light_factor;
    let tb = base_ambient + (1.0 - base_ambient) * light_factor;
    let target_color = bevy::color::LinearRgba::new(tr, tg, tb, 1.0);
    
    let current_color = bevy::color::LinearRgba::from(clear_color.0);
    let factor = (time.delta_seconds() * 5.0).clamp(0.0, 1.0);
    
    let nr = current_color.red + (target_color.red - current_color.red) * factor;
    let ng = current_color.green + (target_color.green - current_color.green) * factor;
    let nb = current_color.blue + (target_color.blue - current_color.blue) * factor;
    let next_color = bevy::color::LinearRgba::new(nr, ng, nb, 1.0);
    if clear_color.0 != Color::from(next_color) {
        clear_color.0 = Color::from(next_color);
    }

    let max_distance = config.render_distance as f32 * 32.0;
    let target_fog_end = max_distance - 8.0;      // 留 8 格緩衝遮蔽區塊突兀生成
    let target_fog_start = max_distance * 0.75;    // 75% 遠處開始起霧

    let mut update_solid = false;
    if let Some(mat) = materials.get(&game_textures.material) {
        if mat.env.fog_color != next_color 
            || mat.env.camera_pos != pos 
            || mat.env.fluid_scroll_speed != config.fluid_scroll_speed
            || mat.env.fog_start != target_fog_start
            || mat.env.fog_end != target_fog_end 
        {
            update_solid = true;
        }
    }
    if update_solid {
        if let Some(mat) = materials.get_mut(&game_textures.material) {
            mat.env.fog_color = next_color;
            mat.env.camera_pos = pos;
            mat.env.fluid_scroll_speed = config.fluid_scroll_speed;
            mat.env.fog_start = target_fog_start;
            mat.env.fog_end = target_fog_end;
        }
    }

    let mut update_fluid = false;
    if let Some(mat) = materials.get(&game_textures.fluid_material) {
        if mat.env.fog_color != next_color 
            || mat.env.camera_pos != pos 
            || mat.env.fluid_scroll_speed != config.fluid_scroll_speed
            || mat.env.fog_start != target_fog_start
            || mat.env.fog_end != target_fog_end 
        {
            update_fluid = true;
        }
    }
    if update_fluid {
        if let Some(mat) = materials.get_mut(&game_textures.fluid_material) {
            mat.env.fog_color = next_color;
            mat.env.camera_pos = pos;
            mat.env.fluid_scroll_speed = config.fluid_scroll_speed;
            mat.env.fog_start = target_fog_start;
            mat.env.fog_end = target_fog_end;
        }
    }
}
