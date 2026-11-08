use bevy::prelude::*;

mod world;
mod render;
mod player;
mod utils;
mod phys;
mod ui;
mod config;
mod item;

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

    let asset_path = {
        if std::path::Path::new("assets").exists() {
            "assets".to_string()
        } else if let Ok(mut exe_dir) = std::env::current_exe() {
            exe_dir.pop(); // 移除 exe
            if exe_dir.join("assets").exists() {
                exe_dir.join("assets").to_string_lossy().to_string()
            } else {
                exe_dir.pop(); // target
                exe_dir.pop(); // 根目錄
                if exe_dir.join("assets").exists() {
                    exe_dir.join("assets").to_string_lossy().to_string()
                } else {
                    "assets".to_string()
                }
            }
        } else {
            "assets".to_string()
        }
    };

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
                file_path: asset_path,
                ..default()
            })
        )
        .init_state::<GameState>()
        .insert_resource(Time::<Fixed>::from_hz(128.0))
        .insert_resource(config::EngineConfig::default())
        .add_plugins(ui::UiPlugin)
        .add_plugins(world::WorldPlugin)
        .add_plugins(render::RenderPlugin)
        .add_plugins(player::PlayerPlugin)
        .add_plugins(phys::PhysicsPlugin)
        .add_plugins(item::ItemPlugin)
        .add_systems(
            FixedUpdate,
            (
                player::player_look,
                player::player_move,
                phys::systems::update_fluid_sensors,
                phys::systems::apply_kinematics,
                phys::systems::resolve_collisions,
            ).chain().run_if(in_state(GameState::InGame))
        )
        .insert_resource(ClearColor(Color::srgb(0.5, 0.8, 1.0)))
        .run();
}
