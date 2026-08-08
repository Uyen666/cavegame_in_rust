use bevy::prelude::*;
use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use crate::GameState;
use crate::world::WorldManager;

pub struct DebugUiPlugin;

impl Plugin for DebugUiPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<FrameTimeDiagnosticsPlugin>() {
            app.add_plugins(FrameTimeDiagnosticsPlugin::default());
        }

        app.insert_resource(DebugUpdateTimer(Timer::from_seconds(0.5, TimerMode::Repeating)));
        app.init_resource::<DebugConfig>();

        app.add_systems(OnEnter(GameState::InGame), setup_debug_ui)
           .add_systems(OnExit(GameState::InGame), cleanup_debug_ui)
           .add_systems(
               Update,
               (toggle_debug_ui, update_debug_text, draw_chunk_gizmos)
                   .run_if(in_state(GameState::InGame))
           );
    }
}

#[derive(Resource, Default)]
pub struct DebugConfig {
    pub show_chunk_borders: bool,
    pub show_light_levels: bool,
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
    mut config: ResMut<DebugConfig>,
    mut engine_config: ResMut<crate::config::EngineConfig>,
    mut q_chunks: Query<&mut crate::world::Chunk>,
) {
    if keys.just_pressed(KeyCode::F3) {
        for mut visibility in q_root.iter_mut() {
            *visibility = match *visibility {
                Visibility::Hidden => Visibility::Inherited,
                _ => Visibility::Hidden,
            };
        }
    }

    if let Ok(vis) = q_root.get_single() {
        if *vis != Visibility::Hidden {
            if keys.just_pressed(KeyCode::KeyC) {
                config.show_chunk_borders = !config.show_chunk_borders;
                println!("【系統通知】區塊邊界: {}", if config.show_chunk_borders { "ON" } else { "OFF" });
            }
            if keys.just_pressed(KeyCode::KeyL) {
                config.show_light_levels = !config.show_light_levels;
                println!("【系統通知】光照除錯面板: {}", if config.show_light_levels { "ON" } else { "OFF" });
            }
            if keys.just_pressed(KeyCode::KeyP) {
                engine_config.smooth_lighting = !engine_config.smooth_lighting;
                println!("【系統通知】平滑光照 (Smooth Lighting): {}", if engine_config.smooth_lighting { "ON" } else { "OFF" });
                
                // 🚀 剛性連鎖防線：強制全網格立刻失效，逼迫 greedy.rs 在下一幀集體重新烘焙！
                for mut chunk in q_chunks.iter_mut() {
                    chunk.is_dirty = true;
                }
            }
        }
    }
}

fn update_debug_text(
    diagnostics: Res<DiagnosticsStore>,
    mut q_text: Query<&mut Text, With<DebugText>>,
    q_root: Query<&Visibility, With<DebugUiRoot>>,
    q_player: Query<(&Transform, &crate::item::Inventory)>,
    q_camera: Query<&GlobalTransform, With<crate::player::PlayerCamera>>,
    world_manager: Res<WorldManager>,
    cycle: Res<crate::world::DayNightCycle>,
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
    let (x, y, z, held_block) = if let Ok((player_transform, inventory)) = q_player.get_single() {
        let pos = player_transform.translation;
        let item_str = if let Some(stack) = inventory.selected_item() {
            if let Some(dur) = stack.durability {
                format!("{:?} x{} (Dur: {})", stack.item_type, stack.count, dur)
            } else {
                format!("{:?} x{}", stack.item_type, stack.count)
            }
        } else {
            "Empty".to_string()
        };
        (pos.x, pos.y, pos.z, item_str)
    } else {
        (0.0, 0.0, 0.0, String::from("Unknown"))
    };

    let foot_pos = IVec3::new(x.floor() as i32, y.floor() as i32, z.floor() as i32);
    let eye_pos = foot_pos + IVec3::Y;

    let foot_sky = world_manager.get_sky_light_global(foot_pos);
    let foot_block = world_manager.get_block_light_global(foot_pos);
    let eye_sky = world_manager.get_sky_light_global(eye_pos);
    let eye_block = world_manager.get_block_light_global(eye_pos);

    // Dynamic sky factor
    let foot_eff = (foot_sky as f32 * cycle.sky_factor).max(foot_block as f32);
    let eye_eff = (eye_sky as f32 * cycle.sky_factor).max(eye_block as f32);

    let pos_ivec = foot_pos;
    let (chunk_pos, local_pos) = WorldManager::global_to_chunk_pos(pos_ivec);
    
    let loaded_entity_chunks = world_manager.chunk_entity_count();
    let loaded_data_chunks   = world_manager.chunk_data_count();

    // ── 實作視線射線 (Raycast Target Query) ──
    let mut targeted_text = String::from("Targeted Block: Looking at air");
    if let Ok(cam_tf) = q_camera.get_single() {
        let cam_pos = cam_tf.translation();
        let cam_forward = cam_tf.forward();
        
        let mut prev_ivec = IVec3::new(cam_pos.x.floor() as i32, cam_pos.y.floor() as i32, cam_pos.z.floor() as i32);
        
        // 步長 0.1，探測 6 公尺 (約 60 步)
        for step in 1..=60 {
            let t = step as f32 * 0.1;
            let current_pos = cam_pos + cam_forward * t;
            let current_ivec = IVec3::new(current_pos.x.floor() as i32, current_pos.y.floor() as i32, current_pos.z.floor() as i32);
            
            if current_ivec != prev_ivec {
                let block = world_manager.get_block_global(current_ivec);
                if block != crate::world::BlockType::Air {
                    // 撞擊到固體！
                    let target_sky = world_manager.get_sky_light_global(prev_ivec);
                    let target_block = world_manager.get_block_light_global(prev_ivec);
                    let target_eff = (target_sky as f32 * cycle.sky_factor).max(target_block as f32);
                    let raw_target_fluid = world_manager.get_fluid_global(prev_ivec);
                    let target_fluid_level = raw_target_fluid & 0x0F;
                    targeted_text = format!(
                        "Targeted Block: {}, {}, {}\n\
                         Targeted Light: {:.1} (sky: {}, block: {})\n\
                         Targeted Fluid Level: {}",
                        current_ivec.x, current_ivec.y, current_ivec.z,
                        target_eff, target_sky, target_block,
                        target_fluid_level
                    );
                    break;
                }
                prev_ivec = current_ivec;
            }
        }
    }

    let text_content = format!(
        "Cavegame Dev 2026\n\
         {}\n\
         Pos: X: {:.2}, Y: {:.2}, Z: {:.2}\n\
         Chunk: CX: {}, CY: {}, CZ: {} [bx: {}, by: {}, bz: {}]\n\
         Client Light: Eye: {:.1} (sky: {}, block: {})\n\
                       Foot: {:.1} (sky: {}, block: {})\n\
         {}\n\
         Holding: {}\n\
         Loaded Chunks: [E: {} / D: {}]",
        *fps_cache,
        x, y, z,
        chunk_pos.x, chunk_pos.y, chunk_pos.z, local_pos.x, local_pos.y, local_pos.z,
        eye_eff, eye_sky, eye_block,
        foot_eff, foot_sky, foot_block,
        targeted_text,
        held_block,
        loaded_entity_chunks, loaded_data_chunks
    );

    for mut text in q_text.iter_mut() {
        text.sections[0].value = text_content.clone();
    }
}

fn draw_chunk_gizmos(
    mut gizmos: Gizmos,
    config: Res<DebugConfig>,
    q_chunks: Query<&Transform, With<crate::world::Chunk>>,
) {
    if !config.show_chunk_borders {
        return;
    }

    for transform in q_chunks.iter() {
        let center = transform.translation + Vec3::splat(16.0);
        gizmos.cuboid(
            Transform::from_translation(center).with_scale(Vec3::splat(32.0)),
            Color::BLACK
        );
    }
}


