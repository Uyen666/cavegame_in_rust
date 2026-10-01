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

pub mod vitals;
pub use vitals::PlayerVitals;

#[derive(Resource, Default)]
pub struct CursorJustLocked(pub bool);

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CursorJustLocked>()
           .add_systems(Startup, setup_player)
           .add_systems(
               Update,
               (
                   toggle_grab_cursor,
                   vitals::update_player_vitals_system,
                   player_interaction,
                   player_input_capture,
                   update_fog_color,
                   draw_target_block_highlight,
               )
                   .chain()
                   .run_if(in_state(crate::GameState::InGame))
           );
    }
}

fn player_input_capture(
    keys: Res<ButtonInput<KeyCode>>,
    mut scroll_evr: EventReader<bevy::input::mouse::MouseWheel>,
    mut q_player: Query<(&mut Player, &mut Inventory)>,
    q_windows: Query<&Window, With<PrimaryWindow>>,
    inv_state: Option<Res<crate::ui::inventory::InventoryScreenState>>,
) {
    if let Ok(window) = q_windows.get_single() {
        if window.cursor.grab_mode != CursorGrabMode::Locked {
            return;
        }
    }

    if let Some(ref state) = inv_state {
        if state.is_open {
            return;
        }
    }

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
    pub mining_target: Option<IVec3>, // 🚀 當前採掘方塊座標
    pub mining_progress: f32,          // 🚀 採掘進度 (0.0 ~ 1.0)
    pub mining_hit_timer: f32,         // 🚀 碎屑噴濺計時器
}

impl Default for Player {
    fn default() -> Self {
        Self {
            pitch: 0.0,
            yaw: 0.0,
            is_crouching: false,
            is_spectator: false,
            wants_to_jump: false,
            has_spawned: false,
            scroll_accumulator: 0.0,
            mining_target: None,
            mining_progress: 0.0,
            mining_hit_timer: 0.0,
        }
    }
}

#[derive(Component)]
pub struct PlayerCamera;

fn setup_player(
    mut commands: Commands,
    registry: Res<ItemRegistry>,
) {
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
        Player::default(),
        PlayerVitals::default(),
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
    mut q_windows: Query<(Entity, &mut Window), With<PrimaryWindow>>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse_btn: Res<ButtonInput<MouseButton>>,
    mut focus_events: EventReader<bevy::window::WindowFocused>,
    inv_state: Option<Res<crate::ui::inventory::InventoryScreenState>>,
    mut initial_grab_done: Local<bool>,
    mut cursor_just_locked: ResMut<CursorJustLocked>,
) {
    cursor_just_locked.0 = false;

    let Ok((window_entity, mut window)) = q_windows.get_single_mut() else { return; };

    // 檢查是否有視窗聚焦/失焦事件
    let mut gained_focus = false;
    let mut lost_focus = false;
    for ev in focus_events.read() {
        if ev.window == window_entity {
            if ev.focused {
                gained_focus = true;
            } else {
                lost_focus = true;
            }
        }
    }

    // 🚀 規則 1：視窗未獲焦點時絕對不鎖鼠標 (失去焦點或目前處於非聚焦狀態)
    // 若視窗未聚焦或剛失去焦點，游標無條件維持自由釋放狀態，絕不置中或鎖定！
    if !window.focused || lost_focus {
        if window.cursor.grab_mode != CursorGrabMode::None {
            window.cursor.grab_mode = CursorGrabMode::None;
        }
        if !window.cursor.visible {
            window.cursor.visible = true;
        }
        return;
    }

    let inv_is_open = inv_state.as_ref().map_or(false, |s| s.is_open);

    // 🚀 規則 2：聚焦時才置中鎖定 (開局首次聚焦就緒或重新獲得焦點時)
    let should_focus_lock = (!*initial_grab_done || gained_focus) && !inv_is_open;
    if should_focus_lock && window.width() > 0.0 && window.height() > 0.0 {
        let center = Vec2::new(window.width() / 2.0, window.height() / 2.0);
        window.set_cursor_position(Some(center));
        window.cursor.grab_mode = CursorGrabMode::Locked;
        window.cursor.visible = false;
        *initial_grab_done = true;
        cursor_just_locked.0 = true; // 避免切回視窗時的點擊誤破壞/放置方塊
        return;
    }

    if !*initial_grab_done && window.width() > 0.0 && window.height() > 0.0 {
        *initial_grab_done = true;
    }

    // 若背包開啟中，游標模式由背包 UI 系統全權託管
    if inv_is_open {
        return;
    }

    // 🚀 規則 3：按 ESC 鍵主動釋放游標脫離遊戲 (游標重獲自由並顯現)
    if keys.just_pressed(KeyCode::Escape) {
        window.cursor.grab_mode = CursorGrabMode::None;
        window.cursor.visible = true;
        return;
    }

    // 🚀 規則 4：在聚焦視窗內點擊滑鼠左鍵或右鍵，重新置中鎖定滑鼠
    if window.cursor.grab_mode != CursorGrabMode::Locked 
        && (mouse_btn.just_pressed(MouseButton::Left) || mouse_btn.just_pressed(MouseButton::Right)) 
    {
        let center = Vec2::new(window.width() / 2.0, window.height() / 2.0);
        window.set_cursor_position(Some(center));
        window.cursor.grab_mode = CursorGrabMode::Locked;
        window.cursor.visible = false;
        cursor_just_locked.0 = true; // 標記剛鎖定，防止此點擊誤觸方塊破壞或放置
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
        Option<&mut PlayerVitals>,
        Option<&Inventory>,
    )>,
    mut q_camera: Query<&mut Transform, (With<PlayerCamera>, Without<Player>)>,
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    world: Res<WorldManager>,
    config: Res<crate::config::EngineConfig>,
    inv_state: Option<Res<crate::ui::inventory::InventoryScreenState>>,
) {
    if let Some(ref state) = inv_state {
        if state.is_open {
            return;
        }
    }

    let (mut player, mut transform, mut vel, mut rb, mut collider, ground, fluid, mut maybe_vitals, maybe_inventory) = q_player.single_mut();
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
    
    let can_sprint = maybe_vitals.as_ref().map_or(true, |v| v.stamina > 2.0);
    let is_sprinting = keys.pressed(KeyCode::ShiftLeft) && can_sprint;
    let base_speed = if player.is_crouching { 
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

    let encumbrance = maybe_inventory.map_or(0.0, |inv| crate::player::vitals::calculate_inventory_encumbrance(inv));
    let enc_speed_mult = (1.0 - encumbrance * 0.25).max(0.6);
    let mut current_move_speed = base_speed * enc_speed_mult;
    let is_jumping_triggered = player.wants_to_jump || keys.pressed(KeyCode::Space);

    // --- Horizontal input ---
    let forward = Vec3::new(-player.yaw.sin(), 0.0, -player.yaw.cos());
    let right   = Vec3::new( player.yaw.cos(), 0.0, -player.yaw.sin());
    let mut input_dir = Vec3::ZERO;
    if keys.pressed(KeyCode::KeyW) { input_dir += forward; }
    if keys.pressed(KeyCode::KeyS) { input_dir -= forward; }
    if keys.pressed(KeyCode::KeyD) { input_dir += right; }
    if keys.pressed(KeyCode::KeyA) { input_dir -= right; }
    if input_dir.length_squared() > 0.0 { input_dir = input_dir.normalize(); }

    if fluid.in_fluid {
        // 🚀 水中移動邏輯：保持流體阻尼與慣性，疾跑時提升游泳速度
        let swim_speed_mult = if is_sprinting { 0.65 } else { 0.4 };
        current_move_speed *= swim_speed_mult;
        vel.x += input_dir.x * current_move_speed * 10.0 * dt;
        vel.z += input_dir.z * current_move_speed * 10.0 * dt;

        if ground.on_ground && is_jumping_triggered {
            // 🚀 水底起跳：從水底河床躍起，保持水阻慣性
            vel.y = config.physics.land_jump_impulse * 0.85;
            player.wants_to_jump = false;
            if let Some(ref mut vitals) = maybe_vitals {
                vitals.stamina = (vitals.stamina - 4.0).max(0.0);
            }
        } else if !fluid.head_in_fluid {
            // 🚀 水面狀態 (頭部已露於水面)
            let is_moving_forward = input_dir.length_squared() > 0.0;
            if rb.is_colliding_horizontally && is_moving_forward && is_jumping_triggered {
                // 🚀 登岸翻越 (Ledge Vault / Shore Hop)：前方有岸邊固體且嘗試翻上岸
                // 僅在尚未具備高額向上速度時賦予單次小衝量，徹底消除連續多影格灌值造成的彈簧床現象
                if vel.y < 2.0 {
                    vel.y = config.physics.land_jump_impulse * 0.75;
                    player.wants_to_jump = false;
                    if let Some(ref mut vitals) = maybe_vitals {
                        vitals.stamina = (vitals.stamina - 5.0).max(0.0);
                    }
                }
            } else if keys.pressed(KeyCode::Space) {
                // 🚀 水面踩水 (Treading Water)：平穩維持在水面，眼部保持在水線之上，不拋射
                if vel.y < 1.0 {
                    vel.y = (vel.y + 12.0 * dt).min(1.0);
                }
                player.wants_to_jump = false;
            }
        } else {
            // 🚀 水下完全潛浸狀態 (Head in fluid)
            if rb.is_colliding_horizontally && is_jumping_triggered {
                // 🚀 瀑布攀爬 / 貼壁上游：平穩向上游升，杜絕 8.5 m/s 火箭發射
                vel.y = (vel.y + 20.0 * dt).min(3.0);
                player.wants_to_jump = false;
            } else {
                if keys.pressed(KeyCode::Space) {
                    // 水中向上游
                    vel.y += config.physics.water_buoyancy * dt;
                }
                let is_diving = keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::KeyC);
                if is_diving {
                    // 🚀 水中下潛：統一與陸地下蹲鍵 (ControlLeft) 一致，並支援 C 鍵
                    vel.y -= config.physics.water_buoyancy * dt;
                }
            }
        }
    } else {
        // 陸地移動邏輯：無慣性瞬時水平速度
        vel.x = input_dir.x * current_move_speed;
        vel.z = input_dir.z * current_move_speed;

        // --- Jump (Normal Land) ---
        if is_jumping_triggered && ground.on_ground {
            vel.y = config.physics.land_jump_impulse;
            player.wants_to_jump = false; 
            if let Some(ref mut vitals) = maybe_vitals {
                vitals.stamina = (vitals.stamina - 6.0).max(0.0);
            }
        }
    }

    // 如果這一 Tick 結束了，且沒有按住 Space，直接洗淨點擊鎖存
    if !ground.on_ground && !fluid.in_fluid && !keys.pressed(KeyCode::Space) {
        player.wants_to_jump = false;
    }
}


fn player_interaction(
    mut commands: Commands,
    time: Res<Time>,
    mouse_keys: Res<ButtonInput<MouseButton>>,
    kbd_keys: Res<ButtonInput<KeyCode>>,
    mut world: ResMut<WorldManager>,
    registry: Res<ItemRegistry>,
    mut particle_mgr: ResMut<crate::render::particles::ParticleManager>,
    mut torch_burn_mgr: ResMut<crate::world::TorchBurnManager>,
    q_camera: Query<&GlobalTransform, With<PlayerCamera>>,
    q_windows: Query<&Window, With<PrimaryWindow>>,
    mut q_player: Query<(&Transform, &mut Player, &mut Inventory, Option<&mut PlayerVitals>)>,
    cursor_just_locked: Res<CursorJustLocked>,
) {
    let Ok(window) = q_windows.get_single() else { return; };
    if window.cursor.grab_mode != CursorGrabMode::Locked || cursor_just_locked.0 { return; }
    
    let Ok((player_transform, mut player, mut inventory, mut maybe_vitals)) = q_player.get_single_mut() else { return; };

    // 🚀 旁觀者權限閹割：禁止修改世界幾何
    if player.is_spectator {
        return; 
    }

    let left_holding = mouse_keys.pressed(MouseButton::Left);
    let right = mouse_keys.just_pressed(MouseButton::Right);
    let key_f = kbd_keys.just_pressed(KeyCode::KeyF);

    if !left_holding && !right && !key_f {
        player.mining_target = None;
        player.mining_progress = 0.0;
        player.mining_hit_timer = 0.0;
        return;
    }

    let mut hit_any_target = false;

    if let Ok(cam_transform) = q_camera.get_single() {
        let start = cam_transform.translation();
        let forward = cam_transform.forward();
        let max_dist = 5.0;

        if let Some(hit) = crate::utils::math::raycast_voxel(&world, start, *forward, max_dist) {
            hit_any_target = true;
            let block_pos = hit.block_pos;
            let place_pos = hit.adjacent_pos;

            if left_holding {
                let old_block = hit.block_type;
                let hardness = old_block.hardness();

                if hardness <= 0.0 {
                    // 🚀 零硬度方塊 (如火把)：直接秒碎
                    let color = crate::render::particles::get_block_debris_color(old_block);
                    particle_mgr.spawn_debris(block_pos.as_vec3() + 0.5, color, 6);

                    world.set_block_global(block_pos, BlockType::Air, &mut commands);
                    crate::world::fluid::wake_up_fluids_in_radius(&mut world, block_pos);
                    if old_block.is_torch() {
                        torch_burn_mgr.remove(block_pos);
                    }

                    if let Some(drop_item) = get_block_drop(old_block) {
                        inventory.add_item(ItemStack::new(drop_item, 1, &registry), &registry);
                    }
                    player.mining_target = None;
                    player.mining_progress = 0.0;
                    player.mining_hit_timer = 0.0;
                } else {
                    // 🚀 具備硬度方塊：累積採掘進度
                    if player.mining_target != Some(block_pos) {
                        player.mining_target = Some(block_pos);
                        player.mining_progress = 0.0;
                        player.mining_hit_timer = 0.0;
                    }

                    let mut speed_multiplier = 1.0;
                    let mut can_harvest = old_block.required_tier() == crate::world::registry::ToolTier::None;

                    if let Some(selected_item) = inventory.selected_item() {
                        if let Some(def) = registry.get(selected_item.item_type) {
                            if let ItemKind::Tool { tool_type, tier, efficiency, .. } = def.kind {
                                if tool_type == old_block.preferred_tool() {
                                    speed_multiplier = efficiency;
                                    if tier >= old_block.required_tier() {
                                        can_harvest = true;
                                    }
                                }
                            }
                        }
                    }

                    let break_time = if can_harvest {
                        old_block.hardness() * 1.5 / speed_multiplier
                    } else {
                        old_block.hardness() * 5.0 / speed_multiplier
                    };

                    let damage_rate = 1.0 / break_time.max(0.05);
                    let dt = time.delta_seconds().min(0.05);
                    player.mining_progress += damage_rate * dt;
                    player.mining_hit_timer += dt;

                    // 敲擊過程中每 0.18 秒飛散 2 顆細微碎屑
                    if player.mining_hit_timer >= 0.18 {
                        player.mining_hit_timer = 0.0;
                        let hit_color = crate::render::particles::get_block_debris_color(old_block);
                        particle_mgr.spawn_debris(block_pos.as_vec3() + 0.5, hit_color, 2);
                    }

                    if player.mining_progress >= 1.0 {
                        // 方塊徹底破碎！噴發 14 顆碎裂粒子
                        let burst_color = crate::render::particles::get_block_debris_color(old_block);
                        particle_mgr.spawn_debris(block_pos.as_vec3() + 0.5, burst_color, 14);

                        world.set_block_global(block_pos, BlockType::Air, &mut commands);
                        crate::world::fluid::wake_up_fluids_in_radius(&mut world, block_pos);
                        if old_block.is_torch() {
                            torch_burn_mgr.remove(block_pos);
                        }

                        if can_harvest {
                            if let Some(drop_item) = get_block_drop(old_block) {
                                inventory.add_item(ItemStack::new(drop_item, 1, &registry), &registry);
                            }
                        }

                        // 🚀 樹木重力與倒塌物理：若破壞原木，觸發 Timber 連鎖倒塌或重力滑落
                        if old_block == BlockType::OakLog {
                            crate::world::tree::handle_tree_break(
                                &mut world,
                                &mut commands,
                                &mut particle_mgr,
                                &mut inventory,
                                &registry,
                                block_pos,
                                can_harvest,
                            );
                        }

                        // 🚀 地質結構穩定度與礦坑坍塌物理：若在深層挖掘，檢查頂板跨度支護
                        crate::world::collapse::handle_mining_collapse(
                            &mut world,
                            &mut commands,
                            &mut particle_mgr,
                            maybe_vitals.as_deref_mut(),
                            player_transform.translation,
                            block_pos,
                            old_block,
                        );

                        inventory.damage_selected_tool(1);
                        player.mining_target = None;
                        player.mining_progress = 0.0;
                        player.mining_hit_timer = 0.0;
                    }
                }
            } else if right {
                // 🚀 生理口渴飲水交互：若對準水源或相鄰格為水，且未手持可放置方塊時，飲水補充水份
                let is_water_target = world.get_fluid_global(place_pos) > 0 || world.get_fluid_global(block_pos + IVec3::Y) > 0;
                let holding_placeable_block = inventory.selected_item().and_then(|it| registry.get(it.item_type)).map_or(false, |def| matches!(def.kind, ItemKind::Block(_)));

                if is_water_target && !holding_placeable_block {
                    if let Some(ref mut vitals) = maybe_vitals {
                        if vitals.thirst < vitals.max_thirst {
                            vitals.thirst = (vitals.thirst + 30.0).min(vitals.max_thirst);
                            vitals.wetness = (vitals.wetness + 0.15).min(1.0);
                            particle_mgr.spawn_debris(place_pos.as_vec3() + 0.5, Color::srgb_u8(60, 140, 240), 10);
                            println!("【生理系統】飲用水源！口渴度恢復至: {:.1}%", vitals.thirst);
                        }
                    }
                } else {
                    let block_aabb = Aabb::new(
                        Vec3::new(place_pos.x as f32, place_pos.y as f32, place_pos.z as f32),
                        Vec3::new(place_pos.x as f32 + 1.0, place_pos.y as f32 + 1.0, place_pos.z as f32 + 1.0),
                    );

                    let p_pos = player_transform.translation;
                    let player_aabb = Aabb::new(
                        Vec3::new(p_pos.x - 0.3, p_pos.y, p_pos.z - 0.3),
                        Vec3::new(p_pos.x + 0.3, p_pos.y + 1.8, p_pos.z + 0.3),
                    );

                    if !player_aabb.intersects(&block_aabb) {
                        if let Some(selected_item) = inventory.selected_item().cloned() {
                            if let Some(def) = registry.get(selected_item.item_type) {
                                if let ItemKind::Block(base_block) = def.kind {
                                    if selected_item.count > 0 {
                                        let mut current_block = base_block;
                                        if base_block == BlockType::Torch {
                                            let diff = hit.normal;
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
                                                // cannot place torch on ceiling
                                                current_block = BlockType::Air;
                                            }
                                        }

                                        if current_block != BlockType::Air {
                                            world.set_block_global(place_pos, current_block, &mut commands);
                                            crate::world::fluid::wake_up_fluids_in_radius(&mut world, place_pos);

                                            if current_block.is_torch() {
                                                torch_burn_mgr.register(place_pos, crate::world::torch::DEFAULT_TORCH_LIFETIME);
                                            }

                                            // 🚀 扣減 1 個物品 (數量降為 0 時自動置為 None)
                                            inventory.consume_selected(1);
                                        }
                                    }
                                } else if selected_item.item_type == ItemType::Flint {
                                    // 🚀 燧石擊打火花與火把引燃合成
                                    let spark_pos = hit.adjacent_pos.as_vec3() + 0.5;
                                    particle_mgr.spawn_debris(spark_pos, Color::srgb_u8(255, 190, 40), 6);

                                    // 若背包擁有 Stick 與 Coal，點火合成 4 支火把！
                                    if inventory.consume_item(ItemType::Stick, 1) {
                                        if inventory.consume_item(ItemType::Coal, 1) {
                                            inventory.add_item(ItemStack::new(ItemType::Torch, 4, &registry), &registry);
                                            particle_mgr.spawn_debris(spark_pos, Color::srgb_u8(255, 120, 20), 14);
                                        } else {
                                            // 煤炭不足時返還 Stick
                                            inventory.add_item(ItemStack::new(ItemType::Stick, 1, &registry), &registry);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            } else if key_f {
                println!("🚀 [Fluid Debug] F Key Pressed! Detecting raycast...");
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

        if !hit_any_target {
            player.mining_target = None;
            player.mining_progress = 0.0;
            player.mining_hit_timer = 0.0;

            if right {
                let p_feet = IVec3::new(
                    player_transform.translation.x.floor() as i32,
                    player_transform.translation.y.floor() as i32,
                    player_transform.translation.z.floor() as i32,
                );
                let in_water = world.get_fluid_global(p_feet) > 0 || world.get_fluid_global(p_feet + IVec3::Y) > 0;
                if in_water {
                    if let Some(ref mut vitals) = maybe_vitals {
                        if vitals.thirst < vitals.max_thirst {
                            vitals.thirst = (vitals.thirst + 30.0).min(vitals.max_thirst);
                            vitals.wetness = 1.0;
                            particle_mgr.spawn_debris(player_transform.translation, Color::srgb_u8(60, 140, 240), 10);
                            println!("【生理系統】於水中飲水！口渴度恢復至: {:.1}%", vitals.thirst);
                        }
                    }
                }
            }
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
    mut materials: ResMut<Assets<crate::render::material::VoxelMaterial>>,
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
    let fog_start = max_distance * 0.75; // 192 格柔和起霧
    let fog_end = max_distance - 8.0;   // 248 格完全消融遮擋地圖邊界

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
            start: fog_start,
            end:   fog_end,
        };
    }

    // 🚀 【GPU 著色器迷霧同步】：將背景色與相機座標直接推播至 WGSL 片元著色器
    let lin = final_color.to_linear();
    let fog_col = Vec4::new(lin.red, lin.green, lin.blue, 1.0);
    let cam_pos = Vec4::new(translation.x, translation.y, translation.z, 1.0);
    for (_, mat) in materials.iter_mut() {
        mat.env.fog_start = fog_start;
        mat.env.fog_end = fog_end;
        mat.env.fog_color = fog_col;
        mat.env.camera_pos = cam_pos;
    }
}

fn draw_target_block_highlight(
    q_camera: Query<&GlobalTransform, With<PlayerCamera>>,
    world: Res<WorldManager>,
    q_player: Query<&Player>,
    q_windows: Query<&Window, With<PrimaryWindow>>,
    mut gizmos: Gizmos,
) {
    if let Ok(window) = q_windows.get_single() {
        if window.cursor.grab_mode != CursorGrabMode::Locked {
            return;
        }
    }
    let Ok(player) = q_player.get_single() else { return; };
    if player.is_spectator { return; } // 旁觀者模式跳過
    
    let Ok(cam_transform) = q_camera.get_single() else { return; };
    let start = cam_transform.translation();
    let forward = cam_transform.forward();
    let max_dist = 5.0;

    // 🚀 執行 3D 快速體素遍歷 (Fast Voxel Traversal / 3D DDA)
    if let Some(hit) = crate::utils::math::raycast_voxel(&world, start, *forward, max_dist) {
        let (aabb_min, aabb_max) = hit.block_type.get_aabb_offsets();
        let origin = hit.block_pos.as_vec3();
        let box_min = origin + Vec3::from_array(aabb_min);
        let box_max = origin + Vec3::from_array(aabb_max);

        let center = (box_min + box_max) * 0.5;
        let size = (box_max - box_min) * 1.002;

        let is_mining = player.mining_target == Some(hit.block_pos) && player.mining_progress > 0.0;
        let border_color = if is_mining {
            Color::srgb(0.2 + player.mining_progress * 0.8, 0.2, 0.0)
        } else {
            Color::srgb(0.1, 0.1, 0.1)
        };

        gizmos.cuboid(
            Transform::from_translation(center).with_scale(size),
            border_color,
        );

        if is_mining {
            let inner_scale = size * (1.0 - player.mining_progress * 0.15);
            gizmos.cuboid(
                Transform::from_translation(center).with_scale(inner_scale),
                Color::srgb(1.0, 0.6, 0.1),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_world() -> (bevy::prelude::World, Entity) {
        let mut world = bevy::prelude::World::new();
        let mut time = Time::<()>::default();
        time.advance_by(std::time::Duration::from_millis(16));
        world.insert_resource(time);
        world.insert_resource(ButtonInput::<KeyCode>::default());
        world.insert_resource(crate::config::EngineConfig::default());
        world.insert_resource(WorldManager::default());

        let entity = world.spawn((
            Transform::from_xyz(0.0, 10.0, 0.0),
            crate::phys::components::Velocity::default(),
            crate::phys::components::AabbCollider::from_dimensions(0.6, 1.8),
            crate::phys::components::RigidBody {
                gravity_scale: 1.0,
                safewalk: false,
                is_kinematic: false,
                is_colliding_horizontally: false,
            },
            crate::phys::components::GroundSensor::default(),
            crate::phys::components::FluidSensor::default(),
            Player {
                has_spawned: true,
                ..default()
            },
            PlayerVitals::default(),
            Inventory::new(36),
        )).id();

        (world, entity)
    }

    #[test]
    fn test_water_surface_treading_no_launch() {
        let (mut world, entity) = create_test_world();
        
        // 設定為水面露頭狀態 (in_fluid = true, head_in_fluid = false, 不碰牆)
        {
            let mut fluid = world.get_mut::<crate::phys::components::FluidSensor>(entity).unwrap();
            fluid.in_fluid = true;
            fluid.head_in_fluid = false;
        }

        // 按住空白鍵
        {
            let mut keys = world.resource_mut::<ButtonInput<KeyCode>>();
            keys.press(KeyCode::Space);
        }

        // 執行 player_move 系統
        let mut schedule = Schedule::default();
        schedule.add_systems(player_move);
        schedule.run(&mut world);

        // 驗證向上速度被限制在平穩踩水上限 (<= 1.0)，絕對不准出現 8.5 的彈簧床拋射
        let vel = world.get::<crate::phys::components::Velocity>(entity).unwrap();
        assert!(vel.y > 0.0, "踩水應提供向上浮力");
        assert!(vel.y <= 1.0, "踩水速度不可超過 1.0 m/s，以防止彈簧床彈跳，目前值: {}", vel.y);
    }

    #[test]
    fn test_water_shore_hop_single_impulse() {
        let (mut world, entity) = create_test_world();
        
        // 設定為水面露頭且貼近岸邊障礙物
        {
            let mut fluid = world.get_mut::<crate::phys::components::FluidSensor>(entity).unwrap();
            fluid.in_fluid = true;
            fluid.head_in_fluid = false;
            let mut rb = world.get_mut::<crate::phys::components::RigidBody>(entity).unwrap();
            rb.is_colliding_horizontally = true;
        }

        // 按住 W 與 Space 嘗試翻上岸
        {
            let mut keys = world.resource_mut::<ButtonInput<KeyCode>>();
            keys.press(KeyCode::KeyW);
            keys.press(KeyCode::Space);
        }

        let mut schedule = Schedule::default();
        schedule.add_systems(player_move);

        // 影格 1：觸發登岸跳躍衝量
        schedule.run(&mut world);
        let vel_y1 = world.get::<crate::phys::components::Velocity>(entity).unwrap().y;
        assert!(vel_y1 >= 6.0, "登岸應觸發單次翻越衝量 (~6.38)，目前值: {}", vel_y1);

        // 影格 2：因為已經有向上速度 (vel.y >= 2.0)，不應再次重置注入衝量
        schedule.run(&mut world);
        let vel_y2 = world.get::<crate::phys::components::Velocity>(entity).unwrap().y;
        assert_eq!(vel_y1, vel_y2, "下一幀不應重複覆蓋衝量，應維持連續自然拋物線");
    }

    #[test]
    fn test_water_submerged_crouch_dive() {
        let (mut world, entity) = create_test_world();
        
        // 設定為完全潛水狀態 (in_fluid = true, head_in_fluid = true)
        {
            let mut fluid = world.get_mut::<crate::phys::components::FluidSensor>(entity).unwrap();
            fluid.in_fluid = true;
            fluid.head_in_fluid = true;
        }

        // 按住下蹲鍵 ControlLeft (或 C)
        {
            let mut keys = world.resource_mut::<ButtonInput<KeyCode>>();
            keys.press(KeyCode::ControlLeft);
        }

        let mut schedule = Schedule::default();
        schedule.add_systems(player_move);
        schedule.run(&mut world);

        // 驗證速度向下 (下潛)
        let vel = world.get::<crate::phys::components::Velocity>(entity).unwrap();
        assert!(vel.y < 0.0, "按下下蹲鍵應向下潛水，目前值: {}", vel.y);
    }

    #[test]
    fn test_progressive_mining_hardness_and_tool_efficiency() {
        let registry = ItemRegistry;
        let stone = BlockType::Stone;
        assert_eq!(stone.hardness(), 1.5);
        assert_eq!(stone.preferred_tool(), crate::world::registry::ToolType::Pickaxe);
        assert_eq!(stone.required_tier(), crate::world::registry::ToolTier::Wood);

        // 1. 空手 (無匹配工具) 對石頭：無法採掘 (can_harvest = false), break_time = 1.5 * 5.0 / 1.0 = 7.5s
        let hand_break_time = stone.hardness() * 5.0 / 1.0;
        assert_eq!(hand_break_time, 7.5);

        // 2. 木鎬 (Pickaxe, Wood tier, efficiency 2.0)：可採掘 (tier >= Wood), break_time = 1.5 * 1.5 / 2.0 = 1.125s
        let wood_pick = registry.get(ItemType::WoodenPickaxe).unwrap();
        if let ItemKind::Tool { efficiency, tier, tool_type, .. } = wood_pick.kind {
            assert_eq!(tool_type, stone.preferred_tool());
            assert!(tier >= stone.required_tier());
            let wood_break_time = stone.hardness() * 1.5 / efficiency;
            assert_eq!(wood_break_time, 1.125);
        } else {
            panic!("Expected tool");
        }

        // 3. 鐵鎬 (Pickaxe, Iron tier, efficiency 6.0)：可採掘 (tier >= Wood), break_time = 1.5 * 1.5 / 6.0 = 0.375s
        let iron_pick = registry.get(ItemType::IronPickaxe).unwrap();
        if let ItemKind::Tool { efficiency, tier, tool_type, .. } = iron_pick.kind {
            assert_eq!(tool_type, stone.preferred_tool());
            assert!(tier >= stone.required_tier());
            let iron_break_time = stone.hardness() * 1.5 / efficiency;
            assert_eq!(iron_break_time, 0.375);
        } else {
            panic!("Expected tool");
        }

        // 4. 火把 (硬度 0.0)：秒碎
        let torch = BlockType::Torch;
        assert_eq!(torch.hardness(), 0.0);
    }

    #[test]
    fn test_cursor_release_on_escape_and_relock_on_click() {
        let mut world = bevy::prelude::World::new();
        world.insert_resource(ButtonInput::<KeyCode>::default());
        world.insert_resource(ButtonInput::<MouseButton>::default());
        world.insert_resource(CursorJustLocked::default());
        world.insert_resource(Events::<bevy::window::WindowFocused>::default());

        let window_entity = world.spawn((
            Window {
                title: "Test Window".into(),
                resolution: (1280.0, 720.0).into(),
                cursor: bevy::window::Cursor {
                    grab_mode: CursorGrabMode::Locked,
                    visible: false,
                    ..default()
                },
                ..default()
            },
            PrimaryWindow,
        )).id();

        let mut schedule = Schedule::default();
        schedule.add_systems(toggle_grab_cursor);

        // 1. 初生影格：自動鎖定並置中 (640, 360)
        schedule.run(&mut world);
        let win = world.get::<Window>(window_entity).unwrap();
        assert_eq!(win.cursor.grab_mode, CursorGrabMode::Locked);
        assert!(!win.cursor.visible);
        assert_eq!(win.cursor_position(), Some(Vec2::new(640.0, 360.0)));

        // 2. 按下 ESC：釋放游標脫離視窗
        {
            let mut keys = world.resource_mut::<ButtonInput<KeyCode>>();
            keys.press(KeyCode::Escape);
        }
        schedule.run(&mut world);
        let win = world.get::<Window>(window_entity).unwrap();
        assert_eq!(win.cursor.grab_mode, CursorGrabMode::None, "按下 ESC 應釋放滑鼠游標");
        assert!(win.cursor.visible, "按下 ESC 應顯示滑鼠游標");

        // 清理 ESC 輸入
        {
            let mut keys = world.resource_mut::<ButtonInput<KeyCode>>();
            keys.release(KeyCode::Escape);
            keys.reset(KeyCode::Escape);
        }

        // 3. 點擊滑鼠左鍵：重新置中並鎖定滑鼠，且觸發 cursor_just_locked
        {
            let mut mouse = world.resource_mut::<ButtonInput<MouseButton>>();
            mouse.press(MouseButton::Left);
        }
        schedule.run(&mut world);
        let win = world.get::<Window>(window_entity).unwrap();
        assert_eq!(win.cursor.grab_mode, CursorGrabMode::Locked, "點擊左鍵應重新鎖定滑鼠");
        assert!(!win.cursor.visible);
        assert_eq!(win.cursor_position(), Some(Vec2::new(640.0, 360.0)), "重新鎖定時游標必須精確置中");
        assert!(world.resource::<CursorJustLocked>().0, "重新鎖定那一影格應標記 cursor_just_locked 防止誤觸方塊");

        // 4. 下一影格無點擊：cursor_just_locked 自動復位
        {
            let mut mouse = world.resource_mut::<ButtonInput<MouseButton>>();
            mouse.release(MouseButton::Left);
            mouse.reset(MouseButton::Left);
        }
        schedule.run(&mut world);
        assert!(!world.resource::<CursorJustLocked>().0, "次幀應清空 cursor_just_locked 恢復正常互動");
    }

    #[test]
    fn test_cursor_unfocused_never_locks_and_focus_locks() {
        let mut world = bevy::prelude::World::new();
        world.insert_resource(ButtonInput::<KeyCode>::default());
        world.insert_resource(ButtonInput::<MouseButton>::default());
        world.insert_resource(CursorJustLocked::default());
        world.insert_resource(Events::<bevy::window::WindowFocused>::default());

        // 初始狀態：視窗未獲得焦點 (focused: false)
        let window_entity = world.spawn((
            Window {
                title: "Test Window".into(),
                resolution: (1280.0, 720.0).into(),
                cursor: bevy::window::Cursor {
                    grab_mode: CursorGrabMode::None,
                    visible: true,
                    ..default()
                },
                focused: false,
                ..default()
            },
            PrimaryWindow,
        )).id();

        let mut schedule = Schedule::default();
        schedule.add_systems(toggle_grab_cursor);

        // 1. 視窗未獲焦點時：絕對不鎖定游標
        schedule.run(&mut world);
        let win = world.get::<Window>(window_entity).unwrap();
        assert_eq!(win.cursor.grab_mode, CursorGrabMode::None, "未聚焦時絕對不鎖定鼠標");
        assert!(win.cursor.visible, "未聚焦時游標必須可見");

        // 2. 未聚焦狀態下即使用戶點擊滑鼠左鍵，也絕不鎖定
        {
            let mut mouse = world.resource_mut::<ButtonInput<MouseButton>>();
            mouse.press(MouseButton::Left);
        }
        schedule.run(&mut world);
        let win = world.get::<Window>(window_entity).unwrap();
        assert_eq!(win.cursor.grab_mode, CursorGrabMode::None, "未聚焦時點擊滑鼠絕不能鎖定");
        assert!(win.cursor.visible);
        assert!(!world.resource::<CursorJustLocked>().0);

        {
            let mut mouse = world.resource_mut::<ButtonInput<MouseButton>>();
            mouse.release(MouseButton::Left);
            mouse.reset(MouseButton::Left);
        }

        // 3. 視窗獲得焦點 (聚焦)：立即置中並鎖定！
        {
            let mut win = world.get_mut::<Window>(window_entity).unwrap();
            win.focused = true;
            let mut events = world.resource_mut::<Events<bevy::window::WindowFocused>>();
            events.send(bevy::window::WindowFocused {
                window: window_entity,
                focused: true,
            });
        }
        schedule.run(&mut world);
        let win = world.get::<Window>(window_entity).unwrap();
        assert_eq!(win.cursor.grab_mode, CursorGrabMode::Locked, "聚焦時才置中鎖定");
        assert!(!win.cursor.visible, "鎖定時隱藏游標");
        assert_eq!(win.cursor_position(), Some(Vec2::new(640.0, 360.0)), "聚焦時精準置中");
        assert!(world.resource::<CursorJustLocked>().0, "聚焦時標記 cursor_just_locked 防止點擊誤觸");

        // 4. 視窗失去焦點 (失焦)：立即釋放游標！
        {
            let mut win = world.get_mut::<Window>(window_entity).unwrap();
            win.focused = false;
            let mut events = world.resource_mut::<Events<bevy::window::WindowFocused>>();
            events.send(bevy::window::WindowFocused {
                window: window_entity,
                focused: false,
            });
        }
        schedule.run(&mut world);
        let win = world.get::<Window>(window_entity).unwrap();
        assert_eq!(win.cursor.grab_mode, CursorGrabMode::None, "失焦時立即解鎖游標");
        assert!(win.cursor.visible, "失焦時游標恢復可見");
    }

    #[test]
    fn test_sprint_requires_stamina_and_jump_drains_stamina() {
        let (mut world, entity) = create_test_world();

        // 站在地面
        {
            let mut ground = world.get_mut::<crate::phys::components::GroundSensor>(entity).unwrap();
            ground.on_ground = true;
        }

        // 1. 滿體力時疾跑 (ShiftLeft + W)
        {
            let mut keys = world.resource_mut::<ButtonInput<KeyCode>>();
            keys.press(KeyCode::ShiftLeft);
            keys.press(KeyCode::KeyW);
        }

        let mut schedule = Schedule::default();
        schedule.add_systems(player_move);
        schedule.run(&mut world);

        let vel = world.get::<crate::phys::components::Velocity>(entity).unwrap();
        assert!((vel.z - (-5.6)).abs() < 0.05, "滿體力時疾跑速度應為 5.6 m/s，當前: {}", vel.z);

        // 2. 體力耗盡 (stamina = 0.0) 時無法疾跑，降回普通走速 (4.3 m/s)
        {
            let mut vitals = world.get_mut::<PlayerVitals>(entity).unwrap();
            vitals.stamina = 0.0;
        }
        schedule.run(&mut world);
        let vel2 = world.get::<crate::phys::components::Velocity>(entity).unwrap();
        assert!((vel2.z - (-4.3)).abs() < 0.05, "體力耗盡時疾跑失效降為普通速度 4.3 m/s，當前: {}", vel2.z);

        // 3. 跳躍扣減體力
        {
            let mut vitals = world.get_mut::<PlayerVitals>(entity).unwrap();
            vitals.stamina = 50.0;
            let mut player = world.get_mut::<Player>(entity).unwrap();
            player.wants_to_jump = true;
        }
        schedule.run(&mut world);
        let vitals = world.get::<PlayerVitals>(entity).unwrap();
        assert_eq!(vitals.stamina, 44.0, "地面起跳扣減 6.0 體力 (50 -> 44)");
    }

    #[test]
    fn test_inventory_encumbrance_slows_player() {
        let (mut world, entity) = create_test_world();
        let registry = ItemRegistry;

        // 填滿 6 組 64 個石頭 = 100% 負重
        {
            let mut inv = world.get_mut::<Inventory>(entity).unwrap();
            for i in 0..6 {
                inv.set_slot(i, Some(ItemStack::new(ItemType::Stone, 64, &registry)));
            }
            let mut keys = world.resource_mut::<ButtonInput<KeyCode>>();
            keys.press(KeyCode::KeyW);
        }

        let mut schedule = Schedule::default();
        schedule.add_systems(player_move);
        schedule.run(&mut world);

        // 基準走速 4.3 * (1.0 - 1.0 * 0.25) = 4.3 * 0.75 = 3.225
        let vel = world.get::<crate::phys::components::Velocity>(entity).unwrap();
        assert!((vel.z - (-3.225)).abs() < 0.05, "100% 負重時移速應減緩 25% (4.3 -> ~3.225)，當前: {}", vel.z);
    }
}


