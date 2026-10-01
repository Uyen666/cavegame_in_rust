use bevy::prelude::*;
use crate::phys::components::FluidSensor;
use crate::world::WorldManager;
use crate::world::DayNightCycle;
use crate::item::{Inventory, ItemType};

/// 玩家生理狀態組件 (Hardcore Physiology Vitals)
#[derive(Component, Debug, Clone, Reflect)]
pub struct PlayerVitals {
    pub health: f32,             // 0.0 ~ 100.0 (生命值)
    pub max_health: f32,         // 100.0
    pub stamina: f32,            // 0.0 ~ 100.0 (體力耐力值)
    pub max_stamina: f32,        // 100.0
    pub thirst: f32,             // 0.0 ~ 100.0 (口渴含水量)
    pub max_thirst: f32,         // 100.0
    pub body_temperature: f32,  // 攝氏體溫，正常 37.0°C
    pub wetness: f32,            // 0.0 ~ 1.0 (潮濕度)
}

impl Default for PlayerVitals {
    fn default() -> Self {
        Self {
            health: 100.0,
            max_health: 100.0,
            stamina: 100.0,
            max_stamina: 100.0,
            thirst: 100.0,
            max_thirst: 100.0,
            body_temperature: 37.0,
            wetness: 0.0,
        }
    }
}

/// 計算背包當前負重比率 (0.0 ~ 1.0+)
pub fn calculate_inventory_encumbrance(inventory: &Inventory) -> f32 {
    let mut heavy_count = 0u32;
    for slot in inventory.slots.iter().flatten() {
        match slot.item_type {
            ItemType::Stone | ItemType::IronOre | ItemType::IronIngot | ItemType::Gravel | ItemType::CoalOre | ItemType::OakLog => {
                heavy_count += slot.count as u32;
            }
            _ => {}
        }
    }
    // 預設攜帶超過 384 個重物 (6 組 64) 即達 100% 滿負重
    (heavy_count as f32 / 384.0).min(1.5)
}

/// 玩家生理狀態核心心跳結算系統
pub fn update_player_vitals_system(
    time: Res<Time>,
    world: Res<WorldManager>,
    cycle: Res<DayNightCycle>,
    keys: Res<ButtonInput<KeyCode>>,
    mut query: Query<(&Transform, &FluidSensor, &Inventory, &mut PlayerVitals)>,
) {
    let dt = time.delta_seconds();
    if dt < 0.0001 { return; }

    for (transform, fluid, inventory, mut vitals) in query.iter_mut() {
        let is_sprinting = keys.pressed(KeyCode::ShiftLeft);
        let is_moving = keys.pressed(KeyCode::KeyW) || keys.pressed(KeyCode::KeyA) || keys.pressed(KeyCode::KeyS) || keys.pressed(KeyCode::KeyD);
        let encumbrance = calculate_inventory_encumbrance(inventory);

        // ── 1. 體力消耗與恢復 ──
        if is_sprinting && is_moving {
            let base_drain = if fluid.in_fluid { 18.0 } else { 12.0 };
            // 負重增加體力消耗 (最高 +50%)
            let drain_rate = base_drain * (1.0 + encumbrance * 0.5);
            vitals.stamina = (vitals.stamina - drain_rate * dt).max(0.0);
        } else {
            // 走動或站立時平滑恢復
            let recovery_rate = 15.0 / (1.0 + encumbrance * 0.3);
            vitals.stamina = (vitals.stamina + recovery_rate * dt).min(vitals.max_stamina);
        }

        // ── 2. 潮濕度與火把烘烤 ──
        if fluid.in_fluid {
            vitals.wetness = (vitals.wetness + 0.6 * dt).min(1.0);
        }

        let player_eye = IVec3::new(
            transform.translation.x.floor() as i32,
            (transform.translation.y + 1.5).floor() as i32,
            transform.translation.z.floor() as i32,
        );
        let block_light = world.get_block_light_global(player_eye);

        // 靠近溫暖火把 (方塊光 >= 9): 急速烘乾蒸發
        if block_light >= 9 {
            vitals.wetness = (vitals.wetness - 0.25 * dt).max(0.0);
            if vitals.body_temperature < 37.0 {
                vitals.body_temperature = (vitals.body_temperature + 0.5 * dt).min(37.0);
            }
        } else if !fluid.in_fluid {
            // 空氣中自然風乾
            vitals.wetness = (vitals.wetness - 0.02 * dt).max(0.0);
        }

        // ── 3. 體溫結算 (失溫 vs 回暖) ──
        if vitals.wetness > 0.25 {
            // 潮濕時熱量迅速流失，夜晚失溫加劇
            let chill_factor = 0.08 + vitals.wetness * 0.16 + (1.0 - cycle.sky_factor) * 0.08;
            vitals.body_temperature = (vitals.body_temperature - chill_factor * dt).max(30.0);
        } else if block_light < 9 && vitals.body_temperature < 37.0 {
            // 乾燥且未烘烤時緩慢體溫自動調節
            vitals.body_temperature = (vitals.body_temperature + 0.05 * dt).min(37.0);
        }

        // ── 4. 口渴度消耗 ──
        let thirst_rate = if is_sprinting { 0.18 } else { 0.05 };
        vitals.thirst = (vitals.thirst - thirst_rate * dt).max(0.0);

        // ── 5. 健康傷害與極限懲罰 ──
        // (a) 失溫凍傷 (體溫低於 35.0°C)
        if vitals.body_temperature < 35.0 {
            let frost_damage = (35.0 - vitals.body_temperature) * 0.8;
            vitals.health = (vitals.health - frost_damage * dt).max(0.0);
        }

        // (b) 嚴重脫水 (口渴度為 0)
        if vitals.thirst <= 0.0 {
            vitals.health = (vitals.health - 2.0 * dt).max(0.0);
        }

        // (c) 飽腹溫暖時自然緩慢治癒
        if vitals.body_temperature >= 36.5 && vitals.thirst >= 75.0 && vitals.stamina >= 70.0 && vitals.health < vitals.max_health {
            vitals.health = (vitals.health + 0.6 * dt).min(vitals.max_health);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vitals_initial_values() {
        let vitals = PlayerVitals::default();
        assert_eq!(vitals.health, 100.0);
        assert_eq!(vitals.stamina, 100.0);
        assert_eq!(vitals.thirst, 100.0);
        assert_eq!(vitals.body_temperature, 37.0);
        assert_eq!(vitals.wetness, 0.0);
    }

    #[test]
    fn test_encumbrance_calculation() {
        let mut inv = Inventory::new(36);
        let registry = crate::item::ItemRegistry;
        assert_eq!(calculate_inventory_encumbrance(&inv), 0.0);

        // 放滿 6 組 64 個石頭 = 384 個重物
        for i in 0..6 {
            inv.set_slot(i, Some(crate::item::ItemStack::new(ItemType::Stone, 64, &registry)));
        }
        let enc = calculate_inventory_encumbrance(&inv);
        assert!((enc - 1.0).abs() < 0.01, "384 個石頭應對應 100% 滿負重");
    }
}
