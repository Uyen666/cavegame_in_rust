use bevy::prelude::*;
use bevy::input::mouse::MouseMotion;
use bevy::window::{CursorGrabMode, PrimaryWindow};
use std::f32::consts::FRAC_PI_2;
use crate::world::{WorldManager, BlockType, Chunk};
use crate::utils::math::Aabb;

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (setup_player, setup_crosshair))
           .add_systems(Update, (player_look, player_move, toggle_grab_cursor, player_interaction, update_crosshair));
    }
}

#[derive(Component)]
pub struct CrosshairPart;

#[derive(Component)]
pub struct Player {
    pub velocity: Vec3,
    pub pitch: f32,
    pub yaw: f32,
    pub on_ground: bool,
    pub is_crouching: bool,
}

#[derive(Component)]
pub struct PlayerCamera;

fn setup_player(mut commands: Commands, mut q_windows: Query<&mut Window, With<PrimaryWindow>>) {
    // Grab cursor
    if let Ok(mut window) = q_windows.get_single_mut() {
        window.cursor.grab_mode = CursorGrabMode::Locked;
        window.cursor.visible = false;
    }

    commands.spawn((
        Player {
            velocity: Vec3::ZERO,
            pitch: 0.0,
            yaw: 0.0,
            on_ground: false,
            is_crouching: false,
        },
        Transform::from_xyz(16.0, 8.0, 16.0), // Spawn above ground (grass is at y=4)
        GlobalTransform::default(),
        VisibilityBundle::default(),
    )).with_children(|parent| {
        parent.spawn((
            Camera3dBundle {
                transform: Transform::from_xyz(0.0, 1.6, 0.0),
                ..default()
            },
            PlayerCamera,
        ));
    });
}

fn toggle_grab_cursor(
    mut q_windows: Query<&mut Window, With<PrimaryWindow>>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse_btn: Res<ButtonInput<MouseButton>>,
) {
    let Ok(mut window) = q_windows.get_single_mut() else { return; };

    // 按 ESC 鍵解鎖滑鼠
    if keys.just_pressed(KeyCode::Escape) {
        window.cursor.grab_mode = CursorGrabMode::None;
        window.cursor.visible = true;
    }

    // 點擊左鍵重新鎖定滑鼠 (如果目前不在鎖定狀態)
    if mouse_btn.just_pressed(MouseButton::Left) {
        window.cursor.grab_mode = CursorGrabMode::Locked;
        window.cursor.visible = false;
    }
}

fn player_look(
    mut q_player: Query<&mut Player>,
    mut q_camera: Query<&mut Transform, With<PlayerCamera>>,
    mut mouse_motion_events: EventReader<MouseMotion>,
    q_windows: Query<&Window, With<PrimaryWindow>>,
) {
    let Ok(window) = q_windows.get_single() else {
        return;
    };

    if window.cursor.grab_mode != CursorGrabMode::Locked {
        return;
    }

    let mut player = q_player.single_mut();
    let mut camera_transform = q_camera.single_mut();

    let mut delta = Vec2::ZERO;
    for event in mouse_motion_events.read() {
        delta += event.delta;
    }

    if delta == Vec2::ZERO {
        return;
    }

    let sensitivity = 0.002;
    player.yaw -= delta.x * sensitivity;
    player.pitch -= delta.y * sensitivity;
    player.pitch = player.pitch.clamp(-FRAC_PI_2 + 0.01, FRAC_PI_2 - 0.01);

    camera_transform.rotation = Quat::from_axis_angle(Vec3::Y, player.yaw)
        * Quat::from_axis_angle(Vec3::X, player.pitch);
}

use crate::utils::physics::swept_aabb;

/// 在單一軸向上進行 Swept AABB 碰撞，回傳安全的實際移動距離
fn sweep_axis(pos: Vec3, size_min: Vec3, size_max: Vec3, velocity_1d: Vec3,
              world: &WorldManager, q_chunks: &Query<(Entity, &Chunk)>) -> (f32, bool) {
    let player_aabb = Aabb::new(pos + size_min, pos + size_max);
    let swept_box = player_aabb.expand_by_velocity(velocity_1d);

    let min_x = (swept_box.min.x - 0.001).floor() as i32;
    let max_x = (swept_box.max.x + 0.001).ceil() as i32;
    let min_y = (swept_box.min.y - 0.001).floor() as i32;
    let max_y = (swept_box.max.y + 0.001).ceil() as i32;
    let min_z = (swept_box.min.z - 0.001).floor() as i32;
    let max_z = (swept_box.max.z + 0.001).ceil() as i32;

    let mut earliest_t = 1.0_f32;
    let mut hit = false;

    for x in min_x..max_x {
        for y in min_y..max_y {
            for z in min_z..max_z {
                let bp = IVec3::new(x, y, z);
                if world.get_block_global(bp, q_chunks).is_solid() {
                    let block_aabb = Aabb::new(
                        Vec3::new(x as f32, y as f32, z as f32),
                        Vec3::new(x as f32 + 1.0, y as f32 + 1.0, z as f32 + 1.0),
                    );
                    let (t, normal) = swept_aabb(&player_aabb, velocity_1d, &block_aabb);
                    if t < earliest_t {
                        // Internal-face culling: only collide if face is exposed
                        let neighbor = bp + IVec3::new(normal.x as i32, normal.y as i32, normal.z as i32);
                        if !world.get_block_global(neighbor, q_chunks).is_solid() {
                            earliest_t = t;
                            hit = true;
                        }
                    }
                }
            }
        }
    }

    // Apply a small safety gap so AABB never actually touches the surface
    let safe_t = if hit { (earliest_t - 0.001).max(0.0) } else { 1.0 };
    (safe_t, hit)
}

fn player_move(
    mut q_player: Query<(&mut Player, &mut Transform)>,
    mut q_camera: Query<&mut Transform, (With<PlayerCamera>, Without<Player>)>,
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    world: Res<WorldManager>,
    q_chunks: Query<(Entity, &Chunk)>,
) {
    let (mut player, mut transform) = q_player.single_mut();
    let dt = time.delta_seconds();

    if dt < 0.0001 {
        return;
    }

    // --- Player dimensions ---
    player.is_crouching = keys.pressed(KeyCode::ControlLeft);
    let player_height = if player.is_crouching { 1.2_f32 } else { 1.8_f32 };
    let player_radius = 0.3_f32;
    let is_sprinting = keys.pressed(KeyCode::ShiftLeft);
    let move_speed = if player.is_crouching { 
        2.5_f32 
    } else if is_sprinting {
        5.6_f32
    } else { 
        4.3_f32 
    };

    // Offsets relative to pos (pos = feet center)
    let size_min = Vec3::new(-player_radius, 0.0, -player_radius);
    let size_max = Vec3::new( player_radius, player_height,  player_radius);

    // --- Camera crouch lerp ---
    if let Ok(mut cam) = q_camera.get_single_mut() {
        let target_cam_y = if player.is_crouching { 1.0 } else { 1.6 };
        cam.translation.y += (target_cam_y - cam.translation.y) * (1.0 - (-10.0_f32 * dt).exp());
    }

    // --- Gravity (accumulate BEFORE movement) ---
    player.velocity.y -= 25.0 * dt;

    // --- Horizontal input ---
    let forward = Vec3::new(-player.yaw.sin(), 0.0, -player.yaw.cos());
    let right   = Vec3::new( player.yaw.cos(), 0.0, -player.yaw.sin());
    let mut input_dir = Vec3::ZERO;
    if keys.pressed(KeyCode::KeyW) { input_dir += forward; }
    if keys.pressed(KeyCode::KeyS) { input_dir -= forward; }
    if keys.pressed(KeyCode::KeyD) { input_dir += right; }
    if keys.pressed(KeyCode::KeyA) { input_dir -= right; }
    if input_dir.length_squared() > 0.0 { input_dir = input_dir.normalize(); }

    // Instant horizontal speed (no lerp – avoids interaction with vertical)
    player.velocity.x = input_dir.x * move_speed;
    player.velocity.z = input_dir.z * move_speed;

    // --- Jump ---
    if keys.just_pressed(KeyCode::Space) && player.on_ground {
        player.velocity.y = 8.0;
        player.on_ground = false;
    }

    // -------------------------------------------------------
    // Separate-axis Swept AABB collision
    // X → Y → Z; velocity is NOT back-calculated from position
    // -------------------------------------------------------
    let mut pos = transform.translation;

    // X axis
    let move_x = Vec3::new(player.velocity.x * dt, 0.0, 0.0);
    if move_x.length_squared() > 0.000001 {
        let (t, hit) = sweep_axis(pos, size_min, size_max, move_x, &world, &q_chunks);
        pos.x += move_x.x * t;
        if hit { player.velocity.x = 0.0; }
    }

    // Y axis
    let move_y = Vec3::new(0.0, player.velocity.y * dt, 0.0);
    if move_y.length_squared() > 0.000001 {
        let (t, hit) = sweep_axis(pos, size_min, size_max, move_y, &world, &q_chunks);
        pos.y += move_y.y * t;
        if hit {
            if player.velocity.y < 0.0 {
                player.on_ground = true;
            }
            player.velocity.y = 0.0;
        } else {
            player.on_ground = false;
        }
    }

    // Z axis
    let move_z = Vec3::new(0.0, 0.0, player.velocity.z * dt);
    if move_z.length_squared() > 0.000001 {
        let (t, hit) = sweep_axis(pos, size_min, size_max, move_z, &world, &q_chunks);
        pos.z += move_z.z * t;
        if hit { player.velocity.z = 0.0; }
    }

    transform.translation = pos;
}


fn player_interaction(
    keys: Res<ButtonInput<MouseButton>>,
    world: Res<WorldManager>,
    mut q_chunks_mut: Query<(Entity, &mut Chunk)>,
    q_camera: Query<&GlobalTransform, With<PlayerCamera>>,
    q_windows: Query<&Window, With<PrimaryWindow>>,
) {
    let left = keys.just_pressed(MouseButton::Left);
    let right = keys.just_pressed(MouseButton::Right);

    if !left && !right {
        return;
    }
    
    let Ok(window) = q_windows.get_single() else { return; };
    if window.cursor.grab_mode != CursorGrabMode::Locked { return; }

    if let Ok(cam_transform) = q_camera.get_single() {
        let start = cam_transform.translation();
        let forward = cam_transform.forward();
        let max_dist = 5.0;

        let mut dist = 0.0;
        let step = 0.05;
        let mut last_air_pos = None;

        while dist < max_dist {
            let pos = start + forward * dist;
            // 修正：3D 體素座標定位必須使用 floor()，與 AABB 的標準對齊
            let block_pos = IVec3::new(pos.x.floor() as i32, pos.y.floor() as i32, pos.z.floor() as i32);

            let block = world.get_block_global_mut(block_pos, &q_chunks_mut);
            if block.is_solid() {
                if left {
                    world.set_block_global(block_pos, BlockType::Air, &mut q_chunks_mut);
                } else if right {
                    if let Some(place_pos) = last_air_pos {
                        world.set_block_global(place_pos, BlockType::Stone, &mut q_chunks_mut);
                    }
                }
                break;
            } else {
                last_air_pos = Some(block_pos);
            }
            dist += step;
        }
    }
}

fn setup_crosshair(mut commands: Commands) {
    // Crosshair container to center everything
    commands.spawn(NodeBundle {
        style: Style {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        ..default()
    }).with_children(|parent| {
        // Horizontal line
        parent.spawn((
            NodeBundle {
                style: Style {
                    width: Val::Px(10.0),
                    height: Val::Px(2.0),
                    position_type: PositionType::Absolute,
                    ..default()
                },
                background_color: BackgroundColor(Color::srgb(0.2, 0.2, 0.2)),
                ..default()
            },
            CrosshairPart,
        ));
        // Vertical line
        parent.spawn((
            NodeBundle {
                style: Style {
                    width: Val::Px(2.0),
                    height: Val::Px(10.0),
                    position_type: PositionType::Absolute,
                    ..default()
                },
                background_color: BackgroundColor(Color::srgb(0.2, 0.2, 0.2)),
                ..default()
            },
            CrosshairPart,
        ));
    });
}

fn update_crosshair(
    mut q_crosshair: Query<&mut BackgroundColor, With<CrosshairPart>>,
    q_camera: Query<&GlobalTransform, With<PlayerCamera>>,
    world: Res<WorldManager>,
    q_chunks: Query<(Entity, &Chunk)>,
) {
    let mut hit_dark = false;

    if let Ok(cam_transform) = q_camera.get_single() {
        let start = cam_transform.translation();
        let forward = cam_transform.forward();
        let max_dist = 50.0;

        let mut dist = 0.0;
        let step = 0.5;
        while dist < max_dist {
            let pos = start + forward * dist;
            let block_pos = IVec3::new(pos.x.floor() as i32, pos.y.floor() as i32, pos.z.floor() as i32);

            let block = world.get_block_global(block_pos, &q_chunks);
            if block.is_solid() {
                if block == BlockType::Stone {
                    hit_dark = true;
                }
                break;
            }
            dist += step;
        }
    }

    let target_color = if hit_dark {
        Color::srgb(0.9, 0.9, 0.9)
    } else {
        Color::srgb(0.2, 0.2, 0.2)
    };

    for mut bg in q_crosshair.iter_mut() {
        bg.0 = target_color;
    }
}


