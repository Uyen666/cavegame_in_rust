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
        .insert_resource(Time::<Fixed>::from_hz(128.0))
        .insert_resource(config::EngineConfig::default())
        .add_plugins(ui::UiPlugin)
        .add_plugins(world::WorldPlugin)
        .add_plugins(render::RenderPlugin)
        .add_plugins(player::PlayerPlugin)
        .add_plugins(phys::PhysicsPlugin)
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
