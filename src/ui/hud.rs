use bevy::prelude::*;
use crate::player::{Player, PlayerCamera};
use crate::world::{WorldManager, BlockType};

#[derive(Component)]
pub struct CrosshairContainer;

#[derive(Component)]
pub struct CrosshairLine;

pub fn setup_crosshair(mut commands: Commands) {
    // Crosshair container to center everything
    commands.spawn((
        NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            background_color: BackgroundColor(Color::NONE),
            ..default()
        },
        CrosshairContainer, // Tag container to despawn easily
    )).with_children(|parent| {
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
            CrosshairLine,
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
            CrosshairLine,
        ));
    });
}

pub fn cleanup_crosshair(
    mut commands: Commands,
    q_container: Query<Entity, With<CrosshairContainer>>,
) {
    for entity in q_container.iter() {
        commands.entity(entity).despawn_recursive();
    }
}

pub fn update_crosshair(
    mut q_lines: Query<&mut BackgroundColor, With<CrosshairLine>>,
    q_camera: Query<&GlobalTransform, With<PlayerCamera>>,
    world: Res<WorldManager>,
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

            let block = world.get_block_global(block_pos);
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
    for mut bg in q_lines.iter_mut() {
        bg.0 = target_color;
    }
}

#[derive(Component)]
pub struct UiPreviewMesh {
    pub slot_index: usize,
}

#[derive(Resource)]
pub struct Ui3dPreviewImages(pub Vec<Handle<Image>>);

pub fn setup_ui_3d_preview(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    textures: Res<crate::render::textures::GameTextures>,
) {
    use bevy::render::render_resource::{Extent3d, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages};
    use bevy::render::camera::RenderTarget;
    use bevy::render::view::RenderLayers;

    let layer = RenderLayers::layer(1);
    let mut image_handles = Vec::new();

    for i in 0..9 {
        let size = Extent3d { width: 128, height: 128, depth_or_array_layers: 1 };
        let mut image = Image {
            texture_descriptor: TextureDescriptor {
                label: Some("UI 3D Preview Render Target"),
                size,
                dimension: TextureDimension::D2,
                format: TextureFormat::Bgra8UnormSrgb,
                mip_level_count: 1,
                sample_count: 1,
                usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST | TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            },
            ..default()
        };
        image.resize(size);
        let image_handle = images.add(image);
        image_handles.push(image_handle.clone());

        let offset_x = i as f32 * 10.0;

        // 3D Camera for UI
        commands.spawn((
            Camera3dBundle {
                camera: Camera {
                    order: -1, // Render before UI
                    target: RenderTarget::Image(image_handle),
                    clear_color: ClearColorConfig::Custom(Color::NONE),
                    ..default()
                },
                projection: Projection::Orthographic(OrthographicProjection {
                    scale: 0.65,
                    scaling_mode: bevy::render::camera::ScalingMode::FixedVertical(2.5),
                    ..default()
                }),
                transform: Transform::from_xyz(1.8 + offset_x, 1.8, 1.8).looking_at(Vec3::new(0.5 + offset_x, 0.5, 0.5), Vec3::Y),
                ..default()
            },
            layer.clone(),
        ));

        // Preview Mesh
        commands.spawn((
            MaterialMeshBundle {
                mesh: meshes.add(crate::render::greedy::build_single_voxel_mesh(BlockType::Air)),
                material: textures.material.clone(),
                transform: Transform::from_xyz(offset_x, 0.0, 0.0),
                ..default()
            },
            UiPreviewMesh { slot_index: i },
            layer.clone(),
        ));
    }

    // Directional Light
    commands.spawn((
        DirectionalLightBundle {
            directional_light: DirectionalLight {
                illuminance: 5000.0,
                shadows_enabled: false,
                ..default()
            },
            transform: Transform::from_xyz(-2.0, 4.0, 2.0).looking_at(Vec3::ZERO, Vec3::Y),
            ..default()
        },
        layer.clone(),
    ));

    commands.insert_resource(Ui3dPreviewImages(image_handles));
}

#[derive(Component)]
pub struct HotbarUi;

#[derive(Component)]
pub struct HotbarSlotUi {
    pub slot_index: usize,
}



#[derive(Component)]
pub struct HotbarPreviewNode;

pub fn setup_hotbar_ui(mut commands: Commands) {
    commands.spawn((
        NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::FlexEnd,
                padding: UiRect::bottom(Val::Px(20.0)),
                ..default()
            },
            background_color: BackgroundColor(Color::NONE),
            ..default()
        },
        HotbarUi,
    )).with_children(|parent| {
        parent.spawn(NodeBundle {
            style: Style {
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(6.0),
                padding: UiRect::all(Val::Px(6.0)),
                ..default()
            },
            background_color: BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.6)),
            ..default()
        }).with_children(|hotbar| {
            for i in 0..9 {
                hotbar.spawn((
                    NodeBundle {
                        style: Style {
                            width: Val::Px(48.0),
                            height: Val::Px(48.0),
                            border: UiRect::all(Val::Px(1.0)),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        background_color: BackgroundColor(Color::srgba(0.1, 0.1, 0.1, 0.5)),
                        border_color: BorderColor(Color::srgba(0.3, 0.3, 0.3, 0.5)),
                        ..default()
                    },
                    HotbarSlotUi { slot_index: i },
                )).with_children(|slot| {
                    // 左上角微型鍵位數字
                    slot.spawn(TextBundle::from_section(
                        format!("{}", i + 1),
                        TextStyle {
                            font_size: 10.0,
                            color: Color::srgb(0.6, 0.6, 0.6),
                            ..default()
                        }
                    ).with_style(Style {
                        position_type: PositionType::Absolute,
                        top: Val::Px(2.0),
                        left: Val::Px(4.0),
                        ..default()
                    }));

                    // 中央方塊圖示預覽 (綁定 RTT Image)
                    slot.spawn((
                        ImageBundle {
                            style: Style {
                                width: Val::Px(40.0),
                                height: Val::Px(40.0),
                                position_type: PositionType::Absolute,
                                ..default()
                            },
                            background_color: BackgroundColor(Color::NONE),
                            ..default()
                        },
                        HotbarPreviewNode,
                    ));
                });
            }
        });
    });
}

pub fn update_hotbar_ui(
    q_player: Query<&Player, Changed<Player>>, // 僅在 Player 狀態改變時響應
    mut q_slots: Query<(&HotbarSlotUi, &mut BackgroundColor, &mut BorderColor, &mut Style, &Children)>,
    mut q_preview: Query<&mut UiImage, (With<HotbarPreviewNode>, Without<HotbarSlotUi>)>,
    preview_imgs: Option<Res<Ui3dPreviewImages>>,
    mut q_preview_mesh: Query<(&mut Handle<Mesh>, &mut Visibility, &UiPreviewMesh)>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let Ok(player) = q_player.get_single() else { return; };

    // 🚀 動態更新 9 個攝影棚的方塊網格 (套用 VoxelMaterial 壓縮頂點與真實材質)
    for (mut mesh_handle, mut vis, preview_mesh) in q_preview_mesh.iter_mut() {
        let block = player.hotbar[preview_mesh.slot_index];
        if block == BlockType::Air {
            *vis = Visibility::Hidden; // 空氣直接隱藏網格
        } else {
            *vis = Visibility::Inherited;
            *mesh_handle = meshes.add(crate::render::greedy::build_single_voxel_mesh(block));
        }
    }

    for (slot, mut bg_color, mut border_color, mut style, children) in q_slots.iter_mut() {
        let is_selected = slot.slot_index == player.selected_slot;
        let block_type = player.hotbar[slot.slot_index];

        // 🚀 UI 選中框風格修正（絕不重疊灰底）
        *bg_color = BackgroundColor(Color::srgba(0.1, 0.1, 0.1, 0.5)); // 永遠保持原本的半透明灰
        if is_selected {
            style.border = UiRect::all(Val::Px(3.0));
            *border_color = BorderColor(Color::srgb(1.0, 0.84, 0.0));
        } else {
            style.border = UiRect::all(Val::Px(1.0));
            *border_color = BorderColor(Color::srgba(0.3, 0.3, 0.3, 0.5));
        }

        // 🚀 同步顯示當前格子的圖示與文字
        for &child in children.iter() {
            if let Ok(mut preview_image) = q_preview.get_mut(child) {
                if block_type == BlockType::Air {
                    preview_image.texture = Handle::default();
                    preview_image.color = Color::NONE; // 空氣完全透明
                } else if let Some(imgs) = &preview_imgs {
                    preview_image.texture = imgs.0[slot.slot_index].clone();
                    preview_image.color = Color::WHITE;
                }
            }
        }
    }
}

pub fn cleanup_hotbar_ui(
    mut commands: Commands,
    q_ui: Query<Entity, With<HotbarUi>>,
) {
    for entity in q_ui.iter() {
        commands.entity(entity).despawn_recursive();
    }
}
