use bevy::prelude::Vec3;
use crate::utils::math::Aabb;

#[allow(dead_code)]
pub fn swept_aabb(
    entity_aabb: &Aabb,
    velocity: Vec3,
    block_aabb: &Aabb,
) -> (f32, Vec3) {
    // 引入 Minecraft 原版的標準安全緩衝
    const MC_EPSILON: f32 = 1.0E-7;

    let mut inv_entry = Vec3::ZERO;
    let mut inv_exit = Vec3::ZERO;

    for i in 0..3 {
        if velocity[i] > 0.0 {
            inv_entry[i] = block_aabb.min[i] - entity_aabb.max[i];
            inv_exit[i] = block_aabb.max[i] - entity_aabb.min[i];
        } else {
            inv_entry[i] = block_aabb.max[i] - entity_aabb.min[i];
            inv_exit[i] = block_aabb.min[i] - entity_aabb.max[i];
        }
    }

    let mut entry = Vec3::ZERO;
    let mut exit = Vec3::ZERO;

    for i in 0..3 {
        // 修正：浮點數判定歸零時，使用小於微小值判定，比 == 0.0 更安全
        if velocity[i].abs() < MC_EPSILON {
            if entity_aabb.max[i] <= block_aabb.min[i] || entity_aabb.min[i] >= block_aabb.max[i] {
                entry[i] = f32::INFINITY;
                exit[i] = f32::NEG_INFINITY;
            } else {
                entry[i] = f32::NEG_INFINITY;
                exit[i] = f32::INFINITY;
            }
        } else {
            entry[i] = inv_entry[i] / velocity[i];
            exit[i] = inv_exit[i] / velocity[i];
        }
    }

    // 找出最晚進入與最早離開的時間
    let entry_time = entry.x.max(entry.y).max(entry.z);
    let exit_time = exit.x.min(exit.y).min(exit.z);

    // 🌟 核心防禦 A：判定是否根本沒有發生碰撞
    if entry_time > exit_time || entry_time > 1.0 || exit_time < 0.0 {
        return (1.0, Vec3::ZERO);
    }

    // 🌟 核心防禦 B：修正多軸時間相等的平手（Tie-breaking）與法線判定
    let mut normal = Vec3::ZERO;
    
    // 引入一個極小的誤差容忍，避免完全相等時的判定崩潰
    if entry.x > entry.y - MC_EPSILON && entry.x > entry.z - MC_EPSILON {
        normal.x = if inv_entry.x < 0.0 { 1.0 } else { -1.0 };
    } else if entry.y > entry.x - MC_EPSILON && entry.y > entry.z - MC_EPSILON {
        normal.y = if inv_entry.y < 0.0 { 1.0 } else { -1.0 };
    } else {
        normal.z = if inv_entry.z < 0.0 { 1.0 } else { -1.0 };
    }

    // 🌟 核心防禦 C：如果起點就稍微重疊 (entry_time < 0)，
    // 強制將時間截斷為 0.0，防止滑動系統往後倒退產生劇烈抖動！
    let final_entry_time = entry_time.max(0.0);

    (final_entry_time, normal)
}
