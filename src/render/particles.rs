use bevy::prelude::*;
use crate::world::{WorldManager, BlockType};
use crate::GameState;

#[derive(Clone, Copy, Debug)]
pub struct Particle {
    pub pos: Vec3,
    pub vel: Vec3,
    pub color: Color,
    pub size: f32,
    pub lifetime: f32,
    pub max_lifetime: f32,
}

#[derive(Resource)]
pub struct ParticleManager {
    pub particles: Vec<Particle>,
    pub rng_seed: u32,
}

impl Default for ParticleManager {
    fn default() -> Self {
        Self {
            particles: Vec::with_capacity(256),
            rng_seed: 13371337,
        }
    }
}

impl ParticleManager {
    fn next_f32(&mut self) -> f32 {
        self.rng_seed = self.rng_seed.wrapping_mul(1664525).wrapping_add(1013904223);
        (self.rng_seed as f32) / (u32::MAX as f32)
    }

    fn range_f32(&mut self, min: f32, max: f32) -> f32 {
        min + self.next_f32() * (max - min)
    }

    /// 產生方塊採掘/破壞碎屑粒子
    pub fn spawn_debris(&mut self, center: Vec3, color: Color, count: usize) {
        for _ in 0..count {
            let offset = Vec3::new(
                self.range_f32(-0.35, 0.35),
                self.range_f32(-0.35, 0.35),
                self.range_f32(-0.35, 0.35),
            );
            let vel = Vec3::new(
                self.range_f32(-2.5, 2.5),
                self.range_f32(1.5, 4.5),
                self.range_f32(-2.5, 2.5),
            );
            let max_lifetime = self.range_f32(0.35, 0.65);
            let size = self.range_f32(0.06, 0.12);

            self.particles.push(Particle {
                pos: center + offset,
                vel,
                color,
                size,
                lifetime: 0.0,
                max_lifetime,
            });
        }
    }
}

/// 依據方塊材質解析碎屑代表色
pub fn get_block_debris_color(block: BlockType) -> Color {
    match block {
        BlockType::Grass => Color::srgb(0.35, 0.65, 0.25),
        BlockType::Stone => Color::srgb(0.55, 0.55, 0.55),
        BlockType::Dirt => Color::srgb(0.52, 0.37, 0.26),
        BlockType::OakLog => Color::srgb(0.42, 0.31, 0.18),
        BlockType::OakLeaves => Color::srgb(0.22, 0.48, 0.16),
        BlockType::Sand => Color::srgb(0.86, 0.82, 0.60),
        BlockType::Gravel => Color::srgb(0.60, 0.58, 0.58),
        BlockType::CoalOre => Color::srgb(0.22, 0.22, 0.22),
        BlockType::IronOre => Color::srgb(0.68, 0.55, 0.48),
        BlockType::Glass => Color::srgba(0.85, 0.95, 1.0, 0.7),
        BlockType::Torch
        | BlockType::TorchWallN
        | BlockType::TorchWallS
        | BlockType::TorchWallE
        | BlockType::TorchWallW => Color::srgb(1.0, 0.75, 0.15),
        _ => Color::srgb(0.5, 0.5, 0.5),
    }
}

pub fn update_and_render_particles(
    time: Res<Time>,
    mut particle_mgr: ResMut<ParticleManager>,
    mut gizmos: Gizmos,
    world: Res<WorldManager>,
) {
    let dt = time.delta_seconds().min(0.05);

    particle_mgr.particles.retain_mut(|p| {
        p.lifetime += dt;
        if p.lifetime >= p.max_lifetime {
            return false;
        }

        // 重力加速度
        p.vel.y -= 15.0 * dt;
        let next_pos = p.pos + p.vel * dt;

        // 方塊碰撞檢測 (簡易離散碰撞)
        let block_pos = IVec3::new(
            next_pos.x.floor() as i32,
            next_pos.y.floor() as i32,
            next_pos.z.floor() as i32,
        );

        if world.get_block_global(block_pos).is_solid() {
            p.vel.x *= 0.5;
            p.vel.z *= 0.5;
            p.vel.y = -p.vel.y * 0.25; // 輕微彈性地面回彈
            p.pos.y = block_pos.y as f32 + 1.0 + p.size;
        } else {
            p.pos = next_pos;
        }

        let life_factor = (1.0 - (p.lifetime / p.max_lifetime)).max(0.0);
        let current_size = p.size * life_factor;

        gizmos.cuboid(
            Transform::from_translation(p.pos).with_scale(Vec3::splat(current_size)),
            p.color,
        );

        true
    });
}

pub struct ParticlesPlugin;

impl Plugin for ParticlesPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ParticleManager>()
            .add_systems(
                Update,
                update_and_render_particles.run_if(in_state(GameState::InGame)),
            );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_debris_particles_spawn_and_lifecycle() {
        let mut mgr = ParticleManager::default();
        assert_eq!(mgr.particles.len(), 0);

        let center = Vec3::new(10.0, 20.0, 30.0);
        let color = get_block_debris_color(BlockType::Stone);
        mgr.spawn_debris(center, color, 10);

        assert_eq!(mgr.particles.len(), 10);
        for p in &mgr.particles {
            assert_eq!(p.color, color);
            assert!(p.lifetime == 0.0);
            assert!(p.max_lifetime > 0.0);
            assert!((p.pos - center).length() < 1.0);
        }
    }
}
