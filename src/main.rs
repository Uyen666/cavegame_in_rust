use bevy::prelude::*;

mod world;
mod render;
mod player;
mod utils;
mod phys;
mod ui;
mod config;

#[derive(States, Debug, Clone, Copy, Eq, PartialEq, Hash, Default)]
pub enum GameState {
    MainMenu,
    WorldCreation,
    #[default]
    InGame,
    Settings,
}

fn main() {
    std::panic::set_hook(Box::new(|info| {
        let backtrace = std::backtrace::Backtrace::capture();
        let _ = std::fs::write("panic.log", format!("Panic occurred: {}\nBacktrace:\n{}", info, backtrace));
    }));
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
        .init_state::<GameState>()
        .insert_resource(config::EngineConfig::default())
        .add_plugins(ui::UiPlugin)
        .add_plugins(world::WorldPlugin)
        .add_plugins(render::RenderPlugin)
        .add_plugins(player::PlayerPlugin)
        .insert_resource(ClearColor(Color::srgb(0.5, 0.8, 1.0)))
        .run();
}
