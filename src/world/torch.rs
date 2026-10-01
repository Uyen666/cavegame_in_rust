use bevy::prelude::*;
use bevy::utils::HashMap;
use crate::world::{WorldManager, BlockType};
use crate::render::particles::ParticleManager;

/// 放置火把預設燃燒時間 (180 秒 = 3 分鐘)
pub const DEFAULT_TORCH_LIFETIME: f32 = 180.0;

/// 火把燃燒管理系統 (Torch Burnout & Lifetime Manager)
/// 追蹤所有放置在世界中的火把剩餘壽命。當燃料耗盡時熄滅並消除光源，產生煙塵粒子。
#[derive(Resource, Debug, Clone, Default)]
pub struct TorchBurnManager {
    /// 座標與剩餘燃燒壽命 (秒)
    pub active_torches: HashMap<IVec3, f32>,
}

impl TorchBurnManager {
    pub fn register(&mut self, pos: IVec3, burn_seconds: f32) {
        self.active_torches.insert(pos, burn_seconds);
    }

    pub fn remove(&mut self, pos: IVec3) {
        self.active_torches.remove(&pos);
    }

    #[allow(dead_code)]
    pub fn get_remaining(&self, pos: IVec3) -> Option<f32> {
        self.active_torches.get(&pos).copied()
    }

    #[allow(dead_code)]
    pub fn count(&self) -> usize {
        self.active_torches.len()
    }
}

/// 火把心跳計時系統：每影格遞減燃燒時間，燃盡時自動熄滅並拔除光源
pub fn torch_burn_tick_system(
    time: Res<Time>,
    mut torch_mgr: ResMut<TorchBurnManager>,
    mut world: ResMut<WorldManager>,
    mut commands: Commands,
    mut particle_mgr: ResMut<ParticleManager>,
) {
    let dt = time.delta_seconds();
    if dt < 0.0001 { return; }

    let mut expired = Vec::new();

    torch_mgr.active_torches.retain(|&pos, timer| {
        // 若該位置已不再是火把 (例如已被手動敲除或被水流沖垮)，從追蹤清單移除
        let current_block = world.get_block_global(pos);
        if !current_block.is_torch() {
            return false;
        }

        *timer -= dt;
        if *timer <= 0.0 {
            expired.push(pos);
            false
        } else {
            true
        }
    });

    for pos in expired {
        // 熄滅火把：轉為空氣，自動觸發 Phase 5a 光源消除
        world.set_block_global(pos, BlockType::Air, &mut commands);
        crate::world::fluid::wake_up_fluids_in_radius(&mut world, pos);

        // 噴發 8 顆灰色煙塵粒子 (代表熄滅煙霧)
        particle_mgr.spawn_debris(pos.as_vec3() + 0.5, Color::srgb_u8(75, 75, 75), 8);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::world::{World as BevyWorld, CommandQueue};
    use crate::world::{ChunkEntry, ChunkLightBuffer, generator};
    use crate::render::particles::ParticleManager;

    #[test]
    fn test_torch_burnout_lifecycle() {
        let mut world_manager = WorldManager::default();
        let ecs_world = BevyWorld::new();
        let mut queue = CommandQueue::default();
        let mut commands = Commands::new(&mut queue, &ecs_world);
        let mut particle_mgr = ParticleManager::default();
        let mut torch_mgr = TorchBurnManager::default();

        let chunk_pos = IVec3::new(0, 0, 0);
        world_manager.chunks.insert(chunk_pos, ChunkEntry {
            buffer: std::sync::Arc::new(generator::ChunkBuffer { blocks: [BlockType::Air; 32768] }),
            light_buffer: std::sync::Arc::new(ChunkLightBuffer::default()),
            fluid_buffer: None,
            entity: None,
            is_modified: false,
            is_lighting_ready: true,
        });

        let torch_pos = IVec3::new(4, 4, 4);
        world_manager.set_block_global(torch_pos, BlockType::Torch, &mut commands);
        assert_eq!(world_manager.get_block_light_global(torch_pos), 15);

        // 註冊火把剩餘 1.0 秒
        torch_mgr.register(torch_pos, 1.0);
        assert_eq!(torch_mgr.count(), 1);
        assert_eq!(torch_mgr.get_remaining(torch_pos), Some(1.0));

        // 模擬經過 1.5 秒
        let mut expired = Vec::new();
        torch_mgr.active_torches.retain(|&pos, timer| {
            *timer -= 1.5;
            if *timer <= 0.0 {
                expired.push(pos);
                false
            } else {
                true
            }
        });

        assert_eq!(torch_mgr.count(), 0);
        assert_eq!(expired.len(), 1);

        for pos in expired {
            world_manager.set_block_global(pos, BlockType::Air, &mut commands);
            particle_mgr.spawn_debris(pos.as_vec3() + 0.5, Color::srgb_u8(75, 75, 75), 8);
        }

        // 驗證火把已被拔除且光源徹底歸零
        assert_eq!(world_manager.get_block_global(torch_pos), BlockType::Air);
        assert_eq!(world_manager.get_block_light_global(torch_pos), 0, "火把熄滅後光源必須徹底消除");
    }
}
