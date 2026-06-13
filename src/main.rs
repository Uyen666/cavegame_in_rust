use bevy::prelude::*;

mod world;
mod render;
mod player;
mod utils;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: "CaveGame".into(),
                    ..default()
                }),
                ..default()
            })
            .set(AssetPlugin {
                file_path: "assets".to_string(),
                ..default()
            })
        )
        .add_plugins(world::WorldPlugin)
        .add_plugins(render::RenderPlugin)
        .add_plugins(player::PlayerPlugin)
        .insert_resource(ClearColor(Color::srgb(0.5, 0.8, 1.0)))
        .run();
}
