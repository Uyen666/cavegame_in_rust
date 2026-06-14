pub mod main_menu;
pub mod hud;
pub mod settings;
pub mod debug;
use bevy::prelude::*;
use crate::GameState;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(debug::DebugUiPlugin)
           .add_systems(OnEnter(GameState::MainMenu), main_menu::setup_main_menu)
           .add_systems(OnExit(GameState::MainMenu), main_menu::cleanup_main_menu)
           
           .add_systems(OnEnter(GameState::InGame), hud::setup_crosshair)
           .add_systems(OnExit(GameState::InGame), hud::cleanup_crosshair)
           .add_systems(
               Update,
               hud::update_crosshair.run_if(in_state(GameState::InGame))
           )
           
           .add_systems(OnEnter(GameState::Settings), settings::setup_settings)
           .add_systems(OnExit(GameState::Settings), settings::cleanup_settings);
    }
}
