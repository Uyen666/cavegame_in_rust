use bevy::prelude::*;
use bevy::input::mouse::MouseMotion;
use bevy::window::{CursorGrabMode, PrimaryWindow};
use std::f32::consts::FRAC_PI_2;
use crate::world::{WorldManager, BlockType, Chunk};
use crate::utils::math::Aabb;

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_player)
           .add_systems(
               Update,
               (player_look, player_move, toggle_grab_cursor, player_interaction)
                   .run_if(in_state(crate::GameState::InGame))
           );
    }
}

#[derive(Component)]
pub struct Player {
    pub velocity: Vec3,
    pub pitch: f32,
    pub yaw: f32,
    pub on_ground: bool,
    pub is_crouching: bool,
    pub is_spectator: bool,
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
            is_spectator: false,
        },
        Transform::from_xyz(16.0, 35.0, 16.0), // 為了適應山脈地形，將初始高度拉高，利用重力自然降落
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

fn player_move(
    mut q_player: Query<(&mut Player, &mut Transform)>,
    mut q_camera: Query<&mut Transform, (With<PlayerCamera>, Without<Player>)>,
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    world: Res<WorldManager>,
) {
    let (mut player, mut transform) = q_player.single_mut();
    let dt = time.delta_seconds();

    if dt < 0.0001 {
        return;
    }

    // ── F4 切換旁觀者模式 (必須在所有物理機制前優先處理) ──────────────
    if keys.just_pressed(KeyCode::F4) {
        player.is_spectator = !player.is_spectator;
        println!("【系統通知】玩家切換模式！旁觀者狀態: {}", player.is_spectator);
    }

    if player.is_spectator {
        // 🚀 【旁觀者分支】：擁有至高無上的相機自由，嚴禁任何物理防禦機制干涉座標！
        player.velocity.y = 0.0; // 清除重力影響

        let mut input_dir = Vec3::ZERO;
        let forward = Vec3::new(-player.yaw.sin(), 0.0, -player.yaw.cos()).normalize();
        let right   = Vec3::new( player.yaw.cos(), 0.0, -player.yaw.sin()).normalize();

        if keys.pressed(KeyCode::KeyW) { input_dir += forward; }
        if keys.pressed(KeyCode::KeyS) { input_dir -= forward; }
        if keys.pressed(KeyCode::KeyD) { input_dir += right; }
        if keys.pressed(KeyCode::KeyA) { input_dir -= right; }
        if keys.pressed(KeyCode::Space) { input_dir.y += 1.0; }
        if keys.pressed(KeyCode::ShiftLeft) { input_dir.y -= 1.0; }

        if input_dir.length_squared() > 0.0 {
            input_dir = input_dir.normalize();
        }

        let spec_speed = if keys.pressed(KeyCode::ControlLeft) { 32.0 } else { 16.0 };
        transform.translation += input_dir * spec_speed * dt;

        return; // 🚀 直接結束！短路下方的防虛空安全門、卡死救援與方塊碰撞！
    }

    // 安全閘門已移除：現在 get_block_global 會自動對超出加載邊界的區塊進行高度自適應回傳，
    // 高空為 Air 允許自由下落，地底為 Stone 給予實心支撐。

    // --- Player dimensions ---
    player.is_crouching = keys.pressed(KeyCode::ControlLeft);
    let player_height = if player.is_crouching { 1.5_f32 } else { 1.8_f32 };
    let player_radius = 0.3_f32;
    let is_sprinting = keys.pressed(KeyCode::ShiftLeft);
    let move_speed = if player.is_crouching { 
        2.5_f32 
    } else if is_sprinting {
        5.6_f32
    } else { 
        4.3_f32 
    };
    // ❌ 【正常生存分支】：執行原本的重力與碰撞結算
    // --- Camera crouch lerp ---
    if let Ok(mut cam) = q_camera.get_single_mut() {
        let target_cam_y = if player.is_crouching { 1.2 } else { 1.6 }; // Adjusted camera height for 1.5 crouch
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
    // Axis-Separated Movement (X -> Z -> Y)
    // -------------------------------------------------------
    let mut pos = transform.translation;
    const EPSILON: f32 = 0.001;

    let get_intersecting_blocks = |p: Vec3| -> Vec<Aabb> {
        let p_aabb = Aabb::new(
            Vec3::new(p.x - player_radius, p.y, p.z - player_radius),
            Vec3::new(p.x + player_radius, p.y + player_height, p.z + player_radius),
        );
        let min_x = (p.x - player_radius).floor() as i32;
        let max_x = (p.x + player_radius).ceil() as i32;
        let min_y = p.y.floor() as i32;
        let max_y = (p.y + player_height).ceil() as i32;
        let min_z = (p.z - player_radius).floor() as i32;
        let max_z = (p.z + player_radius).ceil() as i32;

        let mut hits = Vec::new();
        for x in min_x..max_x {
            for y in min_y..max_y {
                for z in min_z..max_z {
                    let b_pos = IVec3::new(x, y, z);
                    if world.get_block_global(b_pos).is_solid() {
                        let b_aabb = Aabb::new(
                            Vec3::new(x as f32, y as f32, z as f32),
                            Vec3::new(x as f32 + 1.0, y as f32 + 1.0, z as f32 + 1.0),
                        );
                        if p_aabb.intersects(&b_aabb) {
                            hits.push(b_aabb);
                        }
                    }
                }
            }
        }
        hits
    };

    // X axis
    if player.velocity.x != 0.0 {
        pos.x += player.velocity.x * dt;
        let hits = get_intersecting_blocks(pos);
        if !hits.is_empty() {
            if player.velocity.x > 0.0 {
                let wall_x = hits.iter().map(|b| b.min.x).fold(f32::INFINITY, f32::min);
                pos.x = wall_x - player_radius - EPSILON;
            } else {
                let wall_x = hits.iter().map(|b| b.max.x).fold(f32::NEG_INFINITY, f32::max);
                pos.x = wall_x + player_radius + EPSILON;
            }
            player.velocity.x = 0.0;
        }
    }

    // Z axis
    if player.velocity.z != 0.0 {
        pos.z += player.velocity.z * dt;
        let hits = get_intersecting_blocks(pos);
        if !hits.is_empty() {
            if player.velocity.z > 0.0 {
                let wall_z = hits.iter().map(|b| b.min.z).fold(f32::INFINITY, f32::min);
                pos.z = wall_z - player_radius - EPSILON;
            } else {
                let wall_z = hits.iter().map(|b| b.max.z).fold(f32::NEG_INFINITY, f32::max);
                pos.z = wall_z + player_radius + EPSILON;
            }
            player.velocity.z = 0.0;
        }
    }

    // Y axis
    player.on_ground = false;
    if player.velocity.y != 0.0 {
        pos.y += player.velocity.y * dt;
        let hits = get_intersecting_blocks(pos);
        if !hits.is_empty() {
            if player.velocity.y > 0.0 {
                let ceil_y = hits.iter().map(|b| b.min.y).fold(f32::INFINITY, f32::min);
                pos.y = ceil_y - player_height - EPSILON;
            } else {
                let ground_y = hits.iter().map(|b| b.max.y).fold(f32::NEG_INFINITY, f32::max);
                pos.y = ground_y; // Precise grounding
                player.on_ground = true;
            }
            player.velocity.y = 0.0;
        }
    }

    transform.translation = pos;
}


fn player_interaction(
    mut commands: Commands,
    mouse_keys: Res<ButtonInput<MouseButton>>,
    kbd_keys: Res<ButtonInput<KeyCode>>,
    mut world: ResMut<WorldManager>,
    mut q_chunks_mut: Query<(Entity, &mut Chunk)>,
    q_camera: Query<&GlobalTransform, With<PlayerCamera>>,
    q_windows: Query<&Window, With<PrimaryWindow>>,
    q_player: Query<(&Transform, &Player)>,
) {
    let left = mouse_keys.just_pressed(MouseButton::Left);
    let right = mouse_keys.just_pressed(MouseButton::Right);
    let key_f = kbd_keys.just_pressed(KeyCode::KeyF);

    if !left && !right && !key_f {
        return;
    }
    
    let Ok(window) = q_windows.get_single() else { return; };
    if window.cursor.grab_mode != CursorGrabMode::Locked { return; }
    
    let Ok((player_transform, player)) = q_player.get_single() else { return; };

    // 🚀 旁觀者權限閹割：禁止修改世界幾何
    if player.is_spectator {
        return; 
    }

    if let Ok(cam_transform) = q_camera.get_single() {
        let start = cam_transform.translation();
        let forward = cam_transform.forward();
        let max_dist = 5.0;

        let mut dist = 0.0;
        let step = 0.05;
        let mut last_air_pos: Option<IVec3> = None;

        while dist < max_dist {
            let pos = start + forward * dist;
            // 修正：3D 體素座標定位必須使用 floor()，與 AABB 的標準對齊
            let block_pos = IVec3::new(pos.x.floor() as i32, pos.y.floor() as i32, pos.z.floor() as i32);

            let block = world.get_block_global(block_pos);
            if block.is_solid() {
                if left {
                    world.set_block_global(block_pos, BlockType::Air, &mut q_chunks_mut, &mut commands);
                    
                    // 【流體聯鎖喚醒機制】(Fluid Block Update Hook)
                    // 當方塊被挖除時，主動探測半徑 4 格範圍內的流體
                    // 若有流體，將其重新壓入 BFS 佇列，觸發路徑重算與蔓延！
                    crate::world::fluid::wake_up_fluids_in_radius(&mut world, block_pos);
                } else if right {
                    if let Some(place_pos) = last_air_pos {
                        let block_aabb = Aabb::new(
                            Vec3::new(place_pos.x as f32, place_pos.y as f32, place_pos.z as f32),
                            Vec3::new(place_pos.x as f32 + 1.0, place_pos.y as f32 + 1.0, place_pos.z as f32 + 1.0),
                        );

                        let p_pos = player_transform.translation;
                        let player_aabb = Aabb::new(
                            Vec3::new(p_pos.x - 0.3, p_pos.y, p_pos.z - 0.3),
                            Vec3::new(p_pos.x + 0.3, p_pos.y + 1.8, p_pos.z + 0.3),
                        );

                        if player_aabb.intersects(&block_aabb) {
                            break;
                        }

                        world.set_block_global(place_pos, BlockType::Stone, &mut q_chunks_mut, &mut commands);
                        crate::world::fluid::wake_up_fluids_in_radius(&mut world, place_pos);
                    }
                } else if key_f {
                    println!("🚀 [Fluid Debug] F Key Pressed! Detecting raycast...");
                    if let Some(place_pos) = last_air_pos {
                        if world.get_fluid_global(place_pos) > 0 {
                            world.set_fluid_global(place_pos, 0);
                            world.fluid_queue.push_back(place_pos);
                            for dir in [IVec3::Y, IVec3::NEG_Y, IVec3::X, IVec3::NEG_X, IVec3::Z, IVec3::NEG_Z] {
                                world.fluid_queue.push_back(place_pos + dir);
                            }
                            println!("🌊 [Fluid Debug] Removed water at: {:?}", place_pos);
                        } else {
                            world.set_fluid_global(place_pos, crate::config::MAX_FLUID_LEVEL | 0x80);
                            world.fluid_queue.push_back(place_pos);
                            for dir in [IVec3::Y, IVec3::NEG_Y, IVec3::X, IVec3::NEG_X, IVec3::Z, IVec3::NEG_Z] {
                                world.fluid_queue.push_back(place_pos + dir);
                            }
                            println!("🌊 [Fluid Debug] Successfully spawned water source at global pos: {:?}", place_pos);
                        }
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

