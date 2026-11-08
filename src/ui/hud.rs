use bevy::prelude::*;
use crate::player::PlayerCamera;
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

        if let Some(hit) = crate::utils::math::raycast_voxel(&world, start, *forward, max_dist) {
            if hit.block_type == BlockType::Stone {
                hit_dark = true;
            }
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
    pub current_block: BlockType,
}

pub const PREVIEW_BLOCKS: &[BlockType] = &[
    BlockType::Stone,
    BlockType::Dirt,
    BlockType::Grass,
    BlockType::OakLog,
    BlockType::OakLeaves,
    BlockType::Sand,
    BlockType::Gravel,
    BlockType::CoalOre,
    BlockType::IronOre,
    BlockType::Glass,
    BlockType::Torch,
];

#[derive(Resource, Default)]
pub struct Block3dPreviewRegistry {
    pub map: std::collections::HashMap<BlockType, Handle<Image>>,
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

    // 1. 保留原本 Hotbar 9 格動態離屏預覽
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
            UiPreviewMesh { slot_index: i, current_block: BlockType::Air },
            layer.clone(),
        ));
    }

    // 2. 🚀 為全遊戲 11 種方塊建立靜態 3D 等角預覽攝影棚 (供大背包與游標手持展示)
    let mut block_preview_map = std::collections::HashMap::new();
    for (idx, &block) in PREVIEW_BLOCKS.iter().enumerate() {
        let size = Extent3d { width: 128, height: 128, depth_or_array_layers: 1 };
        let mut image = Image {
            texture_descriptor: TextureDescriptor {
                label: Some("Block 3D Static Render Target"),
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
        block_preview_map.insert(block, image_handle.clone());

        let offset_x = (10 + idx) as f32 * 10.0;

        commands.spawn((
            Camera3dBundle {
                camera: Camera {
                    order: -1,
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

        commands.spawn((
            MaterialMeshBundle {
                mesh: meshes.add(crate::render::greedy::build_single_voxel_mesh(block)),
                material: textures.material.clone(),
                transform: Transform::from_xyz(offset_x, 0.0, 0.0),
                ..default()
            },
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
    commands.insert_resource(Block3dPreviewRegistry { map: block_preview_map });
}

#[derive(Resource, Default)]
pub struct ItemIconRegistry {
    pub icons: std::collections::HashMap<ItemType, Handle<Image>>,
    pub fallback: Handle<Image>,
}

pub fn setup_item_icons(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    block_previews: Res<Block3dPreviewRegistry>,
) {
    use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
    use bevy::render::texture::{ImageSampler, ImageSamplerDescriptor, ImageAddressMode, ImageFilterMode};
    use std::fs::File;
    use std::io::Read;
    use zip::ZipArchive;

    let mut icon_map = std::collections::HashMap::new();

    // 優先從官方資源包 assets/resource_pack.zip 中讀取原汁原味高解析 2D 材質
    let mut zip_archive = File::open("assets/resource_pack.zip")
        .ok()
        .and_then(|f| ZipArchive::new(f).ok());

    fn load_from_zip(archive: &mut ZipArchive<File>, images: &mut Assets<Image>, item_name: &str) -> Option<Handle<Image>> {
        use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
        use bevy::render::texture::{ImageSampler, ImageSamplerDescriptor, ImageAddressMode, ImageFilterMode};
        let paths = [
            format!("assets/minecraft/textures/item/{}.png", item_name),
            format!("assets/minecraft/textures/items/{}.png", item_name),
            format!("assets/minecraft/textures/block/{}.png", item_name),
            format!("assets/minecraft/textures/blocks/{}.png", item_name),
        ];
        for path in paths {
            if let Ok(mut zf) = archive.by_name(&path) {
                let mut bytes = Vec::new();
                if zf.read_to_end(&mut bytes).is_ok() {
                    if let Ok(dyn_img) = image::load_from_memory(&bytes) {
                        let rgba = dyn_img.into_rgba8();
                        let (w, h) = rgba.dimensions();
                        let mut img = Image::new(
                            Extent3d { width: w, height: h, depth_or_array_layers: 1 },
                            TextureDimension::D2,
                            rgba.into_raw(),
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
                        return Some(images.add(img));
                    }
                }
            }
        }
        None
    }

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

    // 程式化兜底圖標 (Fallback Icons)
    let pickaxe_fallback = create_icon(&|x, y| {
        if (y == 2 && x >= 3 && x <= 12) || (y == 3 && x >= 2 && x <= 13) || (y == 4 && (x == 2 || x == 13)) {
            [210, 215, 225, 255]
        } else if x == y && x >= 4 && x <= 12 {
            [140, 90, 45, 255]
        } else {
            [0, 0, 0, 0]
        }
    });
    let coal_fallback = create_icon(&|x, y| {
        if x >= 5 && x <= 11 && y >= 5 && y <= 11 { [35, 35, 40, 255] } else { [0, 0, 0, 0] }
    });
    let ingot_fallback = create_icon(&|x, y| {
        if x >= 4 && x <= 12 && y >= 6 && y <= 10 { [200, 205, 215, 255] } else { [0, 0, 0, 0] }
    });
    let stick_fallback = create_icon(&|x, y| {
        if x + y == 15 && x >= 4 && x <= 11 { [140, 90, 45, 255] } else { [0, 0, 0, 0] }
    });
    let flint_fallback = create_icon(&|x, y| {
        if (x >= 5 && x <= 10 && y >= 6 && y <= 11) || (x == 7 && y == 5) || (x == 11 && y == 9) { [50, 55, 60, 255] } else { [0, 0, 0, 0] }
    });

    // 專屬打製石刃像素圖標 (Knapped Tool Heads)
    let flint_axe_head_fallback = create_icon(&|x, y| {
        if (x >= 4 && x <= 11 && y >= 5 && y <= 10) || (x >= 6 && x <= 9 && y >= 3 && y <= 4) {
            if x == 11 || y == 3 || (x == 10 && y == 4) {
                [80, 88, 98, 255] // 鋒利打製刃口高光
            } else if x == 4 || y == 10 {
                [30, 32, 36, 255] // 捆紮凹槽陰影
            } else {
                [48, 52, 58, 255] // 燧石本體
            }
        } else {
            [0, 0, 0, 0]
        }
    });
    let flint_pickaxe_head_fallback = create_icon(&|x, y| {
        let is_arch = (y == 4 && (x == 3 || x == 12)) 
            || (y == 5 && (x == 4 || x == 11))
            || (y == 6 && x >= 5 && x <= 10)
            || (y == 7 && x >= 6 && x <= 9);
        if is_arch {
            if (y == 4 && (x == 3 || x == 12)) || (y == 5 && (x == 4 || x == 11)) {
                [85, 92, 102, 255] // 鎬尖刃口
            } else if y == 7 {
                [32, 34, 38, 255]
            } else {
                [48, 52, 58, 255]
            }
        } else {
            [0, 0, 0, 0]
        }
    });
    let flint_shovel_head_fallback = create_icon(&|x, y| {
        if (x >= 5 && x <= 10 && y >= 5 && y <= 10) || (x >= 6 && x <= 9 && y == 11) {
            if y == 5 || (x == 10 && y <= 8) {
                [80, 88, 98, 255]
            } else if y == 11 || x == 5 {
                [30, 32, 36, 255]
            } else {
                [48, 52, 58, 255]
            }
        } else {
            [0, 0, 0, 0]
        }
    });
    let flint_knife_blade_fallback = create_icon(&|x, y| {
        if (x == y && x >= 5 && x <= 11) || (x == y + 1 && x >= 6 && x <= 11) || (x == y - 1 && x >= 5 && x <= 10) {
            if x == y + 1 {
                [88, 96, 108, 255]
            } else {
                [48, 52, 58, 255]
            }
        } else {
            [0, 0, 0, 0]
        }
    });

    // 專屬原始燧石工具像素圖標 (帶深色黑曜刃面與粗藤條/纖維捆紮 Twine Wrap)
    let flint_axe_fallback = create_icon(&|x, y| {
        let is_handle = x == y && x >= 3 && x <= 13;
        let is_blade = (x >= 3 && x <= 7 && y >= 2 && y <= 5) && !(x == 3 && y == 5);
        let is_twine = (x == 5 && y == 6) || (x == 6 && y == 5) || (x == 6 && y == 7) || (x == 7 && y == 6);

        if is_twine {
            [165, 125, 65, 255] // 植物纖維/繩結捆紮
        } else if is_blade {
            if x == 3 || y == 2 {
                [80, 88, 98, 255]
            } else {
                [45, 48, 55, 255] // 深黑燧石刀刃
            }
        } else if is_handle {
            [135, 85, 40, 255] // 粗糙木柄
        } else {
            [0, 0, 0, 0]
        }
    });
    let flint_pickaxe_fallback = create_icon(&|x, y| {
        let is_handle = x == y && x >= 4 && x <= 13;
        let is_pick = (y == 2 && (x == 2 || x == 9))
            || (y == 3 && x >= 3 && x <= 8)
            || (y == 4 && (x == 4 || x == 7));
        let is_twine = (x == 5 && y == 4) || (x == 6 && y == 3);

        if is_twine {
            [165, 125, 65, 255]
        } else if is_pick {
            if y == 2 || x == 2 || x == 9 {
                [85, 92, 102, 255]
            } else {
                [45, 48, 55, 255]
            }
        } else if is_handle {
            [135, 85, 40, 255]
        } else {
            [0, 0, 0, 0]
        }
    });
    let flint_shovel_fallback = create_icon(&|x, y| {
        let is_handle = x == y && x >= 5 && x <= 13;
        let is_scoop = x >= 2 && x <= 5 && y >= 2 && y <= 5;
        let is_twine = (x == 5 && y == 6) || (x == 6 && y == 5);

        if is_twine {
            [165, 125, 65, 255]
        } else if is_scoop {
            if x == 2 || y == 2 {
                [80, 88, 98, 255]
            } else {
                [45, 48, 55, 255]
            }
        } else if is_handle {
            [135, 85, 40, 255]
        } else {
            [0, 0, 0, 0]
        }
    });
    let flint_knife_fallback = create_icon(&|x, y| {
        let is_handle = (x == y && x >= 9 && x <= 13) || (x == y + 1 && x >= 10 && x <= 13);
        let is_blade = (x == y && x >= 3 && x <= 8) || (x == y - 1 && x >= 3 && x <= 7);
        let is_twine = (x == 8 && y == 8) || (x == 9 && y == 8);

        if is_twine {
            [165, 125, 65, 255]
        } else if is_blade {
            if x == y - 1 {
                [88, 96, 108, 255]
            } else {
                [45, 48, 55, 255]
            }
        } else if is_handle {
            [135, 85, 40, 255]
        } else {
            [0, 0, 0, 0]
        }
    });

    let get_icon = |archive_opt: &mut Option<ZipArchive<File>>, images_res: &mut Assets<Image>, name: &str, fallback: Handle<Image>| -> Handle<Image> {
        if let Some(ref mut archive) = archive_opt {
            load_from_zip(archive, images_res, name).unwrap_or(fallback)
        } else {
            fallback
        }
    };

    // 註冊所有工具與掉落物官方 2D 圖標
    icon_map.insert(ItemType::Stick, get_icon(&mut zip_archive, &mut images, "stick", stick_fallback));
    icon_map.insert(ItemType::Flint, get_icon(&mut zip_archive, &mut images, "flint", flint_fallback.clone()));
    icon_map.insert(ItemType::FlintAxeHead, get_icon(&mut zip_archive, &mut images, "flint_axe_head", flint_axe_head_fallback));
    icon_map.insert(ItemType::FlintPickaxeHead, get_icon(&mut zip_archive, &mut images, "flint_pickaxe_head", flint_pickaxe_head_fallback));
    icon_map.insert(ItemType::FlintShovelHead, get_icon(&mut zip_archive, &mut images, "flint_shovel_head", flint_shovel_head_fallback));
    icon_map.insert(ItemType::FlintKnifeBlade, get_icon(&mut zip_archive, &mut images, "flint_knife_blade", flint_knife_blade_fallback));
    icon_map.insert(ItemType::Coal, get_icon(&mut zip_archive, &mut images, "coal", coal_fallback));
    icon_map.insert(ItemType::IronIngot, get_icon(&mut zip_archive, &mut images, "iron_ingot", ingot_fallback));

    icon_map.insert(ItemType::WoodenAxe, get_icon(&mut zip_archive, &mut images, "wooden_axe", pickaxe_fallback.clone()));
    icon_map.insert(ItemType::StoneAxe, get_icon(&mut zip_archive, &mut images, "stone_axe", pickaxe_fallback.clone()));
    icon_map.insert(ItemType::FlintAxe, get_icon(&mut zip_archive, &mut images, "flint_axe", flint_axe_fallback));
    icon_map.insert(ItemType::FlintPickaxe, get_icon(&mut zip_archive, &mut images, "flint_pickaxe", flint_pickaxe_fallback));
    icon_map.insert(ItemType::FlintShovel, get_icon(&mut zip_archive, &mut images, "flint_shovel", flint_shovel_fallback));
    icon_map.insert(ItemType::FlintKnife, get_icon(&mut zip_archive, &mut images, "flint_knife", flint_knife_fallback));
    icon_map.insert(ItemType::IronAxe, get_icon(&mut zip_archive, &mut images, "iron_axe", pickaxe_fallback.clone()));

    icon_map.insert(ItemType::WoodenPickaxe, get_icon(&mut zip_archive, &mut images, "wooden_pickaxe", pickaxe_fallback.clone()));
    icon_map.insert(ItemType::StonePickaxe, get_icon(&mut zip_archive, &mut images, "stone_pickaxe", pickaxe_fallback.clone()));
    icon_map.insert(ItemType::IronPickaxe, get_icon(&mut zip_archive, &mut images, "iron_pickaxe", pickaxe_fallback.clone()));

    icon_map.insert(ItemType::WoodenShovel, get_icon(&mut zip_archive, &mut images, "wooden_shovel", pickaxe_fallback.clone()));
    icon_map.insert(ItemType::StoneShovel, get_icon(&mut zip_archive, &mut images, "stone_shovel", pickaxe_fallback.clone()));
    icon_map.insert(ItemType::IronShovel, get_icon(&mut zip_archive, &mut images, "iron_shovel", pickaxe_fallback.clone()));

    // 🚀 方塊類物品優先注入等角 3D 渲染圖，使背包與 Hotbar 達到極致立體統一
    for &block in PREVIEW_BLOCKS {
        if let Some(handle) = block_previews.map.get(&block) {
            if let Some(item) = ItemType::from_block(block) {
                icon_map.insert(item, handle.clone());
            }
        }
    }

    commands.insert_resource(ItemIconRegistry {
        icons: icon_map,
        fallback: pickaxe_fallback,
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

#[derive(Component)]
pub struct VitalsHealthBarFill;

#[derive(Component)]
pub struct VitalsStaminaBarFill;

#[derive(Component)]
pub struct VitalsThirstBarFill;

#[derive(Component)]
pub struct VitalsStatusText;

pub fn setup_hotbar_ui(mut commands: Commands) {
    commands.spawn((
        NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::FlexEnd,
                padding: UiRect::bottom(Val::Px(16.0)),
                ..default()
            },
            background_color: BackgroundColor(Color::NONE),
            ..default()
        },
        HotbarUi,
    )).with_children(|parent| {
        parent.spawn(NodeBundle {
            style: Style {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                row_gap: Val::Px(6.0),
                ..default()
            },
            background_color: BackgroundColor(Color::NONE),
            ..default()
        }).with_children(|col| {
            // ── Vitals HUD Container (Health, Stamina, Thirst, Temp/Wetness) ──
            col.spawn(NodeBundle {
                style: Style {
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(10.0),
                    align_items: AlignItems::Center,
                    padding: UiRect::axes(Val::Px(8.0), Val::Px(4.0)),
                    ..default()
                },
                background_color: BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.6)),
                ..default()
            }).with_children(|vitals_ui| {
                // HP Bar
                vitals_ui.spawn(NodeBundle {
                    style: Style {
                        width: Val::Px(70.0),
                        height: Val::Px(6.0),
                        border: UiRect::all(Val::Px(1.0)),
                        ..default()
                    },
                    background_color: BackgroundColor(Color::srgb(0.2, 0.05, 0.05)),
                    border_color: BorderColor(Color::srgb(0.4, 0.1, 0.1)),
                    ..default()
                }).with_children(|bar| {
                    bar.spawn((
                        NodeBundle {
                            style: Style {
                                width: Val::Percent(100.0),
                                height: Val::Percent(100.0),
                                ..default()
                            },
                            background_color: BackgroundColor(Color::srgb(0.9, 0.2, 0.2)),
                            ..default()
                        },
                        VitalsHealthBarFill,
                    ));
                });

                // Stamina Bar
                vitals_ui.spawn(NodeBundle {
                    style: Style {
                        width: Val::Px(70.0),
                        height: Val::Px(6.0),
                        border: UiRect::all(Val::Px(1.0)),
                        ..default()
                    },
                    background_color: BackgroundColor(Color::srgb(0.2, 0.2, 0.05)),
                    border_color: BorderColor(Color::srgb(0.4, 0.4, 0.1)),
                    ..default()
                }).with_children(|bar| {
                    bar.spawn((
                        NodeBundle {
                            style: Style {
                                width: Val::Percent(100.0),
                                height: Val::Percent(100.0),
                                ..default()
                            },
                            background_color: BackgroundColor(Color::srgb(0.95, 0.8, 0.1)),
                            ..default()
                        },
                        VitalsStaminaBarFill,
                    ));
                });

                // Thirst Bar
                vitals_ui.spawn(NodeBundle {
                    style: Style {
                        width: Val::Px(70.0),
                        height: Val::Px(6.0),
                        border: UiRect::all(Val::Px(1.0)),
                        ..default()
                    },
                    background_color: BackgroundColor(Color::srgb(0.05, 0.15, 0.25)),
                    border_color: BorderColor(Color::srgb(0.1, 0.3, 0.5)),
                    ..default()
                }).with_children(|bar| {
                    bar.spawn((
                        NodeBundle {
                            style: Style {
                                width: Val::Percent(100.0),
                                height: Val::Percent(100.0),
                                ..default()
                            },
                            background_color: BackgroundColor(Color::srgb(0.2, 0.6, 0.95)),
                            ..default()
                        },
                        VitalsThirstBarFill,
                    ));
                });

                // Status Text
                vitals_ui.spawn((
                    TextBundle::from_section(
                        "37.0°C | Dry",
                        TextStyle {
                            font_size: 11.0,
                            color: Color::srgb(0.85, 0.85, 0.85),
                            ..default()
                        }
                    ),
                    VitalsStatusText,
                ));
            });

            // ── Hotbar Row ──
            col.spawn(NodeBundle {
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
});
}

pub fn update_hotbar_ui(
    q_player: Query<&Inventory, Changed<Inventory>>,
    registry: Res<ItemRegistry>,
    icon_reg: Option<Res<ItemIconRegistry>>,
    mut q_slots: Query<(&HotbarSlotUi, &mut BackgroundColor, &mut BorderColor, &mut Style, &Children), Without<HotbarDurabilityBarFillNode>>,
    mut q_preview: Query<&mut UiImage, (With<HotbarPreviewNode>, Without<Hotbar2dIconNode>)>,
    mut q_2d_icon: Query<&mut UiImage, (With<Hotbar2dIconNode>, Without<HotbarPreviewNode>)>,
    mut q_count_text: Query<&mut Text, With<HotbarCountTextNode>>,
    q_dur_container: Query<Entity, With<HotbarDurabilityBarContainerNode>>,
    mut q_dur_fill: Query<(&mut Style, &mut BackgroundColor), (With<HotbarDurabilityBarFillNode>, Without<HotbarSlotUi>)>,
    preview_imgs: Option<Res<Ui3dPreviewImages>>,
    mut q_preview_mesh: Query<(Entity, &mut Handle<Mesh>, &mut UiPreviewMesh), Without<HotbarSlotUi>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut q_visibility: Query<&mut Visibility>,
) {
    let Ok(inventory) = q_player.get_single() else { return; };

    // 🚀 1. 動態更新 9 個攝影棚的方塊網格 (具備快取比對，杜絕重複分配)
    for (mesh_entity, mut mesh_handle, mut preview_mesh) in q_preview_mesh.iter_mut() {
        let slot_idx = preview_mesh.slot_index;
        let item_stack = inventory.slot(slot_idx);

        let target_block = if let Some(stack) = item_stack {
            if let Some(def) = registry.get(stack.item_type) {
                if let ItemKind::Block(block_type) = def.kind {
                    block_type
                } else {
                    BlockType::Air
                }
            } else {
                BlockType::Air
            }
        } else {
            BlockType::Air
        };

        let show_mesh = target_block != BlockType::Air;

        if preview_mesh.current_block != target_block {
            preview_mesh.current_block = target_block;
            *mesh_handle = meshes.add(crate::render::greedy::build_single_voxel_mesh(target_block));
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

pub fn update_vitals_hud(
    q_vitals: Query<&crate::player::PlayerVitals, Changed<crate::player::PlayerVitals>>,
    mut q_hp: Query<&mut Style, (With<VitalsHealthBarFill>, Without<VitalsStaminaBarFill>, Without<VitalsThirstBarFill>)>,
    mut q_sta: Query<&mut Style, (With<VitalsStaminaBarFill>, Without<VitalsHealthBarFill>, Without<VitalsThirstBarFill>)>,
    mut q_thirst: Query<&mut Style, (With<VitalsThirstBarFill>, Without<VitalsHealthBarFill>, Without<VitalsStaminaBarFill>)>,
    mut q_text: Query<&mut Text, With<VitalsStatusText>>,
) {
    let Ok(vitals) = q_vitals.get_single() else { return; };

    let hp_pct = (vitals.health / vitals.max_health * 100.0).clamp(0.0, 100.0);
    for mut style in q_hp.iter_mut() {
        style.width = Val::Percent(hp_pct);
    }

    let sta_pct = (vitals.stamina / vitals.max_stamina * 100.0).clamp(0.0, 100.0);
    for mut style in q_sta.iter_mut() {
        style.width = Val::Percent(sta_pct);
    }

    let thirst_pct = (vitals.thirst / vitals.max_thirst * 100.0).clamp(0.0, 100.0);
    for mut style in q_thirst.iter_mut() {
        style.width = Val::Percent(thirst_pct);
    }

    let temp_str = format!("{:.1}°C", vitals.body_temperature);
    let wet_str = if vitals.wetness > 0.05 {
        format!("{:.0}% Wet", vitals.wetness * 100.0)
    } else {
        "Dry".to_string()
    };
    for mut text in q_text.iter_mut() {
        text.sections[0].value = format!("{} | {}", temp_str, wet_str);
    }
}

