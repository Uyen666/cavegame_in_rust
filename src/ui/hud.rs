use bevy::prelude::*;
use crate::player::{Player, PlayerCamera};
use crate::world::{WorldManager, BlockType};
use crate::item::{Inventory, ItemRegistry, ItemKind, ItemType};

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

#[derive(Resource, Default)]
pub struct ItemIconRegistry {
    pub icons: std::collections::HashMap<ItemType, Handle<Image>>,
    pub fallback: Handle<Image>,
}

pub fn setup_item_icons(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
) {
    use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
    use bevy::render::texture::{ImageSampler, ImageSamplerDescriptor, ImageAddressMode, ImageFilterMode};

    let mut icon_map = std::collections::HashMap::new();

    let mut create_icon = |generator: &dyn Fn(u32, u32) -> [u8; 4]| -> Handle<Image> {
        let size = 16;
        let mut data = Vec::with_capacity((size * size * 4) as usize);
        for y in 0..size {
            for x in 0..size {
                let pixel = generator(x, y);
                data.extend_from_slice(&pixel);
            }
        }
        let mut img = Image::new(
            Extent3d { width: size, height: size, depth_or_array_layers: 1 },
            TextureDimension::D2,
            data,
            TextureFormat::Rgba8UnormSrgb,
            bevy::render::render_asset::RenderAssetUsages::RENDER_WORLD | bevy::render::render_asset::RenderAssetUsages::MAIN_WORLD,
        );
        img.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
            address_mode_u: ImageAddressMode::ClampToEdge,
            address_mode_v: ImageAddressMode::ClampToEdge,
            mag_filter: ImageFilterMode::Nearest,
            min_filter: ImageFilterMode::Nearest,
            ..default()
        });
        images.add(img)
    };

    // Pickaxe Icon (Iron / Wooden / Stone)
    let pickaxe_handle = create_icon(&|x, y| {
        if (y == 2 && x >= 3 && x <= 12) || (y == 3 && x >= 2 && x <= 13) || (y == 4 && (x == 2 || x == 13)) {
            [210, 215, 225, 255] // Metallic Silver Head
        } else if x == y && x >= 4 && x <= 12 {
            [140, 90, 45, 255] // Wooden Handle
        } else {
            [0, 0, 0, 0]
        }
    });

    // Coal Icon
    let coal_handle = create_icon(&|x, y| {
        if x >= 5 && x <= 11 && y >= 5 && y <= 11 {
            if x == 6 && y == 6 {
                [90, 90, 100, 255]
            } else {
                [35, 35, 40, 255]
            }
        } else {
            [0, 0, 0, 0]
        }
    });

    // Iron Ingot Icon
    let ingot_handle = create_icon(&|x, y| {
        if x >= 4 && x <= 12 && y >= 6 && y <= 10 {
            if y == 6 {
                [230, 235, 245, 255]
            } else {
                [180, 185, 195, 255]
            }
        } else {
            [0, 0, 0, 0]
        }
    });

    // Stick Icon
    let stick_handle = create_icon(&|x, y| {
        if x + y == 15 && x >= 4 && x <= 11 {
            [140, 90, 45, 255]
        } else {
            [0, 0, 0, 0]
        }
    });

    icon_map.insert(ItemType::IronPickaxe, pickaxe_handle.clone());
    icon_map.insert(ItemType::WoodenPickaxe, pickaxe_handle.clone());
    icon_map.insert(ItemType::StonePickaxe, pickaxe_handle.clone());
    icon_map.insert(ItemType::WoodenShovel, pickaxe_handle.clone());
    icon_map.insert(ItemType::StoneShovel, pickaxe_handle.clone());
    icon_map.insert(ItemType::IronShovel, pickaxe_handle.clone());
    icon_map.insert(ItemType::WoodenAxe, pickaxe_handle.clone());
    icon_map.insert(ItemType::StoneAxe, pickaxe_handle.clone());
    icon_map.insert(ItemType::IronAxe, pickaxe_handle.clone());
    icon_map.insert(ItemType::Coal, coal_handle);
    icon_map.insert(ItemType::IronIngot, ingot_handle);
    icon_map.insert(ItemType::Stick, stick_handle);

    commands.insert_resource(ItemIconRegistry {
        icons: icon_map,
        fallback: pickaxe_handle,
    });
}

#[derive(Component)]
pub struct HotbarUi;

#[derive(Component)]
pub struct HotbarSlotUi {
    pub slot_index: usize,
}

#[derive(Component)]
pub struct HotbarPreviewNode;

#[derive(Component)]
pub struct Hotbar2dIconNode;

#[derive(Component)]
pub struct HotbarCountTextNode;

#[derive(Component)]
pub struct HotbarDurabilityBarContainerNode;

#[derive(Component)]
pub struct HotbarDurabilityBarFillNode;

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
                    // 1. 左上角微型鍵位數字 (1..9)
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

                    // 2. 中央 3D 方塊圖示預覽 (綁定 RTT Image)
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

                    // 3. 中央 2D 物品 Icon (非方塊如鐵鎬, 煤炭)
                    slot.spawn((
                        ImageBundle {
                            style: Style {
                                width: Val::Px(32.0),
                                height: Val::Px(32.0),
                                position_type: PositionType::Absolute,
                                ..default()
                            },
                            background_color: BackgroundColor(Color::NONE),
                            visibility: Visibility::Hidden,
                            ..default()
                        },
                        Hotbar2dIconNode,
                    ));

                    // 4. 右下角數量數字 Text
                    slot.spawn((
                        TextBundle::from_section(
                            "",
                            TextStyle {
                                font_size: 13.0,
                                color: Color::WHITE,
                                ..default()
                            }
                        ).with_style(Style {
                            position_type: PositionType::Absolute,
                            bottom: Val::Px(2.0),
                            right: Val::Px(4.0),
                            ..default()
                        }),
                        HotbarCountTextNode,
                    ));

                    // 5. 槽位底部耐久度條 Container 與 Fill Node
                    slot.spawn((
                        NodeBundle {
                            style: Style {
                                width: Val::Px(36.0),
                                height: Val::Px(3.0),
                                position_type: PositionType::Absolute,
                                bottom: Val::Px(2.0),
                                left: Val::Px(6.0),
                                ..default()
                            },
                            background_color: BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.8)),
                            visibility: Visibility::Hidden,
                            ..default()
                        },
                        HotbarDurabilityBarContainerNode,
                    )).with_children(|bar| {
                        bar.spawn((
                            NodeBundle {
                                style: Style {
                                    width: Val::Percent(100.0),
                                    height: Val::Percent(100.0),
                                    ..default()
                                },
                                background_color: BackgroundColor(Color::srgb(0.2, 0.9, 0.2)),
                                ..default()
                            },
                            HotbarDurabilityBarFillNode,
                        ));
                    });
                });
            }
        });
    });
}

pub fn update_hotbar_ui(
    q_player: Query<(&Inventory, &Player), Or<(Changed<Inventory>, Changed<Player>)>>,
    registry: Res<ItemRegistry>,
    icon_reg: Option<Res<ItemIconRegistry>>,
    mut q_slots: Query<(&HotbarSlotUi, &mut BackgroundColor, &mut BorderColor, &mut Style, &Children), Without<HotbarDurabilityBarFillNode>>,
    mut q_preview: Query<&mut UiImage, (With<HotbarPreviewNode>, Without<Hotbar2dIconNode>)>,
    mut q_2d_icon: Query<&mut UiImage, (With<Hotbar2dIconNode>, Without<HotbarPreviewNode>)>,
    mut q_count_text: Query<&mut Text, With<HotbarCountTextNode>>,
    q_dur_container: Query<Entity, With<HotbarDurabilityBarContainerNode>>,
    mut q_dur_fill: Query<(&mut Style, &mut BackgroundColor), (With<HotbarDurabilityBarFillNode>, Without<HotbarSlotUi>)>,
    preview_imgs: Option<Res<Ui3dPreviewImages>>,
    mut q_preview_mesh: Query<(Entity, &mut Handle<Mesh>, &UiPreviewMesh), Without<HotbarSlotUi>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut q_visibility: Query<&mut Visibility>,
) {
    let Ok((inventory, _player)) = q_player.get_single() else { return; };

    // 🚀 1. 動態更新 9 個攝影棚的方塊網格
    for (mesh_entity, mut mesh_handle, preview_mesh) in q_preview_mesh.iter_mut() {
        let slot_idx = preview_mesh.slot_index;
        let item_stack = inventory.slot(slot_idx);

        let mut show_mesh = false;
        if let Some(stack) = item_stack {
            if let Some(def) = registry.get(stack.item_type) {
                if let ItemKind::Block(block_type) = def.kind {
                    if block_type != BlockType::Air {
                        show_mesh = true;
                        *mesh_handle = meshes.add(crate::render::greedy::build_single_voxel_mesh(block_type));
                    }
                }
            }
        }

        if let Ok(mut vis) = q_visibility.get_mut(mesh_entity) {
            *vis = if show_mesh { Visibility::Inherited } else { Visibility::Hidden };
        }
    }

    // 🚀 2. 更新 Hotbar 槽位 UI（雙軌 3D/2D、選中框高亮、數量數字、耐久度條）
    for (slot, mut bg_color, mut border_color, mut style, children) in q_slots.iter_mut() {
        let is_selected = slot.slot_index == inventory.selected_slot;
        let item_stack = inventory.slot(slot.slot_index);

        // 選中框風格
        *bg_color = BackgroundColor(Color::srgba(0.1, 0.1, 0.1, 0.5));
        if is_selected {
            style.border = UiRect::all(Val::Px(3.0));
            *border_color = BorderColor(Color::srgb(1.0, 0.84, 0.0));
        } else {
            style.border = UiRect::all(Val::Px(1.0));
            *border_color = BorderColor(Color::srgba(0.3, 0.3, 0.3, 0.5));
        }

        for &child in children.iter() {
            // 3D Preview Image Node
            if let Ok(mut preview_image) = q_preview.get_mut(child) {
                let mut show = false;
                if let Some(stack) = item_stack {
                    if let Some(def) = registry.get(stack.item_type) {
                        if let ItemKind::Block(block_type) = def.kind {
                            if block_type != BlockType::Air {
                                if let Some(imgs) = &preview_imgs {
                                    preview_image.texture = imgs.0[slot.slot_index].clone();
                                    preview_image.color = Color::WHITE;
                                    show = true;
                                }
                            }
                        }
                    }
                }
                if let Ok(mut vis) = q_visibility.get_mut(child) {
                    *vis = if show { Visibility::Inherited } else { Visibility::Hidden };
                }
                continue;
            }

            // 2D Icon Node (Tools / Materials)
            if let Ok(mut icon_image) = q_2d_icon.get_mut(child) {
                let mut show = false;
                if let Some(stack) = item_stack {
                    if let Some(def) = registry.get(stack.item_type) {
                        match def.kind {
                            ItemKind::Tool { .. } | ItemKind::Material => {
                                if let Some(ref reg) = icon_reg {
                                    let icon_handle = reg.icons.get(&stack.item_type).cloned().unwrap_or_else(|| reg.fallback.clone());
                                    icon_image.texture = icon_handle;
                                    icon_image.color = Color::WHITE;
                                    show = true;
                                }
                            }
                            _ => {}
                        }
                    }
                }
                if let Ok(mut vis) = q_visibility.get_mut(child) {
                    *vis = if show { Visibility::Inherited } else { Visibility::Hidden };
                }
                continue;
            }

            // Count Text Node
            if let Ok(mut text) = q_count_text.get_mut(child) {
                let mut show = false;
                if let Some(stack) = item_stack {
                    if stack.count > 1 {
                        text.sections[0].value = format!("{}", stack.count);
                        show = true;
                    }
                }
                if let Ok(mut vis) = q_visibility.get_mut(child) {
                    *vis = if show { Visibility::Inherited } else { Visibility::Hidden };
                }
                continue;
            }

            // Durability Bar Container Node
            if let Ok(container_entity) = q_dur_container.get(child) {
                let mut is_damaged_tool = false;
                if let Some(stack) = item_stack {
                    if let (Some(dur), Some(max_dur)) = (stack.durability, stack.max_durability(&registry)) {
                        if max_dur > 0 && dur < max_dur {
                            is_damaged_tool = true;
                            let ratio = (dur as f32 / max_dur as f32).clamp(0.0, 1.0);

                            // 更新子節點 Fill
                            for &bar_child in children.iter() {
                                if let Ok((mut fill_style, mut fill_bg)) = q_dur_fill.get_mut(bar_child) {
                                    fill_style.width = Val::Percent(ratio * 100.0);
                                    fill_bg.0 = if ratio > 0.5 {
                                        Color::srgb(0.2, 0.9, 0.2)
                                    } else if ratio >= 0.2 {
                                        Color::srgb(0.9, 0.9, 0.2)
                                    } else {
                                        Color::srgb(0.9, 0.2, 0.2)
                                    };
                                }
                            }
                        }
                    }
                }

                if let Ok(mut vis) = q_visibility.get_mut(container_entity) {
                    *vis = if is_damaged_tool { Visibility::Inherited } else { Visibility::Hidden };
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

