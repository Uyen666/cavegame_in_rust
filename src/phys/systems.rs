use bevy::prelude::*;
use crate::world::WorldManager;
use crate::config::EngineConfig;
use crate::utils::math::Aabb;
use super::components::*;

pub fn update_fluid_sensors(
    mut query: Query<(&Transform, &AabbCollider, &mut FluidSensor)>,
    world: Res<WorldManager>,
) {
    for (transform, collider, mut fluid_sensor) in query.iter_mut() {
        let pos = transform.translation;
        let p_aabb = collider.get_global_aabb(pos);
        
        let min_x = p_aabb.min.x.floor() as i32;
        let max_x = p_aabb.max.x.ceil() as i32;
        let min_y = p_aabb.min.y.floor() as i32;
        let max_y = p_aabb.max.y.ceil() as i32;
        let min_z = p_aabb.min.z.floor() as i32;
        let max_z = p_aabb.max.z.ceil() as i32;

        let mut in_fluid = false;
        
        for x in min_x..max_x {
            for y in min_y..max_y {
                for z in min_z..max_z {
                    let b_pos = IVec3::new(x, y, z);
                    if world.get_fluid_global(b_pos) > 0 {
                        in_fluid = true;
                        break;
                    }
                }
                if in_fluid { break; }
            }
            if in_fluid { break; }
        }

        let head_y = pos.y + collider.max_offset.y;
        let head_pos = IVec3::new(pos.x.floor() as i32, head_y.floor() as i32, pos.z.floor() as i32);
        let head_in_fluid = world.get_fluid_global(head_pos) > 0;

        fluid_sensor.in_fluid = in_fluid;
        fluid_sensor.head_in_fluid = head_in_fluid;
    }
}

pub fn apply_kinematics(
    time: Res<Time>,
    config: Res<EngineConfig>,
    mut query: Query<(&mut Velocity, &RigidBody, &FluidSensor, &GroundSensor)>,
) {
    let dt = time.delta_seconds();
    if dt < 0.0001 { return; }

    for (mut vel, rb, fluid_sensor, ground_sensor) in query.iter_mut() {
        if rb.is_kinematic {
            continue; // Bypass physics for kinematics / spectator
        }

        if fluid_sensor.in_fluid {
            // 在水中，扣除恆定的水中微弱重力（下沉）
            vel.y -= config.physics.gravity * config.physics.water_gravity_multiplier * rb.gravity_scale * dt;
            
            // 套用高額的水體阻尼（固定乘法）
            let damp = (1.0 - config.physics.water_damping * dt).max(0.0);
            vel.x *= damp;
            vel.z *= damp;
            
            // 如果不是在水面上且沒有按跳躍(這邊水面脫離的邏輯已經移交給Player意圖，所以基礎物理預設給予阻尼)
            // Note: 為了保持架構乾淨，若 Player 需要特殊水面衝量，應由 Player 輸入系統覆蓋
            vel.y *= damp;
        } else {
            // Dry land gravity
            if !ground_sensor.on_ground {
                vel.y -= config.physics.gravity * rb.gravity_scale * dt;
            }
        }
    }
}

pub fn resolve_collisions(
    time: Res<Time>,
    world: Res<WorldManager>,
    mut query: Query<(
        &mut Transform,
        &mut Velocity,
        &AabbCollider,
        &mut RigidBody,
        &mut GroundSensor,
    )>,
) {
    let dt = time.delta_seconds();
    if dt < 0.0001 { return; }

    for (mut transform, mut vel, collider, mut rb, mut ground) in query.iter_mut() {
        if rb.is_kinematic {
            continue;
        }

        let mut pos = transform.translation;
        const EPSILON: f32 = 0.001;

        let get_intersecting_blocks = |p: Vec3, col: &AabbCollider, w: &WorldManager| -> Vec<Aabb> {
            let p_aabb = col.get_global_aabb(p);
            let min_x = p_aabb.min.x.floor() as i32;
            let max_x = p_aabb.max.x.ceil() as i32;
            let min_y = p_aabb.min.y.floor() as i32;
            let max_y = p_aabb.max.y.ceil() as i32;
            let min_z = p_aabb.min.z.floor() as i32;
            let max_z = p_aabb.max.z.ceil() as i32;

            let mut hits = Vec::new();
            for x in min_x..max_x {
                for y in min_y..max_y {
                    for z in min_z..max_z {
                        let b_pos = IVec3::new(x, y, z);
                        if w.get_block_global(b_pos).is_solid() {
                            let b_aabb = Aabb::new(
                                Vec3::new(x as f32, y as f32, z as f32),
                                Vec3::new(x as f32 + 1.0, y as f32 + 1.0, z as f32 + 1.0),
                            );
                            if p_aabb.intersects(&b_aabb) {
                                hits.push(b_aabb);
                            }
                        }
                    }
                }
            }
            hits
        };

        rb.is_colliding_horizontally = false;
        ground.was_on_ground = ground.on_ground;

        // X axis
        if vel.x != 0.0 {
            pos.x += vel.x * dt;
            let hits = get_intersecting_blocks(pos, collider, &world);
            if !hits.is_empty() {
                rb.is_colliding_horizontally = true;
                if vel.x > 0.0 {
                    let wall_x = hits.iter().map(|b| b.min.x).fold(f32::INFINITY, f32::min);
                    pos.x = wall_x - collider.max_offset.x - EPSILON;
                } else {
                    let wall_x = hits.iter().map(|b| b.max.x).fold(f32::NEG_INFINITY, f32::max);
                    pos.x = wall_x - collider.min_offset.x + EPSILON;
                }
                vel.x = 0.0;
            } else if rb.safewalk && ground.was_on_ground {
                let mut fall_test = pos;
                fall_test.y -= 0.05;
                if get_intersecting_blocks(fall_test, collider, &world).is_empty() {
                    pos.x -= vel.x * dt;
                    vel.x = 0.0;
                }
            }
        }
        
        // Z axis (同步使用更新後的 X pos)
        if vel.z != 0.0 {
            pos.z += vel.z * dt;
            let hits = get_intersecting_blocks(pos, collider, &world);
            if !hits.is_empty() {
                rb.is_colliding_horizontally = true;
                if vel.z > 0.0 {
                    let wall_z = hits.iter().map(|b| b.min.z).fold(f32::INFINITY, f32::min);
                    pos.z = wall_z - collider.max_offset.z - EPSILON;
                } else {
                    let wall_z = hits.iter().map(|b| b.max.z).fold(f32::NEG_INFINITY, f32::max);
                    pos.z = wall_z - collider.min_offset.z + EPSILON;
                }
                vel.z = 0.0;
            } else if rb.safewalk && ground.was_on_ground {
                let mut fall_test = pos;
                fall_test.y -= 0.05;
                if get_intersecting_blocks(fall_test, collider, &world).is_empty() {
                    pos.z -= vel.z * dt;
                    vel.z = 0.0;
                }
            }
        }

        // Y axis (同步使用更新後的 X, Z pos)
        ground.on_ground = false;
        if vel.y != 0.0 {
            pos.y += vel.y * dt;
            let hits = get_intersecting_blocks(pos, collider, &world);
            if !hits.is_empty() {
                if vel.y > 0.0 {
                    let ceil_y = hits.iter().map(|b| b.min.y).fold(f32::INFINITY, f32::min);
                    pos.y = ceil_y - collider.max_offset.y - EPSILON;
                } else {
                    let ground_y = hits.iter().map(|b| b.max.y).fold(f32::NEG_INFINITY, f32::max);
                    pos.y = ground_y - collider.min_offset.y; // Precise grounding, min_offset is typically 0 for Y
                    ground.on_ground = true;
                }
                vel.y = 0.0;
            }
        }

        // 寫回最終安全座標
        transform.translation = pos;
    }
}
