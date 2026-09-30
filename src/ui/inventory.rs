use bevy::prelude::*;
use bevy::window::{CursorGrabMode, PrimaryWindow};
use crate::item::{Inventory, ItemRegistry, ItemKind};
use crate::GameState;

#[derive(Resource, Default)]
pub struct InventoryScreenState {
    pub is_open: bool,
}

#[derive(Component)]
pub struct InventoryRootNode;

#[derive(Component)]
pub struct InventorySlotButton {
    pub slot_index: usize,
}

#[derive(Component)]
pub struct SlotItemTextNode {
    pub slot_index: usize,
}

#[derive(Component)]
pub struct SlotCountTextNode {
    pub slot_index: usize,
}

#[derive(Component)]
pub struct SlotDurabilityBarNode {
    pub slot_index: usize,
}

#[derive(Component)]
pub struct CarriedItemTextNode;

pub fn setup_inventory_screen(mut commands: Commands) {
    // 全螢幕遮罩根節點 (預設隱藏)
    commands.spawn((
        NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                position_type: PositionType::Absolute,
                ..default()
            },
            background_color: BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.5)),
            visibility: Visibility::Hidden,
            ..default()
        },
        InventoryRootNode,
    )).with_children(|root| {
        // 主對話盒面板
        root.spawn(NodeBundle {
            style: Style {
                width: Val::Px(450.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                padding: UiRect::all(Val::Px(16.0)),
                row_gap: Val::Px(8.0),
                border: UiRect::all(Val::Px(2.0)),
                ..default()
            },
            background_color: BackgroundColor(Color::srgba(0.12, 0.12, 0.16, 0.95)),
            border_color: BorderColor(Color::srgba(0.35, 0.35, 0.45, 0.9)),
            ..default()
        }).with_children(|panel| {
            // 標題列
            panel.spawn(TextBundle::from_section(
                "INVENTORY (玩家背包)",
                TextStyle {
                    font_size: 18.0,
                    color: Color::srgb(0.9, 0.9, 0.95),
                    ..default()
                },
            ));

            // 游標手持物品狀態指示列
            panel.spawn((
                TextBundle::from_section(
                    "手持游標: 空",
                    TextStyle {
                        font_size: 13.0,
                        color: Color::srgb(0.7, 0.85, 1.0),
                        ..default()
                    },
                ),
                CarriedItemTextNode,
            ));

            // ── 主儲存區 (27 格：Slots 9..35) ──
            panel.spawn(TextBundle::from_section(
                "儲存區 (Main Storage)",
                TextStyle {
                    font_size: 12.0,
                    color: Color::srgb(0.65, 0.65, 0.7),
                    ..default()
                },
            ).with_style(Style {
                align_self: AlignSelf::FlexStart,
                margin: UiRect::top(Val::Px(4.0)),
                ..default()
            }));

            // 3 x 9 網格
            for row in 0..3 {
                panel.spawn(NodeBundle {
                    style: Style {
                        flex_direction: FlexDirection::Row,
                        column_gap: Val::Px(4.0),
                        ..default()
                    },
                    ..default()
                }).with_children(|row_node| {
                    for col in 0..9 {
                        let slot_idx = 9 + row * 9 + col;
                        spawn_slot_button(row_node, slot_idx);
                    }
                });
            }

            // ── 快捷列 (9 格：Slots 0..8) ──
            panel.spawn(TextBundle::from_section(
                "快捷列 (Hotbar)",
                TextStyle {
                    font_size: 12.0,
                    color: Color::srgb(0.65, 0.65, 0.7),
                    ..default()
                },
            ).with_style(Style {
                align_self: AlignSelf::FlexStart,
                margin: UiRect::top(Val::Px(6.0)),
                ..default()
            }));

            panel.spawn(NodeBundle {
                style: Style {
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(4.0),
                    ..default()
                },
                ..default()
            }).with_children(|row_node| {
                for col in 0..9 {
                    spawn_slot_button(row_node, col);
                }
            });

            // 底部提示字樣
            panel.spawn(TextBundle::from_section(
                "[E] 或 [ESC] 關閉背包   |   左鍵點擊槽位可拿起 / 放下 / 堆疊置換",
                TextStyle {
                    font_size: 11.0,
                    color: Color::srgb(0.55, 0.55, 0.6),
                    ..default()
                },
            ).with_style(Style {
                margin: UiRect::top(Val::Px(6.0)),
                ..default()
            }));
        });
    });
}

fn spawn_slot_button(parent: &mut ChildBuilder, slot_index: usize) {
    parent.spawn((
        ButtonBundle {
            style: Style {
                width: Val::Px(42.0),
                height: Val::Px(42.0),
                border: UiRect::all(Val::Px(1.0)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                position_type: PositionType::Relative,
                ..default()
            },
            background_color: BackgroundColor(Color::srgba(0.18, 0.18, 0.22, 0.9)),
            border_color: BorderColor(Color::srgba(0.3, 0.3, 0.38, 0.8)),
            ..default()
        },
        InventorySlotButton { slot_index },
    )).with_children(|slot| {
        // 物品名稱字樣 (縮寫/全名)
        slot.spawn((
            TextBundle::from_section(
                "",
                TextStyle {
                    font_size: 10.0,
                    color: Color::srgb(0.9, 0.9, 0.9),
                    ..default()
                },
            ).with_text_justify(JustifyText::Center),
            SlotItemTextNode { slot_index },
        ));

        // 數量字樣 (右下角)
        slot.spawn((
            TextBundle::from_section(
                "",
                TextStyle {
                    font_size: 11.0,
                    color: Color::WHITE,
                    ..default()
                },
            ).with_style(Style {
                position_type: PositionType::Absolute,
                bottom: Val::Px(2.0),
                right: Val::Px(3.0),
                ..default()
            }),
            SlotCountTextNode { slot_index },
        ));

        // 工具耐久度條
        slot.spawn((
            NodeBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    bottom: Val::Px(1.0),
                    left: Val::Px(4.0),
                    width: Val::Px(34.0),
                    height: Val::Px(2.0),
                    ..default()
                },
                background_color: BackgroundColor(Color::srgb(0.2, 0.9, 0.2)),
                visibility: Visibility::Hidden,
                ..default()
            },
            SlotDurabilityBarNode { slot_index },
        ));
    });
}

pub fn toggle_inventory_screen(
    keys: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<InventoryScreenState>,
    mut q_root: Query<&mut Visibility, With<InventoryRootNode>>,
    mut q_windows: Query<&mut Window, With<PrimaryWindow>>,
    mut q_player: Query<&mut Inventory>,
    registry: Res<ItemRegistry>,
) {
    let press_e = keys.just_pressed(KeyCode::KeyE);
    let press_esc = keys.just_pressed(KeyCode::Escape);

    // 🚀 關鍵防線：只有按下 E 鍵，或在「背包開啟中」按下 ESC 鍵，才觸發背包切換！
    // 若背包處於關閉狀態，ESC 鍵屬於「滑鼠脫離遊戲視窗 (Cursor Release)」，此處絕不攔截與重鎖！
    if !press_e && (!press_esc || !state.is_open) {
        return;
    }

    let Ok(mut window) = q_windows.get_single_mut() else { return; };
    let Ok(mut root_vis) = q_root.get_single_mut() else { return; };
    let Ok(mut inventory) = q_player.get_single_mut() else { return; };

    if press_e {
        state.is_open = !state.is_open;
    } else if press_esc && state.is_open {
        state.is_open = false;
    }

    if state.is_open {
        // 開啟背包：釋放滑鼠游標
        window.cursor.grab_mode = CursorGrabMode::None;
        window.cursor.visible = true;
        *root_vis = Visibility::Inherited;
    } else {
        // 關閉背包：鎖定滑鼠游標並精確置中，自動歸還游標手持物品
        let center = Vec2::new(window.width() / 2.0, window.height() / 2.0);
        window.set_cursor_position(Some(center));
        window.cursor.grab_mode = CursorGrabMode::Locked;
        window.cursor.visible = false;
        *root_vis = Visibility::Hidden;
        inventory.return_carried_item(&registry);
    }
}

pub fn handle_inventory_clicks(
    state: Res<InventoryScreenState>,
    mut q_buttons: Query<(&Interaction, &InventorySlotButton, &mut BackgroundColor), Changed<Interaction>>,
    mut q_player: Query<&mut Inventory>,
    registry: Res<ItemRegistry>,
) {
    if !state.is_open {
        return;
    }

    let Ok(mut inventory) = q_player.get_single_mut() else { return; };

    for (interaction, slot_btn, mut bg) in &mut q_buttons {
        match *interaction {
            Interaction::Pressed => {
                inventory.click_slot(slot_btn.slot_index, &registry);
                bg.0 = Color::srgba(0.35, 0.35, 0.45, 0.9);
            },
            Interaction::Hovered => {
                bg.0 = Color::srgba(0.26, 0.26, 0.32, 0.9);
            },
            Interaction::None => {
                bg.0 = Color::srgba(0.18, 0.18, 0.22, 0.9);
            },
        }
    }
}

pub fn update_inventory_ui(
    state: Res<InventoryScreenState>,
    q_player: Query<&Inventory, Changed<Inventory>>,
    registry: Res<ItemRegistry>,
    mut q_item_texts: Query<(&SlotItemTextNode, &mut Text), Without<SlotCountTextNode>>,
    mut q_count_texts: Query<(&SlotCountTextNode, &mut Text), Without<SlotItemTextNode>>,
    mut q_dur_bars: Query<(&SlotDurabilityBarNode, &mut Style, &mut BackgroundColor, &mut Visibility)>,
    mut q_carried_text: Query<&mut Text, (With<CarriedItemTextNode>, Without<SlotItemTextNode>, Without<SlotCountTextNode>)>,
) {
    if !state.is_open {
        return;
    }

    let Ok(inventory) = q_player.get_single() else { return; };

    // 1. 更新手持游標文字
    if let Ok(mut carried_text) = q_carried_text.get_single_mut() {
        if let Some(ref carried) = inventory.carried_item {
            let name = registry.get(carried.item_type).map(|d| d.name).unwrap_or("Item");
            carried_text.sections[0].value = format!("手持游標: {} x{}", name, carried.count);
            carried_text.sections[0].style.color = Color::srgb(1.0, 0.9, 0.3);
        } else {
            carried_text.sections[0].value = "手持游標: (空)".to_string();
            carried_text.sections[0].style.color = Color::srgb(0.6, 0.65, 0.7);
        }
    }

    // 2. 更新 36 格物品名稱
    for (node, mut text) in &mut q_item_texts {
        if let Some(stack) = inventory.slot(node.slot_index) {
            let name = registry.get(stack.item_type).map(|d| d.name).unwrap_or("");
            // 取簡稱以利 42px 方格美觀排版
            let short_name = match stack.item_type {
                crate::item::ItemType::IronPickaxe => "Iron\nPick",
                crate::item::ItemType::StonePickaxe => "Stone\nPick",
                crate::item::ItemType::WoodenPickaxe => "Wood\nPick",
                crate::item::ItemType::IronShovel => "Iron\nShov",
                crate::item::ItemType::StoneShovel => "Stone\nShov",
                crate::item::ItemType::WoodenShovel => "Wood\nShov",
                crate::item::ItemType::IronAxe => "Iron\nAxe",
                crate::item::ItemType::StoneAxe => "Stone\nAxe",
                crate::item::ItemType::WoodenAxe => "Wood\nAxe",
                _ => name,
            };
            text.sections[0].value = short_name.to_string();
        } else {
            text.sections[0].value = "".to_string();
        }
    }

    // 3. 更新 36 格數量
    for (node, mut text) in &mut q_count_texts {
        if let Some(stack) = inventory.slot(node.slot_index) {
            if stack.count > 1 {
                text.sections[0].value = format!("{}", stack.count);
            } else {
                text.sections[0].value = "".to_string();
            }
        } else {
            text.sections[0].value = "".to_string();
        }
    }

    // 4. 更新耐久度條
    for (node, mut style, mut bg, mut vis) in &mut q_dur_bars {
        if let Some(stack) = inventory.slot(node.slot_index) {
            if let Some(dur) = stack.durability {
                if let Some(def) = registry.get(stack.item_type) {
                    if let ItemKind::Tool { max_durability, .. } = def.kind {
                        let ratio = (dur as f32 / max_durability as f32).clamp(0.0, 1.0);
                        if ratio < 1.0 {
                            *vis = Visibility::Inherited;
                            style.width = Val::Px(34.0 * ratio);
                            if ratio > 0.5 {
                                bg.0 = Color::srgb(0.2, 0.9, 0.2); // 綠
                            } else if ratio > 0.2 {
                                bg.0 = Color::srgb(0.9, 0.8, 0.1); // 黃
                            } else {
                                bg.0 = Color::srgb(0.9, 0.2, 0.1); // 紅
                            }
                            continue;
                        }
                    }
                }
            }
        }
        *vis = Visibility::Hidden;
    }
}

pub struct InventoryUiPlugin;

impl Plugin for InventoryUiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<InventoryScreenState>()
            .add_systems(OnEnter(GameState::InGame), setup_inventory_screen)
            .add_systems(
                Update,
                (
                    toggle_inventory_screen,
                    handle_inventory_clicks,
                    update_inventory_ui,
                ).run_if(in_state(GameState::InGame)),
            );
    }
}
