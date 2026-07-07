use bevy::prelude::*;
use bevy::input::mouse::MouseMotion;
use bevy::window::{CursorGrabMode, PrimaryWindow};
use std::f32::consts::FRAC_PI_2;
use crate::world::{WorldManager, BlockType};
use crate::utils::math::Aabb;
use bevy::pbr::{FogSettings, FogFalloff};
use bevy::color::Mix;

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_player)
           .add_systems(
               Update,
               (toggle_grab_cursor, player_interaction, player_input_capture, update_fog_color)
                   .run_if(in_state(crate::GameState::InGame))
           )
           .add_systems(
               FixedUpdate,
               (player_look, player_move).run_if(in_state(crate::GameState::InGame))
           );
    }
}

fn player_input_capture(
    keys: Res<ButtonInput<KeyCode>>,
    mut scroll_evr: EventReader<bevy::input::mouse::MouseWheel>,
    mut q_player: Query<&mut Player>,
) {
    if let Ok(mut player) = q_player.get_single_mut() {
        if keys.just_pressed(KeyCode::Space) {
            player.wants_to_jump = true; // 🚀 鎖存點擊意圖
        }

        // --- 數字鍵切換 ---
        if keys.just_pressed(KeyCode::Digit1) { player.selected_slot = 0; }
        if keys.just_pressed(KeyCode::Digit2) { player.selected_slot = 1; }
        if keys.just_pressed(KeyCode::Digit3) { player.selected_slot = 2; }
        if keys.just_pressed(KeyCode::Digit4) { player.selected_slot = 3; }
        if keys.just_pressed(KeyCode::Digit5) { player.selected_slot = 4; }
        if keys.just_pressed(KeyCode::Digit6) { player.selected_slot = 5; }
        if keys.just_pressed(KeyCode::Digit7) { player.selected_slot = 6; }
        if keys.just_pressed(KeyCode::Digit8) { player.selected_slot = 7; }
        if keys.just_pressed(KeyCode::Digit9) { player.selected_slot = 8; }

        // --- 滾輪切換 ---
        let mut frame_scroll = 0.0;
        for ev in scroll_evr.read() {
            frame_scroll += ev.y;
        }
        player.scroll_accumulator += frame_scroll;
        
        let threshold = 0.7; // 靈敏度閥值
        if player.scroll_accumulator.abs() >= threshold {
            // 算出滾動方向：正值為 1 (向上滾), 負值為 -1 (向下滾)
            let direction = if player.scroll_accumulator > 0.0 { 1 } else { -1 };

            // 🚀 執行快捷列工整切換，注意加減方向可依據習慣對調
            let new_slot = player.selected_slot as i32 - direction;
            player.selected_slot = ((new_slot % 9 + 9) % 9) as usize;

            // 消費掉這一次的整數能量，保留餘數給下一幀，達成極致絲滑的連續滾動
            player.scroll_accumulator -= direction as f32 * threshold;
        }
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
    pub is_colliding_horizontally: bool,
    pub wants_to_jump: bool, // 🚀 跳躍輸入鎖存器（輸入緩衝）
    pub hotbar: [BlockType; 9],   // 9格快捷列陣列
    pub selected_slot: usize,     // 當前選中的欄位索引 (0 ~ 8)
    pub has_spawned: bool,        // 🚀 初次登入地表降落鎖
    pub scroll_accumulator: f32,  // 🚀 滾輪能量累加器
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
            is_colliding_horizontally: false,
            wants_to_jump: false,
            hotbar: [
                BlockType::Stone,
                BlockType::Dirt,
                BlockType::Grass,
                BlockType::Air,
                BlockType::Air,
                BlockType::Air,
                BlockType::Air,
                BlockType::Air,
                BlockType::Air,
            ],
            selected_slot: 0,
            has_spawned: false,
            scroll_accumulator: 0.0,
        },
        Transform::from_xyz(16.0, 250.0, 16.0), // 為了適應山脈地形，將初始高度拉到極限高空 (Y=250)，確保必定生於世界外表面，再利用重力自然降落
        GlobalTransform::default(),
        VisibilityBundle::default(),
    )).with_children(|parent| {
        parent.spawn((
            Camera3dBundle {
                transform: Transform::from_xyz(0.0, 1.6, 0.0),
                ..default()
            },
            PlayerCamera,
            FogSettings {
                color: Color::srgb(0.5, 0.8, 1.0),
                falloff: FogFalloff::Linear {
                    start: 32.0,
                    end: 128.0,
                },
                ..default()
            },
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
    config: Res<crate::config::EngineConfig>,
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

    // ── 初次出生點地面傳送 ──
    if !player.has_spawned {
        let cx = (transform.translation.x.floor() as i32) >> 5;
        let cz = (transform.translation.z.floor() as i32) >> 5;
        let mut top_loaded_cy = None;
        for cy in (0..=7).rev() {
            if world.get_chunk_ref(IVec3::new(cx, cy, cz)).is_some() {
                top_loaded_cy = Some(cy);
                break;
            }
        }
        
        if top_loaded_cy.is_some() {
            let mut surface_found = false;
            let mut surface_y = 250.0;
            let px = transform.translation.x.floor() as i32;
            let pz = transform.translation.z.floor() as i32;
            for y in (0..=255).rev() {
                let block = world.get_block_global(IVec3::new(px, y, pz));
                if block.is_solid() {
                    surface_y = (y + 1) as f32; // 站在固體頂部
                    surface_found = true;
                    break;
                }
            }
            if surface_found {
                transform.translation.y = surface_y;
                player.has_spawned = true;
                player.velocity = Vec3::ZERO;
                println!("【系統通知】玩家已安全降落於地表: Y={}", surface_y);
            }
        }
        
        if !player.has_spawned {
            // 如果還沒生成好，就讓他懸停在天上，凍結物理，直到 chunk 載入完畢！
            return;
        }
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

    // --- Determine Fluid State ---
    let b_pos = IVec3::new(transform.translation.x.floor() as i32, transform.translation.y.floor() as i32, transform.translation.z.floor() as i32);
    let foot_in_fluid = world.get_fluid_global(b_pos) > 0;

    let head_pos = IVec3::new(transform.translation.x.floor() as i32, (transform.translation.y + player_height).floor() as i32, transform.translation.z.floor() as i32);
    let head_in_fluid = world.get_fluid_global(head_pos) > 0;
    
    let mut current_move_speed = move_speed;

    // 🚀 雙軌起跳真理：不論是 Update 鎖存到了點擊，還是當前正按住不放，一律視為跳躍觸發！
    let is_jumping_triggered = player.wants_to_jump || keys.pressed(KeyCode::Space);

    // 🚀 陸地跳躍絕對優先權 (Ground Overrules Fluid)
    let is_ground_jumping = player.on_ground && is_jumping_triggered;
    
    // 🚀 官方級【水平碰撞上岸特權 (Jump From Fluid)】
    let is_fluid_climbing = foot_in_fluid && player.is_colliding_horizontally && is_jumping_triggered;
    
    // 🚀 官方級【水面起跳脫離衝量 (Surface Escape Impulse)】
    let is_surface_escape = foot_in_fluid && !head_in_fluid && is_jumping_triggered;

    // --- Horizontal input ---
    let forward = Vec3::new(-player.yaw.sin(), 0.0, -player.yaw.cos());
    let right   = Vec3::new( player.yaw.cos(), 0.0, -player.yaw.sin());
    let mut input_dir = Vec3::ZERO;
    if keys.pressed(KeyCode::KeyW) { input_dir += forward; }
    if keys.pressed(KeyCode::KeyS) { input_dir -= forward; }
    if keys.pressed(KeyCode::KeyD) { input_dir += right; }
    if keys.pressed(KeyCode::KeyA) { input_dir -= right; }
    if input_dir.length_squared() > 0.0 { input_dir = input_dir.normalize(); }

    if foot_in_fluid && !is_ground_jumping {
        current_move_speed *= 0.4;
        
        // Fluid horizontal speed with acceleration
        player.velocity.x += input_dir.x * current_move_speed * 10.0 * dt;
        player.velocity.z += input_dir.z * current_move_speed * 10.0 * dt;

        if is_fluid_climbing || is_surface_escape {
            // 大開綠燈，直接視為攀爬上岸 或 破浪而出！賦予完整的陸地跳躍衝量
            player.velocity.y = config.physics.land_jump_impulse;
            player.wants_to_jump = false; // 🚀 成功消費，絕殺重複跳躍與吞鍵！
        } else {
            // 每一 Tick 的垂直動量工整結算
            if keys.pressed(KeyCode::Space) {
                player.velocity.y += config.physics.water_buoyancy * dt; 
            }
            if keys.pressed(KeyCode::ShiftLeft) {
                player.velocity.y -= config.physics.water_buoyancy * dt;
            }
            // 扣除恆定的水中微弱重力（下沉）
            player.velocity.y -= config.physics.gravity * config.physics.water_gravity_multiplier * dt; 
        }
        
        // 套用高額的水體阻尼（固定乘法）
        let damp = (1.0 - config.physics.water_damping * dt).max(0.0);
        player.velocity.x *= damp;
        player.velocity.z *= damp;
        if !is_fluid_climbing && !is_surface_escape {
            player.velocity.y *= damp;
        }
    } else {
        // Instant horizontal speed on dry land (no lerp)
        player.velocity.x = input_dir.x * current_move_speed;
        player.velocity.z = input_dir.z * current_move_speed;
        
        // Dry land gravity
        player.velocity.y -= config.physics.gravity * dt;
    }

    // --- Jump (Normal Land) ---
    if (!foot_in_fluid || is_ground_jumping) && is_jumping_triggered && player.on_ground {
        player.velocity.y = config.physics.land_jump_impulse;
        player.on_ground = false;
        player.wants_to_jump = false; // 🚀 成功消費，絕殺重複跳躍與吞鍵！
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

    let mut is_colliding_horizontally = false;
    let was_on_ground = player.on_ground;

    // X axis
    if player.velocity.x != 0.0 {
        pos.x += player.velocity.x * dt;
        let hits = get_intersecting_blocks(pos);
        if !hits.is_empty() {
            is_colliding_horizontally = true;
            if player.velocity.x > 0.0 {
                let wall_x = hits.iter().map(|b| b.min.x).fold(f32::INFINITY, f32::min);
                pos.x = wall_x - player_radius - EPSILON;
            } else {
                let wall_x = hits.iter().map(|b| b.max.x).fold(f32::NEG_INFINITY, f32::max);
                pos.x = wall_x + player_radius + EPSILON;
            }
            player.velocity.x = 0.0;
        } else if player.is_crouching && was_on_ground {
            // 🚀 潛行邊緣防跌落安全鎖 (Safewalk)
            let mut fall_test = pos;
            fall_test.y -= 0.05;
            if get_intersecting_blocks(fall_test).is_empty() {
                pos.x -= player.velocity.x * dt;
                player.velocity.x = 0.0;
            }
        }
    }

    // Z axis
    if player.velocity.z != 0.0 {
        pos.z += player.velocity.z * dt;
        let hits = get_intersecting_blocks(pos);
        if !hits.is_empty() {
            is_colliding_horizontally = true;
            if player.velocity.z > 0.0 {
                let wall_z = hits.iter().map(|b| b.min.z).fold(f32::INFINITY, f32::min);
                pos.z = wall_z - player_radius - EPSILON;
            } else {
                let wall_z = hits.iter().map(|b| b.max.z).fold(f32::NEG_INFINITY, f32::max);
                pos.z = wall_z + player_radius + EPSILON;
            }
            player.velocity.z = 0.0;
        } else if player.is_crouching && was_on_ground {
            // 🚀 潛行邊緣防跌落安全鎖 (Safewalk)
            let mut fall_test = pos;
            fall_test.y -= 0.05;
            if get_intersecting_blocks(fall_test).is_empty() {
                pos.z -= player.velocity.z * dt;
                player.velocity.z = 0.0;
            }
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

    player.is_colliding_horizontally = is_colliding_horizontally;
    transform.translation = pos;

    // 如果這一 Tick 結束了，玩家人在乾燥陸地的空中，且沒有按住 Space，直接洗淨點擊鎖存
    if !player.on_ground && !foot_in_fluid && !keys.pressed(KeyCode::Space) {
        player.wants_to_jump = false;
    }
}


fn player_interaction(
    mut commands: Commands,
    mouse_keys: Res<ButtonInput<MouseButton>>,
    kbd_keys: Res<ButtonInput<KeyCode>>,
    mut world: ResMut<WorldManager>,
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
                    world.set_block_global(block_pos, BlockType::Air, &mut commands);
                    
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

                        let current_block = player.hotbar[player.selected_slot];
                        world.set_block_global(place_pos, current_block, &mut commands);
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

// 🚀 動態環境迷霧 + 遠剪裁面剛性對齊系統（純粹眼部位置感知）
fn update_fog_color(
    world_manager: Res<crate::world::WorldManager>,
    config: Res<crate::config::EngineConfig>,
    mut clear_color: ResMut<ClearColor>,
    mut q_fog: Query<&mut FogSettings, With<PlayerCamera>>,
    mut q_proj: Query<&mut Projection, With<PlayerCamera>>,
    q_camera: Query<&GlobalTransform, With<PlayerCamera>>,
) {
    let Ok(cam_tf) = q_camera.get_single() else { return; };
    let translation = cam_tf.translation();
    let eye_pos = IVec3::new(
        translation.x.floor() as i32,
        translation.y.floor() as i32,
        translation.z.floor() as i32,
    );
    let eye_light = world_manager.get_light_global(eye_pos);

    // 線性映射：眼部光照 0−15 → 地底深灰 → 蔚藍天空
    let t = (eye_light as f32) / 15.0;
    let sky = bevy::color::LinearRgba::new(0.5, 0.8, 1.0, 1.0);
    let dark_ambient_color = bevy::color::LinearRgba::gray(config.min_ambient_light);
    let mixed = dark_ambient_color.mix(&sky, t);
    let final_color = Color::from(mixed);

    clear_color.0 = final_color;

    // 防禦：render_distance 不得為 0
    if config.render_distance == 0 { return; }
    let max_distance = config.render_distance as f32 * 32.0; // 8 * 32 = 256.0

    // 【遠平面剛性鎖死】：獨立 query 確保不因 FogSettings 缺失而連帶失敗
    if let Ok(mut proj) = q_proj.get_single_mut() {
        if let Projection::Perspective(ref mut persp) = *proj {
            persp.far = max_distance + 64.0; // 256 + 64 = 320，給予充分幾何空間
        }
    }

    // 【原生迷霧阻斷】：黃金比例覆蓋地圖加載邊界
    if let Ok(mut fog) = q_fog.get_single_mut() {
        fog.color = final_color;
        fog.falloff = FogFalloff::Linear {
            start: max_distance * 0.75, // 192 格柔和起霧
            end:   max_distance - 8.0,  // 248 格完全消融遮擋地圖邊界
        };
    }
}

