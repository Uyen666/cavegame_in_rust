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
pub struct SlotIconNode {
    pub slot_index: usize,
}

#[derive(Component)]
pub struct CarriedItemTextNode;

#[derive(Component)]
pub struct FloatingCarriedItemNode;

#[derive(Component)]
pub struct FloatingCarriedIconNode;

#[derive(Component)]
pub struct FloatingCarriedCountNode;

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
                "INVENTORY",
                TextStyle {
                    font_size: 18.0,
                    color: Color::srgb(0.9, 0.9, 0.95),
                    ..default()
                },
            ));

            // 游標手持物品狀態指示列
            panel.spawn((
                TextBundle::from_section(
                    "Carried: (Empty)",
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
                "MAIN STORAGE",
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
                "HOTBAR",
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
                "[E] / [ESC] Close   |   [LMB] Pick / Place / Swap",
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

        // 🚀 游標跟手浮動物品節點 (跟隨滑鼠游標絕對座標移動)
        root.spawn((
            NodeBundle {
                style: Style {
                    width: Val::Px(36.0),
                    height: Val::Px(36.0),
                    position_type: PositionType::Absolute,
                    left: Val::Px(0.0),
                    top: Val::Px(0.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                focus_policy: bevy::ui::FocusPolicy::Pass,
                z_index: ZIndex::Global(100),
                visibility: Visibility::Hidden,
                ..default()
            },
            FloatingCarriedItemNode,
        )).with_children(|carried_node| {
            // 跟手 3D/2D 圖示
            carried_node.spawn((
                ImageBundle {
                    style: Style {
                        width: Val::Px(32.0),
                        height: Val::Px(32.0),
                        ..default()
                    },
                    focus_policy: bevy::ui::FocusPolicy::Pass,
                    image: UiImage::default(),
                    ..default()
                },
                FloatingCarriedIconNode,
            ));

            // 跟手數量數字 (右下角)
            carried_node.spawn((
                TextBundle::from_section(
                    "",
                    TextStyle {
                        font_size: 13.0,
                        color: Color::WHITE,
                        ..default()
                    },
                ).with_style(Style {
                    position_type: PositionType::Absolute,
                    bottom: Val::Px(0.0),
                    right: Val::Px(2.0),
                    ..default()
                }),
                FloatingCarriedCountNode,
            ));
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
        // 物品圖標 (Icon: 方塊為 3D 等角預覽，材料為 2D)
        slot.spawn((
            ImageBundle {
                style: Style {
                    width: Val::Px(32.0),
                    height: Val::Px(32.0),
                    position_type: PositionType::Absolute,
                    ..default()
                },
                image: UiImage::default(),
                visibility: Visibility::Hidden,
                ..default()
            },
            SlotIconNode { slot_index },
        ));

        // 物品名稱字樣 (縮寫/全名，無 Icon 時作為回退顯示)
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
        // 關閉背包：若視窗處於聚焦狀態則置中鎖定滑鼠游標，未聚焦則絕不鎖定，並自動歸還游標手持物品
        let center = Vec2::new(window.width() / 2.0, window.height() / 2.0);
        window.set_cursor_position(Some(center));
        if window.focused {
            window.cursor.grab_mode = CursorGrabMode::Locked;
            window.cursor.visible = false;
        } else {
            window.cursor.grab_mode = CursorGrabMode::None;
            window.cursor.visible = true;
        }
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
    q_player: Query<&Inventory>,
    registry: Res<ItemRegistry>,
    icon_reg: Option<Res<crate::ui::hud::ItemIconRegistry>>,
    q_windows: Query<&Window, With<PrimaryWindow>>,
    mut q_icons: Query<(&SlotIconNode, &mut UiImage, &mut Visibility), (Without<SlotDurabilityBarNode>, Without<FloatingCarriedItemNode>, Without<FloatingCarriedIconNode>)>,
    mut q_item_texts: Query<(&SlotItemTextNode, &mut Text), (Without<SlotCountTextNode>, Without<CarriedItemTextNode>, Without<FloatingCarriedCountNode>)>,
    mut q_count_texts: Query<(&SlotCountTextNode, &mut Text), (Without<SlotItemTextNode>, Without<CarriedItemTextNode>, Without<FloatingCarriedCountNode>)>,
    mut q_dur_bars: Query<(&SlotDurabilityBarNode, &mut Style, &mut BackgroundColor, &mut Visibility), (Without<SlotIconNode>, Without<FloatingCarriedItemNode>)>,
    mut q_carried_text: Query<&mut Text, (With<CarriedItemTextNode>, Without<SlotItemTextNode>, Without<SlotCountTextNode>, Without<FloatingCarriedCountNode>)>,
    q_hovered_slots: Query<(&InventorySlotButton, &Interaction)>,
    mut q_floating_node: Query<(&mut Style, &mut Visibility), (With<FloatingCarriedItemNode>, Without<SlotIconNode>, Without<SlotDurabilityBarNode>)>,
    mut q_floating_icon: Query<&mut UiImage, (With<FloatingCarriedIconNode>, Without<SlotIconNode>)>,
    mut q_floating_count: Query<&mut Text, (With<FloatingCarriedCountNode>, Without<SlotItemTextNode>, Without<SlotCountTextNode>, Without<CarriedItemTextNode>)>,
) {
    if !state.is_open {
        return;
    }

    let Ok(inventory) = q_player.get_single() else { return; };

    // 1. 更新手持游標文字與 Hover 提示列
    if let Ok(mut carried_text) = q_carried_text.get_single_mut() {
        if let Some(ref carried) = inventory.carried_item {
            let name = registry.get(carried.item_type).map(|d| d.name).unwrap_or("Item");
            carried_text.sections[0].value = format!("Carried: {} x{}", name, carried.count);
            carried_text.sections[0].style.color = Color::srgb(1.0, 0.9, 0.3);
        } else {
            // 游標未手持物品時，檢查是否有 Hover 的槽位
            let mut hovered_item_info = None;
            for (btn, interaction) in &q_hovered_slots {
                if *interaction == Interaction::Hovered {
                    if let Some(stack) = inventory.slot(btn.slot_index) {
                        let name = registry.get(stack.item_type).map(|d| d.name).unwrap_or("Item");
                        if let Some(dur) = stack.durability {
                            if let Some(def) = registry.get(stack.item_type) {
                                if let ItemKind::Tool { max_durability, .. } = def.kind {
                                    hovered_item_info = Some(format!("Hover: {} ({}/{})", name, dur, max_durability));
                                    break;
                                }
                            }
                        }
                        hovered_item_info = Some(format!("Hover: {} x{}", name, stack.count));
                        break;
                    }
                }
            }

            if let Some(info) = hovered_item_info {
                carried_text.sections[0].value = info;
                carried_text.sections[0].style.color = Color::srgb(0.7, 0.85, 1.0);
            } else {
                carried_text.sections[0].value = "Carried: (Empty)".to_string();
                carried_text.sections[0].style.color = Color::srgb(0.6, 0.65, 0.7);
            }
        }
    }

    // 2. 更新 36 格圖標 (Icons)
    for (node, mut icon_image, mut icon_vis) in &mut q_icons {
        if let Some(stack) = inventory.slot(node.slot_index) {
            if let Some(ref reg) = icon_reg {
                if let Some(icon_handle) = reg.icons.get(&stack.item_type) {
                    icon_image.texture = icon_handle.clone();
                    icon_image.color = Color::WHITE;
                    *icon_vis = Visibility::Inherited;
                    continue;
                }
            }
        }
        *icon_vis = Visibility::Hidden;
    }

    // 3. 更新 36 格文字 (有 Icon 則隱藏文字；無 Icon 如方塊則顯示簡稱)
    for (node, mut text) in &mut q_item_texts {
        if let Some(stack) = inventory.slot(node.slot_index) {
            let has_icon = icon_reg.as_ref().map_or(false, |reg| reg.icons.contains_key(&stack.item_type));
            if has_icon {
                text.sections[0].value = "".to_string();
            } else {
                let name = registry.get(stack.item_type).map(|d| d.name).unwrap_or("");
                let short_name = match stack.item_type {
                    crate::item::ItemType::OakLog => "Log",
                    crate::item::ItemType::OakLeaves => "Leaves",
                    crate::item::ItemType::Grass => "Grass",
                    crate::item::ItemType::Glass => "Glass",
                    crate::item::ItemType::CoalOre => "Coal\nOre",
                    crate::item::ItemType::IronOre => "Iron\nOre",
                    _ => name,
                };
                text.sections[0].value = short_name.to_string();
            }
        } else {
            text.sections[0].value = "".to_string();
        }
    }

    // 4. 更新 36 格數量
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

    // 5. 更新耐久度條
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

    // 6. 🚀 游標跟手浮動物品更新 (緊跟滑鼠游標絕對座標移動)
    if let Ok((mut float_style, mut float_vis)) = q_floating_node.get_single_mut() {
        if let Some(ref carried) = inventory.carried_item {
            let cursor_pos = q_windows.get_single().ok().and_then(|w| w.cursor_position());
            if let Some(pos) = cursor_pos {
                float_style.left = Val::Px(pos.x - 18.0);
                float_style.top = Val::Px(pos.y - 18.0);
                *float_vis = Visibility::Inherited;

                if let Ok(mut icon_img) = q_floating_icon.get_single_mut() {
                    if let Some(ref reg) = icon_reg {
                        if let Some(icon_handle) = reg.icons.get(&carried.item_type) {
                            icon_img.texture = icon_handle.clone();
                            icon_img.color = Color::WHITE;
                        } else {
                            icon_img.texture = reg.fallback.clone();
                            icon_img.color = Color::WHITE;
                        }
                    }
                }

                if let Ok(mut count_text) = q_floating_count.get_single_mut() {
                    if carried.count > 1 {
                        count_text.sections[0].value = format!("{}", carried.count);
                    } else {
                        count_text.sections[0].value = "".to_string();
                    }
                }
            } else {
                *float_vis = Visibility::Hidden;
            }
        } else {
            *float_vis = Visibility::Hidden;
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::item::{ItemStack, ItemType};

    #[test]
    fn test_all_11_preview_blocks_have_item_types() {
        for &block in crate::ui::hud::PREVIEW_BLOCKS {
            let item = ItemType::from_block(block);
            assert!(item.is_some(), "所有預覽方塊 {:?} 必須具有對應的 ItemType", block);
        }
    }

    #[test]
    fn test_carried_item_return_and_slot_swap() {
        let registry = ItemRegistry::default();
        let mut inv = Inventory::new(36);
        inv.set_slot(0, Some(ItemStack::new(ItemType::Dirt, 32, &registry)));

        // 點擊 slot 0 拿起物品
        inv.click_slot(0, &registry);
        assert!(inv.slot(0).is_none(), "點擊後原格子應被清空");
        let carried = inv.carried_item.as_ref().expect("游標應手持拿起的物品");
        assert_eq!(carried.item_type, ItemType::Dirt);
        assert_eq!(carried.count, 32);

        // 關閉背包時自動歸還
        inv.return_carried_item(&registry);
        assert!(inv.carried_item.is_none(), "歸還後游標手持應為 None");
        assert_eq!(inv.slot(0).as_ref().map(|s| s.count), Some(32), "物品應歸還至第 0 格");
    }
}
