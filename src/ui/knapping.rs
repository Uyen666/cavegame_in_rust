use bevy::prelude::*;
use bevy::window::{CursorGrabMode, PrimaryWindow};
use crate::item::{Inventory, ItemStack, ItemType, ItemRegistry};
use crate::render::particles::ParticleManager;
use crate::GameState;

/// 經典 5x5 燧石斧刃切削配方 (Flint Axe Head Pattern)
pub const FLINT_AXE_HEAD_PATTERN: [[bool; 5]; 5] = [
    [false, false, true,  false, false],
    [false, true,  true,  true,  false],
    [true,  true,  true,  true,  true ],
    [true,  true,  true,  true,  true ],
    [false, true,  false, true,  false],
];

/// 擊石切削狀態資源 (Knapping State)
#[derive(Resource, Debug, Clone)]
pub struct KnappingState {
    pub is_open: bool,
    pub grid: [[bool; 5]; 5],
}

impl Default for KnappingState {
    fn default() -> Self {
        Self {
            is_open: false,
            grid: [[true; 5]; 5],
        }
    }
}

impl KnappingState {
    pub fn reset_grid(&mut self) {
        self.grid = [[true; 5]; 5];
    }

    pub fn matches_axe_head(&self) -> bool {
        self.grid == FLINT_AXE_HEAD_PATTERN
    }
}

#[derive(Component)]
pub struct KnappingRootNode;

#[derive(Component)]
pub struct KnappingGridButton {
    pub row: usize,
    pub col: usize,
}

#[derive(Component)]
pub struct KnappingButtonText {
    pub row: usize,
    pub col: usize,
}

#[derive(Component)]
pub struct KnappingProgressTextNode;

pub struct KnappingPlugin;

impl Plugin for KnappingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<KnappingState>()
            .add_systems(OnEnter(GameState::InGame), setup_knapping_ui)
            .add_systems(OnExit(GameState::InGame), cleanup_knapping_ui)
            .add_systems(
                Update,
                (
                    handle_knapping_input,
                    handle_knapping_clicks,
                    update_knapping_ui,
                ).run_if(in_state(GameState::InGame)),
            );
    }
}

pub fn setup_knapping_ui(mut commands: Commands) {
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
            background_color: BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.70)),
            visibility: Visibility::Hidden,
            ..default()
        },
        KnappingRootNode,
    )).with_children(|root| {
        // 主面板
        root.spawn(NodeBundle {
            style: Style {
                width: Val::Px(370.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                padding: UiRect::all(Val::Px(16.0)),
                border: UiRect::all(Val::Px(3.0)),
                ..default()
            },
            background_color: BackgroundColor(Color::srgb(0.14, 0.15, 0.17)),
            border_color: BorderColor(Color::srgb(0.40, 0.44, 0.48)),
            ..default()
        }).with_children(|panel| {
            // 標題
            panel.spawn(TextBundle::from_section(
                "FLINT KNAPPING",
                TextStyle {
                    font_size: 20.0,
                    color: Color::srgb(0.95, 0.88, 0.65),
                    ..default()
                },
            ).with_style(Style { margin: UiRect::bottom(Val::Px(4.0)), ..default() }));

            // 清楚指引
            panel.spawn(TextBundle::from_section(
                "Click marked flakes (x) to shape an Axe Head!",
                TextStyle {
                    font_size: 13.0,
                    color: Color::srgb(0.75, 0.78, 0.82),
                    ..default()
                },
            ).with_style(Style { margin: UiRect::bottom(Val::Px(4.0)), ..default() }));

            // 進度動態指示列
            panel.spawn((
                TextBundle::from_section(
                    "Progress: 0 / 11 Flakes Chipped",
                    TextStyle {
                        font_size: 13.0,
                        color: Color::srgb(0.45, 0.85, 0.95),
                        ..default()
                    },
                ).with_style(Style { margin: UiRect::bottom(Val::Px(12.0)), ..default() }),
                KnappingProgressTextNode,
            ));

            // 5x5 石塊切削格點容器
            panel.spawn(NodeBundle {
                style: Style {
                    width: Val::Px(250.0),
                    height: Val::Px(250.0),
                    flex_direction: FlexDirection::Column,
                    justify_content: JustifyContent::SpaceBetween,
                    align_items: AlignItems::Center,
                    padding: UiRect::all(Val::Px(6.0)),
                    border: UiRect::all(Val::Px(2.0)),
                    ..default()
                },
                background_color: BackgroundColor(Color::srgb(0.08, 0.09, 0.10)),
                border_color: BorderColor(Color::srgb(0.28, 0.30, 0.34)),
                ..default()
            }).with_children(|grid_container| {
                for r in 0..5 {
                    grid_container.spawn(NodeBundle {
                        style: Style {
                            width: Val::Percent(100.0),
                            height: Val::Px(44.0),
                            flex_direction: FlexDirection::Row,
                            justify_content: JustifyContent::SpaceBetween,
                            ..default()
                        },
                        ..default()
                    }).with_children(|row_node| {
                        for c in 0..5 {
                            let is_blade = FLINT_AXE_HEAD_PATTERN[r][c];
                            let initial_border = if is_blade {
                                Color::srgb(0.38, 0.58, 0.75) // 斧刃主體為高亮鋼青色邊框
                            } else {
                                Color::srgb(0.65, 0.35, 0.32) // 待敲除石片為微暗紅標記邊框
                            };
                            let initial_symbol = if is_blade { "" } else { "x" };

                            row_node.spawn((
                                ButtonBundle {
                                    style: Style {
                                        width: Val::Px(44.0),
                                        height: Val::Px(44.0),
                                        border: UiRect::all(Val::Px(2.0)),
                                        justify_content: JustifyContent::Center,
                                        align_items: AlignItems::Center,
                                        ..default()
                                    },
                                    background_color: BackgroundColor(Color::srgb(0.30, 0.32, 0.36)),
                                    border_color: BorderColor(initial_border),
                                    ..default()
                                },
                                KnappingGridButton { row: r, col: c },
                            )).with_children(|btn| {
                                btn.spawn((
                                    TextBundle::from_section(
                                        initial_symbol,
                                        TextStyle {
                                            font_size: 15.0,
                                            color: Color::srgba(0.85, 0.45, 0.40, 0.9),
                                            ..default()
                                        },
                                    ),
                                    KnappingButtonText { row: r, col: c },
                                ));
                            });
                        }
                    });
                }
            });

            // 底部操作提示
            panel.spawn(TextBundle::from_section(
                "[LMB] Chip Flake   |   [Esc] Cancel",
                TextStyle {
                    font_size: 12.0,
                    color: Color::srgb(0.55, 0.58, 0.62),
                    ..default()
                },
            ).with_style(Style { margin: UiRect::top(Val::Px(12.0)), ..default() }));
        });
    });
}

pub fn cleanup_knapping_ui(
    mut commands: Commands,
    q_root: Query<Entity, With<KnappingRootNode>>,
) {
    for entity in q_root.iter() {
        commands.entity(entity).despawn_recursive();
    }
}

pub fn handle_knapping_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<KnappingState>,
    mut q_root: Query<&mut Visibility, With<KnappingRootNode>>,
    mut q_windows: Query<&mut Window, With<PrimaryWindow>>,
) {
    if !state.is_open {
        return;
    }

    // 按 Esc 退出 Knapping 介面
    if keys.just_pressed(KeyCode::Escape) {
        state.is_open = false;
        if let Ok(mut vis) = q_root.get_single_mut() {
            *vis = Visibility::Hidden;
        }
        if let Ok(mut window) = q_windows.get_single_mut() {
            if window.focused {
                window.cursor.grab_mode = CursorGrabMode::Locked;
                window.cursor.visible = false;
            }
        }
    }
}

pub fn handle_knapping_clicks(
    mut interaction_query: Query<
        (&Interaction, &KnappingGridButton, &mut BackgroundColor, &mut BorderColor),
        (Changed<Interaction>, With<KnappingGridButton>),
    >,
    mut state: ResMut<KnappingState>,
    mut q_player: Query<(&Transform, &mut Inventory), With<crate::player::Player>>,
    mut particle_mgr: ResMut<ParticleManager>,
    mut q_root: Query<&mut Visibility, With<KnappingRootNode>>,
    mut q_windows: Query<&mut Window, With<PrimaryWindow>>,
) {
    if !state.is_open {
        return;
    }

    let Ok((p_transform, mut inventory)) = q_player.get_single_mut() else { return; };
    let registry = ItemRegistry;

    for (interaction, btn, mut bg, mut border) in interaction_query.iter_mut() {
        let (r, c) = (btn.row, btn.col);
        if *interaction == Interaction::Pressed {
            if state.grid[r][c] {
                // 點擊敲碎該格石片！
                state.grid[r][c] = false;
                bg.0 = Color::srgba(0.08, 0.08, 0.09, 0.25);
                border.0 = BorderColor(Color::srgba(0.18, 0.20, 0.22, 0.3)).0;

                // 噴散清脆深灰石屑與微弱火花粒子
                let flint_color = Color::srgb(0.35, 0.36, 0.40);
                particle_mgr.spawn_debris(p_transform.translation + Vec3::new(0.0, 1.2, 0.0), flint_color, 4);

                // 配方比對演算法：檢查是否吻合斧刃
                if state.matches_axe_head() {
                    // 扣除手持的 2 塊燧石
                    let has_enough_flint = inventory.selected_item().map_or(false, |it| it.item_type == ItemType::Flint && it.count >= 2);
                    if has_enough_flint {
                        inventory.consume_selected(2);
                        let axe_head = ItemStack::new(ItemType::FlintAxeHead, 1, &registry);
                        if let Some(overflow) = inventory.add_item(axe_head, &registry) {
                            info!("【擊石切削】背包已滿，溢出物品 {:?}", overflow);
                        }

                        // 爆發金黃與石質成功火花
                        let spark_color = Color::srgb(0.95, 0.85, 0.45);
                        particle_mgr.spawn_debris(p_transform.translation + Vec3::new(0.0, 1.2, 0.0), spark_color, 16);

                        println!("【擊石工藝】成功切削出 燧石斧刃 (Flint Axe Head)！");
                        println!("【提示】手持燧石斧刃且背包內有木棒時點擊右鍵即可組裝「燧石斧」！");

                        // 關閉介面並重新鎖定游標
                        state.is_open = false;
                        if let Ok(mut vis) = q_root.get_single_mut() {
                            *vis = Visibility::Hidden;
                        }
                        if let Ok(mut window) = q_windows.get_single_mut() {
                            if window.focused {
                                window.cursor.grab_mode = CursorGrabMode::Locked;
                                window.cursor.visible = false;
                            }
                        }
                        break;
                    }
                }
            }
        }
    }
}

pub fn update_knapping_ui(
    state: Res<KnappingState>,
    mut q_buttons: Query<(&KnappingGridButton, &mut BackgroundColor, &mut BorderColor)>,
    mut q_btn_texts: Query<(&KnappingButtonText, &mut Text), Without<KnappingProgressTextNode>>,
    mut q_progress_text: Query<&mut Text, With<KnappingProgressTextNode>>,
) {
    if !state.is_open {
        return;
    }

    // 計算敲除進度
    let chipped_correct = (0..5).flat_map(|r| (0..5).map(move |c| (r, c)))
        .filter(|&(r, c)| !FLINT_AXE_HEAD_PATTERN[r][c] && !state.grid[r][c])
        .count();

    let chipped_ruined = (0..5).flat_map(|r| (0..5).map(move |c| (r, c)))
        .any(|(r, c)| FLINT_AXE_HEAD_PATTERN[r][c] && !state.grid[r][c]);

    if let Ok(mut prog_text) = q_progress_text.get_single_mut() {
        if chipped_ruined {
            prog_text.sections[0].value = "Ruined! Core blade chipped! Press Esc to cancel".to_string();
            prog_text.sections[0].style.color = Color::srgb(0.95, 0.35, 0.30);
        } else if chipped_correct == 11 {
            prog_text.sections[0].value = "Pattern Complete! Flint Axe Head formed!".to_string();
            prog_text.sections[0].style.color = Color::srgb(0.95, 0.85, 0.35);
        } else {
            prog_text.sections[0].value = format!("Progress: {} / 11 Flakes Chipped", chipped_correct);
            prog_text.sections[0].style.color = Color::srgb(0.45, 0.85, 0.95);
        }
    }

    for (btn, mut bg, mut border) in q_buttons.iter_mut() {
        let (r, c) = (btn.row, btn.col);
        let is_blade = FLINT_AXE_HEAD_PATTERN[r][c];
        if state.grid[r][c] {
            bg.0 = Color::srgb(0.30, 0.32, 0.36);
            border.0 = if is_blade {
                BorderColor(Color::srgb(0.38, 0.58, 0.75)).0
            } else {
                BorderColor(Color::srgb(0.65, 0.35, 0.32)).0
            };
        } else {
            bg.0 = Color::srgba(0.08, 0.08, 0.09, 0.25);
            border.0 = BorderColor(Color::srgba(0.18, 0.20, 0.22, 0.3)).0;
        }
    }

    for (node, mut text) in q_btn_texts.iter_mut() {
        let (r, c) = (node.row, node.col);
        let is_blade = FLINT_AXE_HEAD_PATTERN[r][c];
        if state.grid[r][c] {
            text.sections[0].value = if is_blade { "".to_string() } else { "x".to_string() };
        } else {
            text.sections[0].value = "".to_string();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::item::{ItemRegistry, ItemType, ItemKind};
    use crate::world::registry::{ToolType, ToolTier};
    use crate::world::BlockType;

    #[test]
    fn test_knapping_pattern_matching() {
        let mut state = KnappingState::default();
        // 初始狀態全為 true，不應符合斧刃形狀
        assert!(!state.matches_axe_head(), "初始全滿石塊不應匹配斧刃");

        // 模擬玩家依照 FLINT_AXE_HEAD_PATTERN 逐格敲除四周多餘石片
        let to_remove = [
            (0, 0), (0, 1), (0, 3), (0, 4),
            (1, 0), (1, 4),
            (4, 0), (4, 2), (4, 4),
        ];
        for (r, c) in to_remove {
            state.grid[r][c] = false;
        }

        // 精確吻合 5x5 燧石斧刃切削形狀
        assert!(state.matches_axe_head(), "敲除多餘石片後應精確匹配燧石斧刃配方");

        // 若多敲損毀一塊核心石料 (例如 Row 2, Col 2)
        state.grid[2][2] = false;
        assert!(!state.matches_axe_head(), "過度敲損斧刃本體應判定失敗");

        // 測試重設
        state.reset_grid();
        assert!(!state.matches_axe_head());
        assert_eq!(state.grid, [[true; 5]; 5], "reset_grid 應重設整塊 5x5 石板");
    }

    #[test]
    fn test_flint_axe_and_head_registry() {
        let registry = ItemRegistry;
        let head_def = registry.get(ItemType::FlintAxeHead).expect("FlintAxeHead 必須註冊");
        assert_eq!(head_def.name, "Flint Axe Head");
        assert_eq!(head_def.kind, ItemKind::Material);
        assert_eq!(head_def.max_stack, 16);

        let axe_def = registry.get(ItemType::FlintAxe).expect("FlintAxe 必須註冊");
        assert_eq!(axe_def.name, "Flint Axe");
        assert_eq!(axe_def.max_stack, 1);
        if let ItemKind::Tool { tool_type, tier, efficiency, max_durability } = axe_def.kind {
            assert_eq!(tool_type, ToolType::Axe, "燧石斧應屬於斧頭工具類別");
            assert_eq!(tier, ToolTier::Wood, "燧石斧採伐等級應為木級 (Wood Tier)");
            assert!((efficiency - 2.0).abs() < 1e-4, "燧石斧砍伐效率應為 2.0");
            assert_eq!(max_durability, 45, "燧石斧耐久度應為 45");
        } else {
            panic!("FlintAxe 必須被註冊為 Tool 類別！");
        }
    }

    #[test]
    fn test_flint_axe_can_harvest_oak_log() {
        let registry = ItemRegistry;
        let axe_def = registry.get(ItemType::FlintAxe).expect("FlintAxe 必須存在");
        let oak_log = BlockType::OakLog;

        assert_eq!(oak_log.preferred_tool(), ToolType::Axe, "橡木原木偏好工具為斧頭");
        assert_eq!(oak_log.required_tier(), ToolTier::Wood, "橡木原木採收門檻為木級工具");

        if let ItemKind::Tool { tool_type, tier, .. } = axe_def.kind {
            assert_eq!(tool_type, oak_log.preferred_tool());
            assert!(tier >= oak_log.required_tier(), "燧石斧必須能夠有效收割橡木原木 (OakLog)");
        }
    }
}
