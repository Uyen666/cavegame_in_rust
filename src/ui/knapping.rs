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

/// 經典 5x5 燧石鎬頭切削配方 (Flint Pickaxe Head Pattern)
pub const FLINT_PICKAXE_HEAD_PATTERN: [[bool; 5]; 5] = [
    [true,  false, false, false, true ],
    [true,  true,  false, true,  true ],
    [false, true,  true,  true,  false],
    [false, false, true,  false, false],
    [false, false, true,  false, false],
];

/// 經典 5x5 燧石鏟刃切削配方 (Flint Shovel Head Pattern)
pub const FLINT_SHOVEL_HEAD_PATTERN: [[bool; 5]; 5] = [
    [false, true,  true,  true,  false],
    [true,  true,  true,  true,  true ],
    [true,  true,  true,  true,  true ],
    [false, true,  true,  true,  false],
    [false, false, true,  false, false],
];

/// 經典 5x5 燧石刀刃切削配方 (Flint Knife Blade Pattern)
pub const FLINT_KNIFE_BLADE_PATTERN: [[bool; 5]; 5] = [
    [false, false, true,  false, false],
    [false, false, true,  true,  false],
    [false, false, true,  true,  false],
    [false, true,  true,  false, false],
    [false, true,  true,  false, false],
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KnappingRecipe {
    AxeHead,
    PickaxeHead,
    ShovelHead,
    KnifeBlade,
}

impl KnappingRecipe {
    pub const ALL: [Self; 4] = [
        Self::AxeHead,
        Self::PickaxeHead,
        Self::ShovelHead,
        Self::KnifeBlade,
    ];

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::AxeHead => "Axe Head",
            Self::PickaxeHead => "Pickaxe Head",
            Self::ShovelHead => "Shovel Head",
            Self::KnifeBlade => "Knife Blade",
        }
    }

    pub fn output_item(&self) -> ItemType {
        match self {
            Self::AxeHead => ItemType::FlintAxeHead,
            Self::PickaxeHead => ItemType::FlintPickaxeHead,
            Self::ShovelHead => ItemType::FlintShovelHead,
            Self::KnifeBlade => ItemType::FlintKnifeBlade,
        }
    }

    pub fn pattern(&self) -> [[bool; 5]; 5] {
        match self {
            Self::AxeHead => FLINT_AXE_HEAD_PATTERN,
            Self::PickaxeHead => FLINT_PICKAXE_HEAD_PATTERN,
            Self::ShovelHead => FLINT_SHOVEL_HEAD_PATTERN,
            Self::KnifeBlade => FLINT_KNIFE_BLADE_PATTERN,
        }
    }

    pub fn waste_count(&self) -> usize {
        let pat = self.pattern();
        (0..5).flat_map(|r| (0..5).map(move |c| (r, c))).filter(|&(r, c)| !pat[r][c]).count()
    }
}

/// 擊石切削狀態資源 (Knapping State)
#[derive(Resource, Debug, Clone)]
pub struct KnappingState {
    pub is_open: bool,
    pub active_recipe: KnappingRecipe,
    pub grid: [[bool; 5]; 5],
}

impl Default for KnappingState {
    fn default() -> Self {
        Self {
            is_open: false,
            active_recipe: KnappingRecipe::AxeHead,
            grid: [[true; 5]; 5],
        }
    }
}

impl KnappingState {
    pub fn reset_grid(&mut self) {
        self.grid = [[true; 5]; 5];
    }

    pub fn matches_active_recipe(&self) -> bool {
        self.grid == self.active_recipe.pattern()
    }
}

#[derive(Component)]
pub struct KnappingRootNode;

#[derive(Component)]
pub struct KnappingRecipeTabButton {
    pub recipe: KnappingRecipe,
}

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

#[derive(Component)]
pub struct KnappingInstructionTextNode;

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
        // Main Knapping Dialog Panel
        root.spawn(NodeBundle {
            style: Style {
                width: Val::Px(390.0),
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
            // Title Header
            panel.spawn(TextBundle::from_section(
                "FLINT KNAPPING",
                TextStyle {
                    font_size: 20.0,
                    color: Color::srgb(0.95, 0.88, 0.65),
                    ..default()
                },
            ).with_style(Style { margin: UiRect::bottom(Val::Px(4.0)), ..default() }));

            // Recipe Selection Tab Row
            panel.spawn(NodeBundle {
                style: Style {
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(6.0),
                    margin: UiRect::vertical(Val::Px(4.0)),
                    ..default()
                },
                ..default()
            }).with_children(|tab_row| {
                for &recipe in &KnappingRecipe::ALL {
                    let is_default = recipe == KnappingRecipe::AxeHead;
                    tab_row.spawn((
                        ButtonBundle {
                            style: Style {
                                padding: UiRect::axes(Val::Px(8.0), Val::Px(4.0)),
                                border: UiRect::all(Val::Px(1.5)),
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                ..default()
                            },
                            background_color: BackgroundColor(if is_default {
                                Color::srgb(0.25, 0.45, 0.65)
                            } else {
                                Color::srgb(0.18, 0.20, 0.24)
                            }),
                            border_color: BorderColor(if is_default {
                                Color::srgb(0.45, 0.75, 0.95)
                            } else {
                                Color::srgb(0.32, 0.35, 0.40)
                            }),
                            ..default()
                        },
                        KnappingRecipeTabButton { recipe },
                    )).with_children(|btn| {
                        btn.spawn(TextBundle::from_section(
                            recipe.display_name(),
                            TextStyle {
                                font_size: 11.5,
                                color: Color::srgb(0.90, 0.92, 0.96),
                                ..default()
                            },
                        ));
                    });
                }
            });

            // Action Instruction
            panel.spawn((
                TextBundle::from_section(
                    "Click marked flakes (x) to shape an Axe Head!",
                    TextStyle {
                        font_size: 13.0,
                        color: Color::srgb(0.75, 0.78, 0.82),
                        ..default()
                    },
                ).with_style(Style { margin: UiRect::vertical(Val::Px(4.0)), ..default() }),
                KnappingInstructionTextNode,
            ));

            // Dynamic Progress Indicator
            panel.spawn((
                TextBundle::from_section(
                    "Progress: 0 / 11 Flakes Chipped",
                    TextStyle {
                        font_size: 13.0,
                        color: Color::srgb(0.45, 0.85, 0.95),
                        ..default()
                    },
                ).with_style(Style { margin: UiRect::bottom(Val::Px(10.0)), ..default() }),
                KnappingProgressTextNode,
            ));

            // 5x5 Stone Grid Container
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
                                Color::srgb(0.38, 0.58, 0.75) // Blade core highlighted steel blue
                            } else {
                                Color::srgb(0.65, 0.35, 0.32) // Waste flake soft red
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

            // Footer Control Prompt
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

    // Press Esc to exit Knapping interface
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
    mut grid_interaction_query: Query<
        (&Interaction, &KnappingGridButton, &mut BackgroundColor, &mut BorderColor),
        (Changed<Interaction>, With<KnappingGridButton>),
    >,
    mut tab_interaction_query: Query<
        (&Interaction, &KnappingRecipeTabButton),
        (Changed<Interaction>, With<KnappingRecipeTabButton>),
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

    // 1. Recipe Tab Selection
    for (interaction, tab_btn) in tab_interaction_query.iter_mut() {
        if *interaction == Interaction::Pressed {
            if state.active_recipe != tab_btn.recipe {
                state.active_recipe = tab_btn.recipe;
                state.reset_grid();
            }
        }
    }

    // 2. 5x5 Stone Flake Chipping
    let Ok((p_transform, mut inventory)) = q_player.get_single_mut() else { return; };
    let registry = ItemRegistry;

    for (interaction, btn, mut bg, mut border) in grid_interaction_query.iter_mut() {
        let (r, c) = (btn.row, btn.col);
        if *interaction == Interaction::Pressed {
            if state.grid[r][c] {
                // Chip this stone flake!
                state.grid[r][c] = false;
                bg.0 = Color::srgba(0.08, 0.08, 0.09, 0.25);
                border.0 = BorderColor(Color::srgba(0.18, 0.20, 0.22, 0.3)).0;

                // Emit flint chipping debris particles
                let flint_color = Color::srgb(0.35, 0.36, 0.40);
                particle_mgr.spawn_debris(p_transform.translation + Vec3::new(0.0, 1.2, 0.0), flint_color, 4);

                // Recipe Pattern Matching
                if state.matches_active_recipe() {
                    let has_enough_flint = inventory.selected_item().map_or(false, |it| it.item_type == ItemType::Flint && it.count >= 2);
                    if has_enough_flint {
                        inventory.consume_selected(2);
                        let output_item = state.active_recipe.output_item();
                        let crafted = ItemStack::new(output_item, 1, &registry);
                        if let Some(overflow) = inventory.add_item(crafted, &registry) {
                            info!("[Knapping] Inventory full, dropped item {:?}", overflow);
                        }

                        // Burst golden sparks on successful crafting
                        let spark_color = Color::srgb(0.95, 0.85, 0.45);
                        particle_mgr.spawn_debris(p_transform.translation + Vec3::new(0.0, 1.2, 0.0), spark_color, 16);

                        println!("[Crafting] Successfully crafted {}!", state.active_recipe.display_name());
                        println!("[Tip] Hold the tool head with a Stick in inventory and Right-Click to assemble!");

                        // Close UI and re-lock cursor
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
    mut q_tabs: Query<(&KnappingRecipeTabButton, &mut BackgroundColor, &mut BorderColor), Without<KnappingGridButton>>,
    mut q_instruction: Query<&mut Text, (With<KnappingInstructionTextNode>, Without<KnappingProgressTextNode>, Without<KnappingButtonText>)>,
    mut q_buttons: Query<(&KnappingGridButton, &mut BackgroundColor, &mut BorderColor), Without<KnappingRecipeTabButton>>,
    mut q_btn_texts: Query<(&KnappingButtonText, &mut Text), (Without<KnappingProgressTextNode>, Without<KnappingInstructionTextNode>)>,
    mut q_progress_text: Query<&mut Text, (With<KnappingProgressTextNode>, Without<KnappingInstructionTextNode>, Without<KnappingButtonText>)>,
) {
    if !state.is_open {
        return;
    }

    let pat = state.active_recipe.pattern();
    let waste_count = state.active_recipe.waste_count();

    // 1. Update Tab Button Styles
    for (tab_btn, mut bg, mut border) in q_tabs.iter_mut() {
        let is_active = tab_btn.recipe == state.active_recipe;
        if is_active {
            bg.0 = Color::srgb(0.25, 0.45, 0.65);
            border.0 = Color::srgb(0.45, 0.75, 0.95);
        } else {
            bg.0 = Color::srgb(0.18, 0.20, 0.24);
            border.0 = Color::srgb(0.32, 0.35, 0.40);
        }
    }

    // 2. Update Instruction Text
    if let Ok(mut inst_text) = q_instruction.get_single_mut() {
        inst_text.sections[0].value = format!("Click marked flakes (x) to shape a {}!", state.active_recipe.display_name());
    }

    // 3. Calculate Chipping Progress
    let chipped_correct = (0..5).flat_map(|r| (0..5).map(move |c| (r, c)))
        .filter(|&(r, c)| !pat[r][c] && !state.grid[r][c])
        .count();

    let chipped_ruined = (0..5).flat_map(|r| (0..5).map(move |c| (r, c)))
        .any(|(r, c)| pat[r][c] && !state.grid[r][c]);

    if let Ok(mut prog_text) = q_progress_text.get_single_mut() {
        if chipped_ruined {
            prog_text.sections[0].value = "Ruined! Core blade chipped! Press Esc or select tab to reset".to_string();
            prog_text.sections[0].style.color = Color::srgb(0.95, 0.35, 0.30);
        } else if chipped_correct == waste_count {
            prog_text.sections[0].value = format!("Pattern Complete! {} formed!", state.active_recipe.display_name());
            prog_text.sections[0].style.color = Color::srgb(0.95, 0.85, 0.35);
        } else {
            prog_text.sections[0].value = format!("Progress: {} / {} Flakes Chipped", chipped_correct, waste_count);
            prog_text.sections[0].style.color = Color::srgb(0.45, 0.85, 0.95);
        }
    }

    // 4. Update Grid Buttons (Ghost Guide)
    for (btn, mut bg, mut border) in q_buttons.iter_mut() {
        let (r, c) = (btn.row, btn.col);
        let is_blade = pat[r][c];
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

    // 5. Update Grid Button 'x' Marks
    for (node, mut text) in q_btn_texts.iter_mut() {
        let (r, c) = (node.row, node.col);
        let is_blade = pat[r][c];
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
    fn test_knapping_all_recipes_pattern_matching() {
        for recipe in [
            KnappingRecipe::AxeHead,
            KnappingRecipe::PickaxeHead,
            KnappingRecipe::ShovelHead,
            KnappingRecipe::KnifeBlade,
        ] {
            let mut state = KnappingState::default();
            state.active_recipe = recipe;
            assert!(!state.matches_active_recipe(), "Initial full stone slab should not match {:?}", recipe);

            let pat = recipe.pattern();
            for r in 0..5 {
                for c in 0..5 {
                    if !pat[r][c] {
                        state.grid[r][c] = false;
                    }
                }
            }

            assert!(state.matches_active_recipe(), "Chipped stone should match pattern for {:?}", recipe);

            // Excessive chip in pattern body should fail
            let mut found_core = false;
            for r in 0..5 {
                for c in 0..5 {
                    if pat[r][c] {
                        state.grid[r][c] = false;
                        found_core = true;
                        break;
                    }
                }
                if found_core {
                    break;
                }
            }
            assert!(!state.matches_active_recipe(), "Excessive chipping should invalidate recipe {:?}", recipe);

            // Reset restores all cells
            state.reset_grid();
            assert_eq!(state.grid, [[true; 5]; 5]);
        }
    }

    #[test]
    fn test_all_flint_tools_and_heads_registry() {
        let registry = ItemRegistry;

        let tool_heads = [
            (ItemType::FlintAxeHead, "Flint Axe Head", 16),
            (ItemType::FlintPickaxeHead, "Flint Pickaxe Head", 16),
            (ItemType::FlintShovelHead, "Flint Shovel Head", 16),
            (ItemType::FlintKnifeBlade, "Flint Knife Blade", 16),
        ];

        for (item_type, name, max_stack) in tool_heads {
            let def = registry.get(item_type).unwrap_or_else(|| panic!("{:?} must be registered", item_type));
            assert_eq!(def.name, name);
            assert_eq!(def.kind, ItemKind::Material);
            assert_eq!(def.max_stack, max_stack);
        }

        let tools = [
            (ItemType::FlintAxe, "Flint Axe", ToolType::Axe, ToolTier::Wood, 2.0, 45),
            (ItemType::FlintPickaxe, "Flint Pickaxe", ToolType::Pickaxe, ToolTier::Wood, 2.0, 40),
            (ItemType::FlintShovel, "Flint Shovel", ToolType::Shovel, ToolTier::Wood, 2.5, 45),
            (ItemType::FlintKnife, "Flint Knife", ToolType::Sword, ToolTier::Wood, 2.0, 55),
        ];

        for (item_type, name, expected_tool, expected_tier, expected_eff, expected_dur) in tools {
            let def = registry.get(item_type).unwrap_or_else(|| panic!("{:?} must be registered", item_type));
            assert_eq!(def.name, name);
            assert_eq!(def.max_stack, 1);
            if let ItemKind::Tool { tool_type, tier, efficiency, max_durability } = def.kind {
                assert_eq!(tool_type, expected_tool);
                assert_eq!(tier, expected_tier);
                assert!((efficiency - expected_eff).abs() < 1e-4);
                assert_eq!(max_durability, expected_dur);
            } else {
                panic!("{:?} must be registered as Tool!", item_type);
            }
        }
    }

    #[test]
    fn test_flint_tools_harvest_capabilities() {
        let registry = ItemRegistry;
        let axe_def = registry.get(ItemType::FlintAxe).expect("FlintAxe must exist");
        let pick_def = registry.get(ItemType::FlintPickaxe).expect("FlintPickaxe must exist");
        let shovel_def = registry.get(ItemType::FlintShovel).expect("FlintShovel must exist");

        // OakLog harvest with FlintAxe
        let oak_log = BlockType::OakLog;
        if let ItemKind::Tool { tool_type, tier, .. } = axe_def.kind {
            assert_eq!(tool_type, oak_log.preferred_tool());
            assert!(tier >= oak_log.required_tier());
        }

        // Stone harvest with FlintPickaxe
        let stone = BlockType::Stone;
        if let ItemKind::Tool { tool_type, tier, .. } = pick_def.kind {
            assert_eq!(tool_type, stone.preferred_tool());
            assert!(tier >= stone.required_tier());
        }

        // CoalOre harvest with FlintPickaxe
        let coal = BlockType::CoalOre;
        if let ItemKind::Tool { tool_type, tier, .. } = pick_def.kind {
            assert_eq!(tool_type, coal.preferred_tool());
            assert!(tier >= coal.required_tier());
        }

        // Dirt / Grass with FlintShovel
        let dirt = BlockType::Dirt;
        if let ItemKind::Tool { tool_type, .. } = shovel_def.kind {
            assert_eq!(tool_type, dirt.preferred_tool());
        }
    }
}
