use bevy::prelude::Vec3;
use super::math::Aabb;

pub fn swept_aabb(
    player_aabb: &Aabb,
    velocity: Vec3,
    block_aabb: &Aabb,
) -> (f32, Vec3) {
    let mut inv_entry = Vec3::ZERO;
    let mut inv_exit = Vec3::ZERO;

    for i in 0..3 {
        if velocity[i] > 0.0 {
            inv_entry[i] = block_aabb.min[i] - player_aabb.max[i];
            inv_exit[i] = block_aabb.max[i] - player_aabb.min[i];
        } else {
            inv_entry[i] = block_aabb.max[i] - player_aabb.min[i];
            inv_exit[i] = block_aabb.min[i] - player_aabb.max[i];
        }
    }

    let mut entry = Vec3::ZERO;
    let mut exit = Vec3::ZERO;

    for i in 0..3 {
        if velocity[i] == 0.0 {
            if player_aabb.max[i] <= block_aabb.min[i] || player_aabb.min[i] >= block_aabb.max[i] {
                // No overlap on this axis and not moving on it -> impossible to collide
                entry[i] = f32::INFINITY;
                exit[i] = -f32::INFINITY;
            } else {
                // Overlapping on this axis and not moving -> overlaps forever
                entry[i] = -f32::INFINITY;
                exit[i] = f32::INFINITY;
            }
        } else {
            entry[i] = inv_entry[i] / velocity[i];
            exit[i] = inv_exit[i] / velocity[i];
        }
    }

    let entry_time = entry.x.max(entry.y).max(entry.z);
    let exit_time = exit.x.min(exit.y).min(exit.z);

    if entry_time > exit_time || entry_time > 1.0 || exit_time < 0.0 {
        return (1.0, Vec3::ZERO);
    }

    let mut normal = Vec3::ZERO;
    if entry.x > entry.y && entry.x > entry.z {
        if inv_entry.x < 0.0 {
            normal.x = 1.0;
        } else {
            normal.x = -1.0;
        }
    } else if entry.y > entry.x && entry.y > entry.z {
        if inv_entry.y < 0.0 {
            normal.y = 1.0;
        } else {
            normal.y = -1.0;
        }
    } else {
        if inv_entry.z < 0.0 {
            normal.z = 1.0;
        } else {
            normal.z = -1.0;
        }
    }

    (entry_time, normal)
}
