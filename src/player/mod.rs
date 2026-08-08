use bevy::prelude::*;
use bevy::input::mouse::MouseMotion;
use bevy::window::{CursorGrabMode, PrimaryWindow};
use std::f32::consts::FRAC_PI_2;
use crate::world::{WorldManager, BlockType};
use crate::utils::math::Aabb;
use bevy::pbr::{FogSettings, FogFalloff};
use bevy::color::Mix;
use bevy::render::view::RenderLayers;
use crate::item::{Inventory, ItemStack, ItemType, ItemKind, ItemRegistry, get_block_drop};

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup_player)
           .add_systems(
               Update,
               (toggle_grab_cursor, player_interaction, player_input_capture, update_fog_color, draw_target_block_highlight)
                   .run_if(in_state(crate::GameState::InGame))
           );
    }
}

fn player_input_capture(
    keys: Res<ButtonInput<KeyCode>>,
    mut scroll_evr: EventReader<bevy::input::mouse::MouseWheel>,
    mut q_player: Query<(&mut Player, &mut Inventory)>,
) {
    if let Ok((mut player, mut inventory)) = q_player.get_single_mut() {
        if keys.just_pressed(KeyCode::Space) {
            player.wants_to_jump = true; // 🚀 鎖存點擊意圖
        }

        // --- 數字鍵切換 ---
        if keys.just_pressed(KeyCode::Digit1) { inventory.selected_slot = 0; }
        if keys.just_pressed(KeyCode::Digit2) { inventory.selected_slot = 1; }
        if keys.just_pressed(KeyCode::Digit3) { inventory.selected_slot = 2; }
        if keys.just_pressed(KeyCode::Digit4) { inventory.selected_slot = 3; }
        if keys.just_pressed(KeyCode::Digit5) { inventory.selected_slot = 4; }
        if keys.just_pressed(KeyCode::Digit6) { inventory.selected_slot = 5; }
        if keys.just_pressed(KeyCode::Digit7) { inventory.selected_slot = 6; }
        if keys.just_pressed(KeyCode::Digit8) { inventory.selected_slot = 7; }
        if keys.just_pressed(KeyCode::Digit9) { inventory.selected_slot = 8; }

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
            let new_slot = inventory.selected_slot as i32 - direction;
            inventory.selected_slot = ((new_slot % 9 + 9) % 9) as usize;

            // 消費掉這一次的整數能量，保留餘數給下一幀，達成極致絲滑的連續滾動
            player.scroll_accumulator -= direction as f32 * threshold;
        }
    }
}

#[derive(Component)]
pub struct Player {
    pub pitch: f32,
    pub yaw: f32,
    pub is_crouching: bool,
    pub is_spectator: bool,
    pub wants_to_jump: bool, // 🚀 跳躍輸入鎖存器（輸入緩衝）
    pub has_spawned: bool,        // 🚀 初次登入地表降落鎖
    pub scroll_accumulator: f32,  // 🚀 滾輪能量累加器
}

#[derive(Component)]
pub struct PlayerCamera;

fn setup_player(
    mut commands: Commands,
    mut q_windows: Query<&mut Window, With<PrimaryWindow>>,
    registry: Res<ItemRegistry>,
) {
    // Grab cursor
    if let Ok(mut window) = q_windows.get_single_mut() {
        window.cursor.grab_mode = CursorGrabMode::Locked;
        window.cursor.visible = false;
    }

    let mut inventory = Inventory::new(36);
    inventory.set_slot(0, Some(ItemStack::new(ItemType::Stone, 64, &registry)));
    inventory.set_slot(1, Some(ItemStack::new(ItemType::Dirt, 64, &registry)));
    inventory.set_slot(2, Some(ItemStack::new(ItemType::Grass, 64, &registry)));
    inventory.set_slot(3, Some(ItemStack::new(ItemType::OakLog, 64, &registry)));
    inventory.set_slot(4, Some(ItemStack::new(ItemType::OakLeaves, 64, &registry)));
    inventory.set_slot(5, Some(ItemStack::new(ItemType::Sand, 64, &registry)));
    inventory.set_slot(6, Some(ItemStack::new(ItemType::Glass, 64, &registry)));
    inventory.set_slot(7, Some(ItemStack::new(ItemType::Torch, 64, &registry)));
    inventory.set_slot(8, Some(ItemStack::new(ItemType::IronPickaxe, 1, &registry)));

    commands.spawn((
        Player {
            pitch: 0.0,
            yaw: 0.0,
            is_crouching: false,
            is_spectator: false,
            wants_to_jump: false,
            has_spawned: false,
            scroll_accumulator: 0.0,
        },
        inventory,
        crate::phys::components::RigidBody {
            gravity_scale: 1.0,
            safewalk: false,
            is_kinematic: false,
            is_colliding_horizontally: false,
        },
        crate::phys::components::AabbCollider::from_dimensions(0.6, 1.8),
        crate::phys::components::Velocity::default(),
        crate::phys::components::GroundSensor::default(),
        crate::phys::components::FluidSensor::default(),
        Transform::from_xyz(16.0, 250.0, 16.0), // 為了適應山脈地形，將初始高度拉到極限高空 (Y=250)，確保必定生於世界外表面，再利用重力自然降落
        GlobalTransform::default(),
        VisibilityBundle::default(),
    )).with_children(|parent| {
        parent.spawn((
            Camera3dBundle {
                transform: Transform::from_xyz(0.0, 1.6, 0.0),
                ..default()
            },
            RenderLayers::layer(0),
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

pub fn player_look(
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

pub fn player_move(
    mut q_player: Query<(
        &mut Player,
        &mut Transform,
        &mut crate::phys::components::Velocity,
        &mut crate::phys::components::RigidBody,
        &mut crate::phys::components::AabbCollider,
        &crate::phys::components::GroundSensor,
        &crate::phys::components::FluidSensor,
    )>,
    mut q_camera: Query<&mut Transform, (With<PlayerCamera>, Without<Player>)>,
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    world: Res<WorldManager>,
    config: Res<crate::config::EngineConfig>,
) {
    let (mut player, mut transform, mut vel, mut rb, mut collider, ground, fluid) = q_player.single_mut();
    let dt = time.delta_seconds();

    if dt < 0.0001 {
        return;
    }

    // ── F4 切換旁觀者模式 ──────────────
    if keys.just_pressed(KeyCode::F4) {
        player.is_spectator = !player.is_spectator;
        println!("【系統通知】玩家切換模式！旁觀者狀態: {}", player.is_spectator);
    }

    rb.is_kinematic = player.is_spectator;

    if player.is_spectator {
        vel.y = 0.0;
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
        return;
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
                vel.0 = Vec3::ZERO;
                println!("【系統通知】玩家已安全降落於地表: Y={}", surface_y);
            }
        }
        
        if !player.has_spawned {
            return;
        }
    }

    // --- Player dimensions ---
    player.is_crouching = keys.pressed(KeyCode::ControlLeft);
    rb.safewalk = player.is_crouching; // 🚀 同步 safewalk 到 RigidBody
    
    let player_height = if player.is_crouching { 1.5_f32 } else { 1.8_f32 };
    *collider = crate::phys::components::AabbCollider::from_dimensions(0.6, player_height);
    
    let is_sprinting = keys.pressed(KeyCode::ShiftLeft);
    let move_speed = if player.is_crouching { 
        2.5_f32 
    } else if is_sprinting {
        5.6_f32
    } else { 
        4.3_f32 
    };

    // --- Camera crouch lerp ---
    if let Ok(mut cam) = q_camera.get_single_mut() {
        let target_cam_y = if player.is_crouching { 1.2 } else { 1.6 };
        cam.translation.y += (target_cam_y - cam.translation.y) * (1.0 - (-10.0_f32 * dt).exp());
    }

    let mut current_move_speed = move_speed;
    let is_jumping_triggered = player.wants_to_jump || keys.pressed(KeyCode::Space);
    let is_ground_jumping = ground.on_ground && is_jumping_triggered;
    let is_fluid_climbing = fluid.in_fluid && rb.is_colliding_horizontally && is_jumping_triggered;
    let is_surface_escape = fluid.in_fluid && !fluid.head_in_fluid && is_jumping_triggered;

    // --- Horizontal input ---
    let forward = Vec3::new(-player.yaw.sin(), 0.0, -player.yaw.cos());
    let right   = Vec3::new( player.yaw.cos(), 0.0, -player.yaw.sin());
    let mut input_dir = Vec3::ZERO;
    if keys.pressed(KeyCode::KeyW) { input_dir += forward; }
    if keys.pressed(KeyCode::KeyS) { input_dir -= forward; }
    if keys.pressed(KeyCode::KeyD) { input_dir += right; }
    if keys.pressed(KeyCode::KeyA) { input_dir -= right; }
    if input_dir.length_squared() > 0.0 { input_dir = input_dir.normalize(); }

    if fluid.in_fluid && !is_ground_jumping {
        current_move_speed *= 0.4;
        vel.x += input_dir.x * current_move_speed * 10.0 * dt;
        vel.z += input_dir.z * current_move_speed * 10.0 * dt;

        if is_fluid_climbing || is_surface_escape {
            vel.y = config.physics.land_jump_impulse;
            player.wants_to_jump = false;
        } else {
            if keys.pressed(KeyCode::Space) {
                vel.y += config.physics.water_buoyancy * dt; 
            }
            if keys.pressed(KeyCode::ShiftLeft) {
                vel.y -= config.physics.water_buoyancy * dt;
            }
            // Gravity & Damping in fluid are handled by PhysicsPlugin (apply_kinematics)
        }
    } else {
        // Instant horizontal speed on dry land
        vel.x = input_dir.x * current_move_speed;
        vel.z = input_dir.z * current_move_speed;
        // Gravity on land is handled by PhysicsPlugin
    }

    // --- Jump (Normal Land) ---
    if (!fluid.in_fluid || is_ground_jumping) && is_jumping_triggered && ground.on_ground {
        vel.y = config.physics.land_jump_impulse;
        player.wants_to_jump = false; 
    }

    // 如果這一 Tick 結束了，且沒有按住 Space，直接洗淨點擊鎖存
    if !ground.on_ground && !fluid.in_fluid && !keys.pressed(KeyCode::Space) {
        player.wants_to_jump = false;
    }
}


fn player_interaction(
    mut commands: Commands,
    mouse_keys: Res<ButtonInput<MouseButton>>,
    kbd_keys: Res<ButtonInput<KeyCode>>,
    mut world: ResMut<WorldManager>,
    registry: Res<ItemRegistry>,
    q_camera: Query<&GlobalTransform, With<PlayerCamera>>,
    q_windows: Query<&Window, With<PrimaryWindow>>,
    mut q_player: Query<(&Transform, &Player, &mut Inventory)>,
) {
    let left = mouse_keys.just_pressed(MouseButton::Left);
    let right = mouse_keys.just_pressed(MouseButton::Right);
    let key_f = kbd_keys.just_pressed(KeyCode::KeyF);

    if !left && !right && !key_f {
        return;
    }
    
    let Ok(window) = q_windows.get_single() else { return; };
    if window.cursor.grab_mode != CursorGrabMode::Locked { return; }
    
    let Ok((player_transform, player, mut inventory)) = q_player.get_single_mut() else { return; };

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

            let mut hit_aabb = false;
            let block = world.get_block_global(block_pos);
            if block.is_solid() || block.is_torch() {
                let (aabb_min, aabb_max) = block.get_aabb_offsets();
                let origin = Vec3::new(block_pos.x as f32, block_pos.y as f32, block_pos.z as f32);
                let box_min = origin + Vec3::from_array(aabb_min);
                let box_max = origin + Vec3::from_array(aabb_max);
                
                if pos.x >= box_min.x && pos.x <= box_max.x &&
                   pos.y >= box_min.y && pos.y <= box_max.y &&
                   pos.z >= box_min.z && pos.z <= box_max.z {
                    hit_aabb = true;
                }
            }

            if hit_aabb {
                if left {
                    let old_block = world.get_block_global(block_pos);
                    world.set_block_global(block_pos, BlockType::Air, &mut commands);
                    
                    // 【流體聯鎖喚醒機制】(Fluid Block Update Hook)
                    crate::world::fluid::wake_up_fluids_in_radius(&mut world, block_pos);

                    // 🚀 直接自動入包 (Direct Auto-Pickup)
                    if let Some(drop_item) = get_block_drop(old_block) {
                        inventory.add_item(ItemStack::new(drop_item, 1, &registry), &registry);
                    }

                    // 🚀 若手持工具，扣減耐久度 1 (耐久歸零時自動碎裂清空為 None)
                    inventory.damage_selected_tool(1);
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

                        if let Some(selected_item) = inventory.selected_item().cloned() {
                            if let Some(def) = registry.get(selected_item.item_type) {
                                if let ItemKind::Block(base_block) = def.kind {
                                    if selected_item.count > 0 {
                                        let mut current_block = base_block;
                                        if base_block == BlockType::Torch {
                                            let diff = place_pos - block_pos;
                                            if diff == IVec3::Y {
                                                current_block = BlockType::Torch;
                                            } else if diff == IVec3::X {
                                                current_block = BlockType::TorchWallW;
                                            } else if diff == IVec3::NEG_X {
                                                current_block = BlockType::TorchWallE;
                                            } else if diff == IVec3::Z {
                                                current_block = BlockType::TorchWallN;
                                            } else if diff == IVec3::NEG_Z {
                                                current_block = BlockType::TorchWallS;
                                            } else if diff == IVec3::NEG_Y {
                                                break; // cannot place torch on ceiling
                                            }
                                        }

                                        world.set_block_global(place_pos, current_block, &mut commands);
                                        crate::world::fluid::wake_up_fluids_in_radius(&mut world, place_pos);

                                        // 🚀 扣減 1 個物品 (數量降為 0 時自動置為 None)
                                        inventory.consume_selected(1);
                                    }
                                }
                            }
                        }
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
    cycle: Res<crate::world::DayNightCycle>,
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
    let sky_light = world_manager.get_sky_light_global(eye_pos) as f32;
    let block_light = world_manager.get_block_light_global(eye_pos) as f32;
    let eye_light = (sky_light * cycle.sky_factor).max(block_light);

    // 線性映射：眼部光照 0−15 → 地底深灰 → 蔚藍/暗夜天空
    let t = eye_light / 15.0;
    let day_sky = bevy::color::LinearRgba::new(0.5, 0.8, 1.0, 1.0);
    let night_sky = bevy::color::LinearRgba::new(0.01, 0.02, 0.08, 1.0);
    let current_sky = night_sky.mix(&day_sky, cycle.sky_factor);
    
    let dark_ambient_color = bevy::color::LinearRgba::gray(config.min_ambient_light);
    let mixed = dark_ambient_color.mix(&current_sky, t);
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

fn draw_target_block_highlight(
    q_camera: Query<&GlobalTransform, With<PlayerCamera>>,
    world: Res<WorldManager>,
    q_player: Query<&Player>,
    mut gizmos: Gizmos,
) {
    let Ok(player) = q_player.get_single() else { return; };
    if player.is_spectator { return; } // 旁觀者模式跳過
    
    let Ok(cam_transform) = q_camera.get_single() else { return; };
    let start = cam_transform.translation();
    let forward = cam_transform.forward();
    let max_dist = 5.0;

    // 🚀 執行 3D 體素射線步進 (Raycast)
    let mut dist = 0.0;
    let step = 0.05;
    while dist < max_dist {
        let pos = start + forward * dist;
        let block_pos = IVec3::new(pos.x.floor() as i32, pos.y.floor() as i32, pos.z.floor() as i32);
        
        let mut hit_aabb = false;
        let mut box_min = Vec3::ZERO;
        let mut box_max = Vec3::ZERO;
        
        let target_block = world.get_block_global(block_pos);
        if target_block.is_solid() || target_block.is_torch() {
            let (aabb_min, aabb_max) = target_block.get_aabb_offsets();
            let origin = Vec3::new(block_pos.x as f32, block_pos.y as f32, block_pos.z as f32);
            box_min = origin + Vec3::from_array(aabb_min);
            box_max = origin + Vec3::from_array(aabb_max);
            
            if pos.x >= box_min.x && pos.x <= box_max.x &&
               pos.y >= box_min.y && pos.y <= box_max.y &&
               pos.z >= box_min.z && pos.z <= box_max.z {
                hit_aabb = true;
            }
        }

        if hit_aabb {
            let center = (box_min + box_max) * 0.5;
            let size = (box_max - box_min) * 1.002;
            
            gizmos.cuboid(
                Transform::from_translation(center).with_scale(size),
                Color::srgb(0.1, 0.1, 0.1),
            );
            break; // 找到第一個固體方塊且擊中 AABB 即可停手
        }
        dist += step;
    }
}
