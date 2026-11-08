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

/// 螢幕與相機震顫資源 (Screen Shake Trauma System)
/// 用於地質坍塌前兆、倒木砸地等劇烈物理事件的視覺反饋
#[derive(Resource, Default, Debug, Clone)]
pub struct ScreenShake {
    pub trauma: f32, // 0.0 ~ 1.0 (非線性平方衰減)
}

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CursorJustLocked>()
           .init_resource::<ScreenShake>()
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
    knapping_state: Option<Res<crate::ui::knapping::KnappingState>>,
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

    if let Some(ref state) = knapping_state {
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

impl Player {
    #[inline]
    pub fn reset_mining(&mut self) {
        self.mining_target = None;
        self.mining_progress = 0.0;
        self.mining_hit_timer = 0.0;
    }

    #[inline]
    pub fn start_mining(&mut self, pos: IVec3) {
        self.mining_target = Some(pos);
        self.mining_progress = 0.0;
        self.mining_hit_timer = 0.0;
    }
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
    // 🚀 快捷列 (Hotbar 0..9)：開局直接配置燧石與木棍，打通 Knapping 切削測試與生存起步
    inventory.set_slot(0, Some(ItemStack::new(ItemType::Flint, 4, &registry)));
    inventory.set_slot(1, Some(ItemStack::new(ItemType::Stick, 4, &registry)));
    inventory.set_slot(2, Some(ItemStack::new(ItemType::OakLog, 16, &registry)));
    inventory.set_slot(3, Some(ItemStack::new(ItemType::Torch, 16, &registry)));
    inventory.set_slot(4, Some(ItemStack::new(ItemType::Dirt, 64, &registry)));
    inventory.set_slot(5, Some(ItemStack::new(ItemType::Stone, 64, &registry)));
    inventory.set_slot(6, Some(ItemStack::new(ItemType::Sand, 64, &registry)));
    inventory.set_slot(7, Some(ItemStack::new(ItemType::OakLeaves, 64, &registry)));
    inventory.set_slot(8, Some(ItemStack::new(ItemType::Glass, 64, &registry)));

    // 🚀 主存儲區 (Main Storage 9..35)：放置備用高階工具與建築建材
    inventory.set_slot(9, Some(ItemStack::new(ItemType::IronAxe, 1, &registry)));
    inventory.set_slot(10, Some(ItemStack::new(ItemType::IronPickaxe, 1, &registry)));

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
        Transform::from_xyz(16.0, (crate::utils::math::WORLD_MAX_Y - 4) as f32, 16.0), // 為了適應 1024 宏觀山脈地形，初始高度對齊大氣層頂部，確保必定生於世界外表面，再安全降落地表
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
    knapping_state: Option<Res<crate::ui::knapping::KnappingState>>,
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
    let knapping_is_open = knapping_state.as_ref().map_or(false, |s| s.is_open);
    let ui_is_open = inv_is_open || knapping_is_open;

    // 🚀 規則 2：聚焦時才置中鎖定 (開局首次聚焦就緒或重新獲得焦點時)
    let should_focus_lock = (!*initial_grab_done || gained_focus) && !ui_is_open;
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

    // 若 UI（背包或 Knapping）開啟中，游標模式由 UI 系統全權託管
    if ui_is_open {
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
    time: Res<Time>,
    mut q_player: Query<&mut Player>,
    mut q_camera: Query<&mut Transform, With<PlayerCamera>>,
    mut mouse_motion_events: EventReader<MouseMotion>,
    q_windows: Query<&Window, With<PrimaryWindow>>,
    mut shake: Option<ResMut<ScreenShake>>,
    knapping_state: Option<Res<crate::ui::knapping::KnappingState>>,
) {
    if let Some(ref state) = knapping_state {
        if state.is_open {
            return;
        }
    }

    let Ok(window) = q_windows.get_single() else {
        return;
    };

    let mut player = q_player.single_mut();
    let mut camera_transform = q_camera.single_mut();

    if window.cursor.grab_mode == CursorGrabMode::Locked {
        let mut delta = Vec2::ZERO;
        for event in mouse_motion_events.read() {
            delta += event.delta;
        }

        if delta != Vec2::ZERO {
            let sensitivity = 0.002;
            player.yaw -= delta.x * sensitivity;
            player.pitch -= delta.y * sensitivity;
            player.pitch = player.pitch.clamp(-FRAC_PI_2 + 0.01, FRAC_PI_2 - 0.01);
        }
    }

    let base_rot = Quat::from_axis_angle(Vec3::Y, player.yaw)
        * Quat::from_axis_angle(Vec3::X, player.pitch);

    let dt = time.delta_seconds();
    if let Some(ref mut s) = shake {
        if s.trauma > 0.0 {
            s.trauma = (s.trauma - dt * 1.5).max(0.0);
            let t = time.elapsed_seconds();
            let shake_yaw = (t * 47.0).sin() * s.trauma * 0.02;
            let shake_pitch = (t * 53.0).cos() * s.trauma * 0.02;
            let shake_roll = (t * 39.0).sin() * s.trauma * 0.015;
            camera_transform.rotation = base_rot * Quat::from_euler(EulerRot::YXZ, shake_yaw, shake_pitch, shake_roll);

            let shake_offset = Vec3::new((t * 57.0).sin(), (t * 43.0).cos(), 0.0) * (s.trauma * 0.04);
            camera_transform.translation = Vec3::new(0.0, 1.6, 0.0) + shake_offset;
            return;
        }
    }

    camera_transform.rotation = base_rot;
    camera_transform.translation = Vec3::new(0.0, 1.6, 0.0);
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
        println!("[System] Player mode switched! Spectator: {}", player.is_spectator);
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
        for cy in (0..crate::utils::math::WORLD_CHUNKS_Y).rev() {
            if world.get_chunk_ref(IVec3::new(cx, cy, cz)).is_some() {
                top_loaded_cy = Some(cy);
                break;
            }
        }
        
        if top_loaded_cy.is_some() {
            let mut surface_found = false;
            let mut surface_y = (crate::utils::math::WORLD_MAX_Y - 4) as f32;
            let px = transform.translation.x.floor() as i32;
            let pz = transform.translation.z.floor() as i32;
            for y in (0..crate::utils::math::WORLD_MAX_Y).rev() {
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
                println!("[System] Player safely spawned on surface: Y={}", surface_y);
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

#[inline]
fn resolve_torch_placement(normal: IVec3) -> BlockType {
    if normal == IVec3::Y {
        BlockType::Torch
    } else if normal == IVec3::X {
        BlockType::TorchWallW
    } else if normal == IVec3::NEG_X {
        BlockType::TorchWallE
    } else if normal == IVec3::Z {
        BlockType::TorchWallN
    } else if normal == IVec3::NEG_Z {
        BlockType::TorchWallS
    } else {
        BlockType::Air
    }
}

#[inline]
fn enqueue_fluid_neighbors(world: &mut WorldManager, pos: IVec3) {
    world.fluid_queue.push_back(pos);
    for dir in [IVec3::Y, IVec3::NEG_Y, IVec3::X, IVec3::NEG_X, IVec3::Z, IVec3::NEG_Z] {
        world.fluid_queue.push_back(pos + dir);
    }
}

fn handle_flint_spark_crafting(
    spark_pos: Vec3,
    inventory: &mut Inventory,
    registry: &ItemRegistry,
    particle_mgr: &mut crate::render::particles::ParticleManager,
) {
    particle_mgr.spawn_debris(spark_pos, Color::srgb_u8(255, 190, 40), 6);
    if inventory.consume_item(ItemType::Stick, 1) {
        if inventory.consume_item(ItemType::Coal, 1) {
            inventory.add_item(ItemStack::new(ItemType::Torch, 4, registry), registry);
            particle_mgr.spawn_debris(spark_pos, Color::srgb_u8(255, 120, 20), 14);
        } else {
            inventory.add_item(ItemStack::new(ItemType::Stick, 1, registry), registry);
        }
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
    mut q_primary_window: Query<&mut Window, With<PrimaryWindow>>,
    mut q_player: Query<(&Transform, &mut Player, &mut Inventory, Option<&mut PlayerVitals>)>,
    mut meshes: ResMut<Assets<Mesh>>,
    gt: Res<crate::render::textures::GameTextures>,
    cursor_just_locked: Res<CursorJustLocked>,
    mut knapping_state: Option<ResMut<crate::ui::knapping::KnappingState>>,
    mut q_knapping_root: Query<&mut Visibility, (With<crate::ui::knapping::KnappingRootNode>, Without<crate::ui::inventory::InventoryRootNode>)>,
) {
    let Ok(mut window) = q_primary_window.get_single_mut() else { return; };
    if window.cursor.grab_mode != CursorGrabMode::Locked || cursor_just_locked.0 { return; }
    
    let Ok((player_transform, mut player, mut inventory, mut maybe_vitals)) = q_player.get_single_mut() else { return; };

    // 🚀 旁觀者權限閹割：禁止修改世界幾何
    if player.is_spectator {
        return; 
    }

    let left_holding = mouse_keys.pressed(MouseButton::Left);
    let right = mouse_keys.just_pressed(MouseButton::Right);
    let key_f = kbd_keys.just_pressed(KeyCode::KeyF);
    let key_q = kbd_keys.just_pressed(KeyCode::KeyQ);

    // 🚀 手動丟棄物品 (Q 鍵)
    if key_q {
        if let Some(selected) = inventory.selected_item().cloned() {
            if selected.count > 0 {
                inventory.consume_selected(1);
                let drop_stack = ItemStack::new(selected.item_type, 1, &registry);
                if let Ok(cam_transform) = q_camera.get_single() {
                    let forward = *cam_transform.forward();
                    let spawn_p = cam_transform.translation() + forward * 0.8;
                    let throw_vel = forward * 5.0 + Vec3::new(0.0, 2.0, 0.0);
                    if gt.ready {
                        crate::item::spawn_item_drop(
                            &mut commands,
                            &mut *meshes,
                            gt.material.clone(),
                            drop_stack,
                            spawn_p,
                            throw_vel,
                            Some(&world),
                        );
                    }
                }
            }
        }
    }

    // 🚀 手持各類燧石工具頭部 (Flint Tool Heads) 右鍵組裝工具，或手持雙燧石開啟切削 (Knapping)
    if right {
        if let Some(selected) = inventory.selected_item().cloned() {
            let tool_assembly = match selected.item_type {
                ItemType::FlintAxeHead => Some((
                    ItemType::FlintAxe,
                    "Flint Axe",
                    "You can now fell trees to trigger falling tree physics and harvest logs!",
                )),
                ItemType::FlintPickaxeHead => Some((
                    ItemType::FlintPickaxe,
                    "Flint Pickaxe",
                    "You can now mine solid stone into cobblestone and excavate coal ore!",
                )),
                ItemType::FlintShovelHead => Some((
                    ItemType::FlintShovel,
                    "Flint Shovel",
                    "You can now excavate soil and gravel rapidly!",
                )),
                ItemType::FlintKnifeBlade => Some((
                    ItemType::FlintKnife,
                    "Flint Knife",
                    "You can now harvest foliage and plant fibers swiftly!",
                )),
                _ => None,
            };

            if let Some((tool_type, tool_name, usage_tip)) = tool_assembly {
                if inventory.consume_item(ItemType::Stick, 1) {
                    inventory.consume_selected(1);
                    let tool = ItemStack::new(tool_type, 1, &registry);
                    let max_dur = tool.max_durability(&registry).unwrap_or(40);
                    if let Some(overflow) = inventory.add_item(tool, &registry) {
                        println!("[Crafting] Inventory full! Overflowed item: {:?}", overflow);
                    }
                    if let Ok(cam_transform) = q_camera.get_single() {
                        let forward = *cam_transform.forward();
                        let spark_pos = cam_transform.translation() + forward * 0.8;
                        particle_mgr.spawn_debris(spark_pos, Color::srgb(0.95, 0.85, 0.45), 14);
                        particle_mgr.spawn_debris(spark_pos, Color::srgb(0.55, 0.40, 0.25), 8);
                    }
                    println!("[Crafting] Successfully assembled {}! Durability: {}/{}", tool_name, max_dur, max_dur);
                    println!("[Tip] {}", usage_tip);
                    return;
                } else {
                    println!("[Crafting] Missing Stick! Break leaves to collect 1 Stick to assemble {}!", tool_name);
                    return;
                }
            } else if selected.item_type == ItemType::Flint && selected.count >= 2 {
                if let Some(ref mut k_state) = knapping_state {
                    k_state.is_open = true;
                    k_state.reset_grid();
                    if let Ok(mut vis) = q_knapping_root.get_single_mut() {
                        *vis = Visibility::Inherited;
                    }
                    window.cursor.grab_mode = CursorGrabMode::None;
                    window.cursor.visible = true;
                    println!("[Knapping] Opened 5x5 Knapping interface! Click stone flakes to shape tool heads!");
                    return;
                }
            }
        }
    }

    if !left_holding && !right && !key_f {
        player.reset_mining();
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
                        if gt.ready {
                            let drop_pos = block_pos.as_vec3() + Vec3::new(0.5, 0.5, 0.5);
                            let angle = (block_pos.x.wrapping_mul(37) ^ block_pos.z.wrapping_mul(17) ^ block_pos.y) as f32;
                            let drop_vel = Vec3::new(angle.cos() * 1.2, 2.8, angle.sin() * 1.2);
                            crate::item::spawn_item_drop(
                                &mut commands,
                                &mut *meshes,
                                gt.material.clone(),
                                ItemStack::new(drop_item, 1, &registry),
                                drop_pos,
                                drop_vel,
                                Some(&world),
                            );
                        } else {
                            inventory.add_item(ItemStack::new(drop_item, 1, &registry), &registry);
                        }
                    }
                    player.reset_mining();
                } else {
                    // 🚀 具備硬度方塊：累積採掘進度
                    if player.mining_target != Some(block_pos) {
                        player.start_mining(block_pos);
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
                            // 注意：OakLog 的掉落物統一由 handle_tree_break 處理（整棵樹傾倒或單塊木頭收穫），避免在原地提前生成
                            if old_block != BlockType::OakLog {
                                if let Some(drop_item) = get_block_drop(old_block) {
                                    if gt.ready {
                                        let drop_pos = block_pos.as_vec3() + Vec3::new(0.5, 0.5, 0.5);
                                        let angle = (block_pos.x.wrapping_mul(37) ^ block_pos.z.wrapping_mul(17) ^ block_pos.y) as f32;
                                        let drop_vel = Vec3::new(angle.cos() * 1.2, 2.8, angle.sin() * 1.2);
                                        crate::item::spawn_item_drop(
                                            &mut commands,
                                            &mut *meshes,
                                            gt.material.clone(),
                                            ItemStack::new(drop_item, 1, &registry),
                                            drop_pos,
                                            drop_vel,
                                            Some(&world),
                                        );
                                    } else {
                                        inventory.add_item(ItemStack::new(drop_item, 1, &registry), &registry);
                                    }
                                }

                                // 🚀 野外淘石機制：空手或鏟子挖掘泥土/草地時，有 15% 機率翻找出一顆燧石 (Flint)
                                let flint_chance = ((block_pos.x.wrapping_mul(73) ^ block_pos.z.wrapping_mul(31) ^ block_pos.y) & 0xFF) as f32 / 255.0;
                                if (old_block == BlockType::Dirt || old_block == BlockType::Grass) && flint_chance < 0.15 {
                                    if gt.ready {
                                        let flint_pos = block_pos.as_vec3() + Vec3::new(0.5, 0.5, 0.5);
                                        let flint_vel = Vec3::new(0.5, 2.2, -0.5);
                                        crate::item::spawn_item_drop(
                                            &mut commands,
                                            &mut *meshes,
                                            gt.material.clone(),
                                            ItemStack::new(ItemType::Flint, 1, &registry),
                                            flint_pos,
                                            flint_vel,
                                            Some(&world),
                                        );
                                    } else {
                                        inventory.add_item(ItemStack::new(ItemType::Flint, 1, &registry), &registry);
                                    }
                                }
                            }
                        } else {
                            let tool_name = match old_block.preferred_tool() {
                                crate::world::registry::ToolType::Axe => "Axe",
                                crate::world::registry::ToolType::Pickaxe => "Pickaxe",
                                crate::world::registry::ToolType::Shovel => "Shovel",
                                _ => "appropriate tool",
                            };
                            println!("[Harvest] Insufficient tool tier or not holding an {} to harvest {}!", tool_name, old_block.name());
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
                                Some(player_transform.translation),
                                if gt.ready { Some((&mut *meshes, gt.material.clone())) } else { None },
                            );
                        }

                        // 🚀 地質結構穩定度與礦坑坍塌物理：若在深層挖掘，檢查頂板跨度支護
                        crate::world::collapse::handle_mining_collapse(
                            &world,
                            &mut commands,
                            &mut particle_mgr,
                            block_pos,
                            old_block,
                        );

                        // 🚀 鬆散土壤自然滑坡與重力喚醒 (Sand / Gravel Angle of Repose)
                        crate::world::soil::wake_up_soils_in_radius(
                            &mut world,
                            &mut commands,
                            &mut particle_mgr,
                            block_pos,
                        );

                        inventory.damage_selected_tool(1);
                        player.reset_mining();
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
                            println!("[Vitals] Drank water! Thirst restored to: {:.1}%", vitals.thirst);
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
                                        let current_block = if base_block == BlockType::Torch {
                                            resolve_torch_placement(hit.normal)
                                        } else {
                                            base_block
                                        };

                                        if current_block != BlockType::Air {
                                            world.set_block_global(place_pos, current_block, &mut commands);
                                            crate::world::fluid::wake_up_fluids_in_radius(&mut world, place_pos);

                                            // 🚀 鬆散土壤放置時即時結算垂直重力與安息角滑坡
                                            if crate::world::soil::is_loose_soil(current_block) {
                                                crate::world::soil::update_soil_physics_at(
                                                    &mut world,
                                                    &mut commands,
                                                    &mut particle_mgr,
                                                    place_pos,
                                                );
                                            }

                                            if current_block.is_torch() {
                                                torch_burn_mgr.register(place_pos, crate::world::torch::DEFAULT_TORCH_LIFETIME);
                                            }

                                            // 🚀 扣減 1 個物品 (數量降為 0 時自動置為 None)
                                            inventory.consume_selected(1);
                                        }
                                    }
                                } else if selected_item.item_type == ItemType::Flint {
                                    handle_flint_spark_crafting(
                                        hit.adjacent_pos.as_vec3() + 0.5,
                                        &mut inventory,
                                        &registry,
                                        &mut particle_mgr,
                                    );
                                }
                            }
                        }
                    }
                }
            } else if key_f {
                println!("🚀 [Fluid Debug] F Key Pressed! Detecting raycast...");
                if world.get_fluid_global(place_pos) > 0 {
                    world.set_fluid_global(place_pos, 0);
                    enqueue_fluid_neighbors(&mut world, place_pos);
                    println!("🌊 [Fluid Debug] Removed water at: {:?}", place_pos);
                } else {
                    world.set_fluid_global(place_pos, crate::config::MAX_FLUID_LEVEL | 0x80);
                    enqueue_fluid_neighbors(&mut world, place_pos);
                    println!("🌊 [Fluid Debug] Successfully spawned water source at global pos: {:?}", place_pos);
                }
            }
        }

        if !hit_any_target {
            player.reset_mining();

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
                            println!("[Vitals] Drank water while submerged! Thirst restored to: {:.1}%", vitals.thirst);
                        }
                    }
                }
            }
        }
    }
}

/// 計算大氣天空與環境迷霧色彩（純粹眼部天空暴露感知，徹底與地面火把等方塊光源解耦）
pub fn calculate_sky_and_fog_color(
    sky_light: f32,
    sky_factor: f32,
    min_ambient_light: f32,
) -> Color {
    // 🚀 真黑極限暗夜：天幕底色維持與 Step 2 真黑環境一致的極致深邃黑夜 (0.0005, 0.001, 0.004)
    // 杜絕普通遊戲中偏亮發藍的夜空，還原硬核生存無光環境下伸手不見五指的壓迫感
    let night_sky = bevy::color::LinearRgba::new(0.0005, 0.001, 0.004, 1.0);
    let day_sky = bevy::color::LinearRgba::new(0.5, 0.8, 1.0, 1.0);
    // 將 sky_factor [0.05..1.0] 正規化為晝夜插值權重 [0.0..1.0]
    let day_factor = ((sky_factor - 0.05) / 0.95).clamp(0.0, 1.0);
    let current_sky = night_sky.mix(&day_sky, day_factor);

    // 依據眼部天空暴露度 (sky_light >= 12 視為完全開闊天空，< 12 向地底黑暗洞穴平滑過渡)
    // 徹底解耦 block_light：火把僅照亮方塊幾何，不得照亮天體大氣或將地底洞穴霧氣變為藍天
    let sky_exposure = (sky_light / 12.0).clamp(0.0, 1.0);
    let dark_ambient_color = bevy::color::LinearRgba::gray(min_ambient_light);
    let mixed = dark_ambient_color.mix(&current_sky, sky_exposure);
    Color::from(mixed)
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
    q_player: Query<&Player>,
) {
    let Ok(cam_tf) = q_camera.get_single() else { return; };
    let translation = cam_tf.translation();
    let eye_pos = IVec3::new(
        translation.x.floor() as i32,
        translation.y.floor() as i32,
        translation.z.floor() as i32,
    );
    let in_water = world_manager.get_fluid_global(eye_pos) > 0;
    let sky_light = world_manager.get_sky_light_global(eye_pos) as f32;

    // 🚀 水中沉浸感知 vs 陸地大氣感知：
    // 在水下時視距急劇縮短至 36 格，背景色切換為深邃海藍黑，還原真實深海能見度與幽暗壓迫感！
    let max_distance = (config.render_distance as f32 * 32.0).max(64.0);
    let (final_color, fog_start, fog_end) = if in_water {
        let light_ratio = (sky_light / 15.0).clamp(0.0, 1.0) * cycle.sky_factor;
        let deep_abyss = bevy::color::LinearRgba::new(0.001, 0.003, 0.006, 1.0);
        let surface_sea = bevy::color::LinearRgba::new(0.015, 0.06, 0.16, 1.0);
        let water_color = deep_abyss.mix(&surface_sea, light_ratio);
        (Color::from(water_color), 2.0, 36.0)
    } else {
        let c = calculate_sky_and_fog_color(sky_light, cycle.sky_factor, config.min_ambient_light);
        (c, max_distance * 0.75, max_distance - 8.0)
    };

    clear_color.0 = final_color;

    // 防禦：render_distance 不得為 0
    if config.render_distance == 0 { return; }

    // 【遠平面剛性鎖死】：獨立 query 確保不因 FogSettings 缺失而連帶失敗
    if let Ok(mut proj) = q_proj.get_single_mut() {
        if let Projection::Perspective(ref mut persp) = *proj {
            persp.far = (max_distance + 64.0).max(crate::utils::math::WORLD_MAX_Y as f32 * 1.25); // 仰望 1024 宏觀雪峰不被遠裁剪面截斷
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

    let mining_data = if let Ok(player) = q_player.get_single() {
        if let Some(target) = player.mining_target {
            if player.mining_progress > 0.0 && world_manager.get_block_global(target) == BlockType::OakLog {
                Vec4::new(target.x as f32, target.y as f32, target.z as f32, player.mining_progress)
            } else {
                Vec4::ZERO
            }
        } else {
            Vec4::ZERO
        }
    } else {
        Vec4::ZERO
    };

    for (_, mat) in materials.iter_mut() {
        mat.env.fog_start = fog_start;
        mat.env.fog_end = fog_end;
        mat.env.fog_color = fog_col;
        mat.env.camera_pos = cam_pos;
        mat.env.mining_data = mining_data;
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

    #[test]
    fn test_sky_and_fog_color_decoupled_from_torches() {
        let min_ambient = 0.0;

        // 1. 夜晚開闊地表 (sky_light = 15.0, sky_factor = 0.05)
        // 應維持真黑極限暗夜底色 (0.0005, 0.001, 0.004)，絕不受玩家眼部或地面有無火把影響
        let night_surface_col = calculate_sky_and_fog_color(15.0, 0.05, min_ambient);
        let night_lin = night_surface_col.to_linear();
        assert!((night_lin.blue - 0.004).abs() < 0.001, "夜晚地表天空應呈現極致深黑夜空（真黑環境），當前藍色通道: {}", night_lin.blue);
        assert!(night_lin.red < 0.001 && night_lin.green < 0.002);

        // 2. 地底洞穴無天空光 (sky_light = 0.0，無論白天或黑夜)
        // 應呈現純粹地底黑霧 (0.0, 0.0, 0.0)，火把不得把洞穴遠景渲染成藍天
        let cave_col_day = calculate_sky_and_fog_color(0.0, 1.0, min_ambient);
        let cave_lin_day = cave_col_day.to_linear();
        assert_eq!(cave_lin_day.red, 0.0);
        assert_eq!(cave_lin_day.green, 0.0);
        assert_eq!(cave_lin_day.blue, 0.0);

        let cave_col_night = calculate_sky_and_fog_color(0.0, 0.05, min_ambient);
        let cave_lin_night = cave_col_night.to_linear();
        assert_eq!(cave_lin_night.red, 0.0);
        assert_eq!(cave_lin_night.green, 0.0);
        assert_eq!(cave_lin_night.blue, 0.0);

        // 3. 白天開闊地表 (sky_light = 15.0, sky_factor = 1.0)
        let day_surface_col = calculate_sky_and_fog_color(15.0, 1.0, min_ambient);
        let day_lin = day_surface_col.to_linear();
        assert!((day_lin.red - 0.5).abs() < 0.01);
        assert!((day_lin.green - 0.8).abs() < 0.01);
        assert!((day_lin.blue - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_underwater_fog_and_abyss_darkness() {
        // 驗證水下迷霧：深墨藍到深淵黑，視距急劇收窄至 36 米以內
        let deep_abyss = bevy::color::LinearRgba::new(0.001, 0.003, 0.006, 1.0);
        let surface_sea = bevy::color::LinearRgba::new(0.015, 0.06, 0.16, 1.0);
        let water_color_day = deep_abyss.mix(&surface_sea, 1.0);
        assert!(water_color_day.blue > water_color_day.red);
        assert!(water_color_day.blue < 0.20, "Underwater daylight fog must be dark ocean blue, not sky blue");

        let water_color_deep = deep_abyss.mix(&surface_sea, 0.0);
        assert!(water_color_deep.blue < 0.01, "Deep water abyss must be pitch black");
    }
}


