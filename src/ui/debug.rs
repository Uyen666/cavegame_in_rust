use bevy::prelude::*;
use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use crate::GameState;
use crate::player::Player;
use crate::world::WorldManager;

pub struct DebugUiPlugin;

impl Plugin for DebugUiPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<FrameTimeDiagnosticsPlugin>() {
            app.add_plugins(FrameTimeDiagnosticsPlugin::default());
        }

        app.insert_resource(DebugUpdateTimer(Timer::from_seconds(0.5, TimerMode::Repeating)));

        app.add_systems(OnEnter(GameState::InGame), setup_debug_ui)
           .add_systems(OnExit(GameState::InGame), cleanup_debug_ui)
           .add_systems(
               Update,
               (toggle_debug_ui, update_debug_text)
                   .run_if(in_state(GameState::InGame))
           );
    }
}

#[derive(Component)]
struct DebugUiRoot;

#[derive(Component)]
struct DebugText;

#[derive(Resource)]
struct DebugUpdateTimer(Timer);

fn setup_debug_ui(mut commands: Commands) {
    let font_size = 18.0;

    commands.spawn((
        NodeBundle {
            style: Style {
                position_type: PositionType::Absolute,
                top: Val::Px(5.0),
                left: Val::Px(5.0),
                padding: UiRect::all(Val::Px(8.0)),
                ..default()
            },
            background_color: Color::srgba(0.0, 0.0, 0.0, 0.4).into(),
            visibility: Visibility::Hidden, // Default to hidden
            ..default()
        },
        DebugUiRoot,
    )).with_children(|parent| {
        parent.spawn((
            TextBundle::from_section(
                "Debug Info",
                TextStyle {
                    font_size,
                    color: Color::WHITE,
                    ..default()
                },
            ),
            DebugText,
        ));
    });
}

fn cleanup_debug_ui(mut commands: Commands, q_root: Query<Entity, With<DebugUiRoot>>) {
    for entity in q_root.iter() {
        commands.entity(entity).despawn_recursive();
    }
}

fn toggle_debug_ui(
    keys: Res<ButtonInput<KeyCode>>,
    mut q_root: Query<&mut Visibility, With<DebugUiRoot>>,
) {
    if keys.just_pressed(KeyCode::F3) {
        for mut visibility in q_root.iter_mut() {
            *visibility = match *visibility {
                Visibility::Hidden => Visibility::Inherited,
                _ => Visibility::Hidden,
            };
        }
    }
}

fn update_debug_text(
    diagnostics: Res<DiagnosticsStore>,
    mut q_text: Query<&mut Text, With<DebugText>>,
    q_root: Query<&Visibility, With<DebugUiRoot>>,
    q_player: Query<&Transform, With<Player>>,
    world_manager: Res<WorldManager>,
    mut timer: ResMut<DebugUpdateTimer>,
    time: Res<Time>,
    mut fps_cache: Local<String>,
) {
    let Ok(visibility) = q_root.get_single() else { return };
    if *visibility == Visibility::Hidden {
        return; // Don't compute anything if hidden
    }

    // Only update FPS cache every 0.5s to prevent flickering
    if timer.0.tick(time.delta()).just_finished() || fps_cache.is_empty() {
        let mut fps = 0.0;
        let mut frame_time = 0.0;

        if let Some(fps_diagnostic) = diagnostics.get(&FrameTimeDiagnosticsPlugin::FPS) {
            if let Some(value) = fps_diagnostic.smoothed() {
                fps = value;
            }
        }
        if let Some(frame_time_diagnostic) = diagnostics.get(&FrameTimeDiagnosticsPlugin::FRAME_TIME) {
            if let Some(value) = frame_time_diagnostic.smoothed() {
                frame_time = value;
            }
        }
        *fps_cache = format!("FPS: {:.0} ({:.2}ms)", fps, frame_time);
    }

    // Always compute player coordinates every frame so movement is smooth
    let (x, y, z) = if let Ok(player_transform) = q_player.get_single() {
        let pos = player_transform.translation;
        (pos.x, pos.y, pos.z)
    } else {
        (0.0, 0.0, 0.0)
    };

    let pos_ivec = IVec3::new(x.floor() as i32, y.floor() as i32, z.floor() as i32);
    let (chunk_pos, local_pos) = WorldManager::global_to_chunk_pos(pos_ivec);
    
    let loaded_chunks = world_manager.chunks.len();

    let text_content = format!(
        "Cavegame Dev 2026\n\
         {}\n\
         Pos: X: {:.2}, Y: {:.2}, Z: {:.2}\n\
         Chunk: CX: {}, CY: {}, CZ: {} [bx: {}, by: {}, bz: {}]\n\
         Loaded Chunks: {}",
        *fps_cache,
        x, y, z,
        chunk_pos.x, chunk_pos.y, chunk_pos.z, local_pos.x, local_pos.y, local_pos.z,
        loaded_chunks
    );

    for mut text in q_text.iter_mut() {
        text.sections[0].value = text_content.clone();
    }
}
