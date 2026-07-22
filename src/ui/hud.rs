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

// 🚀 快捷列 UI 標記與材質資源
#[derive(Resource)]
pub struct HotbarTextures {
    pub stone: Handle<Image>,
    pub grass: Handle<Image>,
}

pub fn load_hotbar_textures(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.insert_resource(HotbarTextures {
        stone: asset_server.load("textures/stone.png"),
        grass: asset_server.load("textures/grass.png"),
    });
}

#[derive(Component)]
pub struct HotbarUi;

#[derive(Component)]
pub struct HotbarSlotUi {
    pub slot_index: usize,
}

#[derive(Component)]
pub struct HotbarNameText;

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

                    // 中央方塊代表色塊 / 圖示預覽 (改用 ImageBundle 支援貼圖)
                    slot.spawn((
                        ImageBundle {
                            style: Style {
                                width: Val::Px(24.0),
                                height: Val::Px(24.0),
                                position_type: PositionType::Absolute,
                                ..default()
                            },
                            background_color: BackgroundColor(Color::NONE),
                            ..default()
                        },
                        HotbarPreviewNode,
                    ));

                    // 底部名稱文字
                    slot.spawn((
                        TextBundle::from_section(
                            "",
                            TextStyle {
                                font_size: 12.0,
                                color: Color::WHITE,
                                ..default()
                            }
                        ).with_style(Style {
                            position_type: PositionType::Absolute,
                            bottom: Val::Px(2.0),
                            ..default()
                        }),
                        HotbarNameText,
                    ));
                });
            }
        });
    });
}

pub fn update_hotbar_ui(
    q_player: Query<&Player, Changed<Player>>, // 僅在 Player 狀態改變時響應
    mut q_slots: Query<(&HotbarSlotUi, &mut BackgroundColor, &mut BorderColor, &mut Style, &Children)>,
    mut q_text: Query<&mut Text, With<HotbarNameText>>,
    mut q_preview: Query<&mut UiImage, (With<HotbarPreviewNode>, Without<HotbarSlotUi>)>,
    textures: Option<Res<HotbarTextures>>,
) {
    let Ok(player) = q_player.get_single() else { return; };

    for (slot, mut bg_color, mut border_color, mut style, children) in q_slots.iter_mut() {
        let is_selected = slot.slot_index == player.selected_slot;
        let block_type = player.hotbar[slot.slot_index];

        // 🚀 動態更新高亮邊框：選中的 Slot 加粗並亮黃色，未選中的保持暗灰色
        if is_selected {
            style.border = UiRect::all(Val::Px(3.0));
            *bg_color = BackgroundColor(Color::srgba(0.3, 0.3, 0.3, 0.8));
            *border_color = BorderColor(Color::srgb(1.0, 0.84, 0.0));
        } else {
            style.border = UiRect::all(Val::Px(1.0));
            *bg_color = BackgroundColor(Color::srgba(0.1, 0.1, 0.1, 0.5));
            *border_color = BorderColor(Color::srgba(0.3, 0.3, 0.3, 0.5));
        }

        // 🚀 同步顯示當前格子的圖示與文字
        for &child in children.iter() {
            if let Ok(mut text) = q_text.get_mut(child) {
                if block_type == BlockType::Air {
                    text.sections[0].value = "".to_string(); // 絕殺 Air 字樣
                } else {
                    text.sections[0].value = format!("{:?}", block_type);
                }
            }
            if let Ok(mut preview_image) = q_preview.get_mut(child) {
                match block_type {
                    BlockType::Air => {
                        preview_image.texture = Handle::default();
                        preview_image.color = Color::NONE; // 🚀 空氣完全透明
                    }
                    BlockType::Stone => {
                        if let Some(tex) = &textures {
                            preview_image.texture = tex.stone.clone();
                            preview_image.color = Color::WHITE; // 取消色塊遮罩，顯示原圖
                        } else {
                            preview_image.texture = Handle::default();
                            preview_image.color = Color::srgb(0.6, 0.6, 0.6); // 降級水泥灰
                        }
                    }
                    BlockType::Grass => {
                        if let Some(tex) = &textures {
                            preview_image.texture = tex.grass.clone();
                            preview_image.color = Color::WHITE;
                        } else {
                            preview_image.texture = Handle::default();
                            preview_image.color = Color::srgb(0.2, 0.8, 0.2); // 降級鮮草綠
                        }
                    }
                    BlockType::Dirt => {
                        // Dirt 目前尚無貼圖檔案，維持向量色塊
                        preview_image.texture = Handle::default();
                        preview_image.color = Color::srgb(0.54, 0.27, 0.07); // 泥土棕
                    }
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
