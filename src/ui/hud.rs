use bevy::prelude::*;
use crate::GameState;
use crate::player::PlayerCamera;
use crate::world::{WorldManager, BlockType, Chunk};

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
    q_chunks: Query<(Entity, &Chunk)>,
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

            let block = world.get_block_global(block_pos, &q_chunks);
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
