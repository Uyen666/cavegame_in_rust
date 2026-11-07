#![allow(dead_code)]
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use crate::world::{BlockType, registry::{ToolTier, ToolType, DropTable}};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u16)]
pub enum ItemType {
    // Block Items
    Air = 0,
    Grass = 1,
    Stone = 2,
    Dirt = 3,
    OakLog = 4,
    OakLeaves = 5,
    Sand = 6,
    Gravel = 7,
    CoalOre = 8,
    IronOre = 9,
    Glass = 10,
    Torch = 11,

    // Materials / Drops
    Coal = 100,
    IronIngot = 101,
    Stick = 102,
    Flint = 103,
    FlintAxeHead = 104,

    // Tools
    WoodenPickaxe = 200,
    StonePickaxe = 201,
    IronPickaxe = 202,
    WoodenShovel = 203,
    StoneShovel = 204,
    IronShovel = 205,
    WoodenAxe = 206,
    StoneAxe = 207,
    IronAxe = 208,
    FlintAxe = 209,
}

impl ItemType {
    pub fn from_block(block: BlockType) -> Option<Self> {
        match block {
            BlockType::Air => None,
            BlockType::Grass => Some(ItemType::Grass),
            BlockType::Stone => Some(ItemType::Stone),
            BlockType::Dirt => Some(ItemType::Dirt),
            BlockType::OakLog => Some(ItemType::OakLog),
            BlockType::OakLeaves => Some(ItemType::OakLeaves),
            BlockType::Sand => Some(ItemType::Sand),
            BlockType::Gravel => Some(ItemType::Gravel),
            BlockType::CoalOre => Some(ItemType::CoalOre),
            BlockType::IronOre => Some(ItemType::IronOre),
            BlockType::Glass => Some(ItemType::Glass),
            BlockType::Torch | BlockType::TorchWallN | BlockType::TorchWallS | BlockType::TorchWallE | BlockType::TorchWallW => Some(ItemType::Torch),
        }
    }

    pub fn to_block(&self) -> Option<BlockType> {
        match self {
            ItemType::Grass => Some(BlockType::Grass),
            ItemType::Stone => Some(BlockType::Stone),
            ItemType::Dirt => Some(BlockType::Dirt),
            ItemType::OakLog => Some(BlockType::OakLog),
            ItemType::OakLeaves => Some(BlockType::OakLeaves),
            ItemType::Sand => Some(BlockType::Sand),
            ItemType::Gravel => Some(BlockType::Gravel),
            ItemType::CoalOre => Some(BlockType::CoalOre),
            ItemType::IronOre => Some(BlockType::IronOre),
            ItemType::Glass => Some(BlockType::Glass),
            ItemType::Torch => Some(BlockType::Torch),
            _ => None,
        }
    }
}

pub const DROP_ITEM_ID_COAL: u16 = 100;
pub const DROP_ITEM_ID_IRON_INGOT: u16 = 101;
pub const DROP_ITEM_ID_STICK: u16 = 102;

pub fn get_block_drop(block: BlockType) -> Option<ItemType> {
    match block.drop_table() {
        DropTable::None => None,
        DropTable::ItemDrop { item_id, .. } => match item_id {
            DROP_ITEM_ID_COAL => Some(ItemType::Coal),
            DROP_ITEM_ID_STICK => Some(ItemType::Stick),
            _ => None,
        },
        _ => match block {
            BlockType::Grass => Some(ItemType::Dirt),
            BlockType::CoalOre => Some(ItemType::Coal),
            BlockType::OakLeaves => Some(ItemType::Stick),
            BlockType::Gravel => Some(ItemType::Flint),
            other => ItemType::from_block(other),
        },
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ItemKind {
    Block(BlockType),
    Tool {
        tool_type: ToolType,
        tier: ToolTier,
        efficiency: f32,
        max_durability: u16,
    },
    Material,
}

/// 物品物理與掉落物表現屬性 (Item Physical Properties)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ItemPhysicsProperties {
    pub scale: Vec3,         // 3D 渲染比例
    pub half_height: f32,    // 著地休止半高
    pub flat_resting: bool,  // 著地時是否平躺 90 度橫臥 (如木棒、工具等)
    pub buoyancy: f32,       // 水中浮力加速度 (> 0 浮起，如木質 +18.0；<= 0 下沉，如礦物 -4.0)
    pub is_emissive: bool,   // 是否自體發光 (如火把，夜間不隨環境光變暗)
}

impl Default for ItemPhysicsProperties {
    fn default() -> Self {
        Self {
            scale: Vec3::splat(0.28),
            half_height: 0.14,
            flat_resting: false,
            buoyancy: -4.0,
            is_emissive: false,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ItemDefinition {
    pub id: ItemType,
    pub name: &'static str,
    pub kind: ItemKind,
    pub max_stack: u16,
    pub physics: ItemPhysicsProperties,
}

#[derive(Resource, Debug, Clone, Default)]
pub struct ItemRegistry;

impl ItemRegistry {
    pub fn get(&self, item: ItemType) -> Option<ItemDefinition> {
        let (name, kind, max_stack, physics) = match item {
            ItemType::Air => return None,
            ItemType::Grass => ("Grass Block", ItemKind::Block(BlockType::Grass), 64, ItemPhysicsProperties::default()),
            ItemType::Stone => ("Stone", ItemKind::Block(BlockType::Stone), 64, ItemPhysicsProperties::default()),
            ItemType::Dirt => ("Dirt", ItemKind::Block(BlockType::Dirt), 64, ItemPhysicsProperties::default()),
            ItemType::OakLog => ("Oak Log", ItemKind::Block(BlockType::OakLog), 64, ItemPhysicsProperties { buoyancy: 18.0, ..default() }),
            ItemType::OakLeaves => ("Oak Leaves", ItemKind::Block(BlockType::OakLeaves), 64, ItemPhysicsProperties { buoyancy: 18.0, ..default() }),
            ItemType::Sand => ("Sand", ItemKind::Block(BlockType::Sand), 64, ItemPhysicsProperties::default()),
            ItemType::Gravel => ("Gravel", ItemKind::Block(BlockType::Gravel), 64, ItemPhysicsProperties::default()),
            ItemType::CoalOre => ("Coal Ore", ItemKind::Block(BlockType::CoalOre), 64, ItemPhysicsProperties::default()),
            ItemType::IronOre => ("Iron Ore", ItemKind::Block(BlockType::IronOre), 64, ItemPhysicsProperties::default()),
            ItemType::Glass => ("Glass", ItemKind::Block(BlockType::Glass), 64, ItemPhysicsProperties::default()),
            ItemType::Torch => ("Torch", ItemKind::Block(BlockType::Torch), 64, ItemPhysicsProperties { is_emissive: true, ..default() }),

            ItemType::Coal => ("Coal", ItemKind::Material, 64, ItemPhysicsProperties { scale: Vec3::splat(0.2), half_height: 0.10, ..default() }),
            ItemType::IronIngot => ("Iron Ingot", ItemKind::Material, 64, ItemPhysicsProperties { scale: Vec3::new(0.24, 0.12, 0.16), half_height: 0.06, flat_resting: true, ..default() }),
            ItemType::Stick => ("Stick", ItemKind::Material, 64, ItemPhysicsProperties { scale: Vec3::new(0.08, 0.35, 0.08), half_height: 0.04, flat_resting: true, buoyancy: 18.0, is_emissive: false }),
            ItemType::Flint => ("Flint", ItemKind::Material, 64, ItemPhysicsProperties { scale: Vec3::new(0.22, 0.15, 0.22), half_height: 0.08, ..default() }),
            ItemType::FlintAxeHead => ("Flint Axe Head", ItemKind::Material, 16, ItemPhysicsProperties { scale: Vec3::new(0.24, 0.08, 0.24), half_height: 0.04, flat_resting: true, buoyancy: -4.0, is_emissive: false }),

            ItemType::WoodenPickaxe => ("Wooden Pickaxe", ItemKind::Tool { tool_type: ToolType::Pickaxe, tier: ToolTier::Wood, efficiency: 2.0, max_durability: 59 }, 1, ItemPhysicsProperties { scale: Vec3::new(0.12, 0.35, 0.12), half_height: 0.06, flat_resting: true, buoyancy: 18.0, ..default() }),
            ItemType::StonePickaxe => ("Stone Pickaxe", ItemKind::Tool { tool_type: ToolType::Pickaxe, tier: ToolTier::Stone, efficiency: 4.0, max_durability: 131 }, 1, ItemPhysicsProperties { scale: Vec3::new(0.12, 0.35, 0.12), half_height: 0.06, flat_resting: true, ..default() }),
            ItemType::IronPickaxe => ("Iron Pickaxe", ItemKind::Tool { tool_type: ToolType::Pickaxe, tier: ToolTier::Iron, efficiency: 6.0, max_durability: 250 }, 1, ItemPhysicsProperties { scale: Vec3::new(0.12, 0.35, 0.12), half_height: 0.06, flat_resting: true, ..default() }),
            ItemType::WoodenShovel => ("Wooden Shovel", ItemKind::Tool { tool_type: ToolType::Shovel, tier: ToolTier::Wood, efficiency: 2.0, max_durability: 59 }, 1, ItemPhysicsProperties { scale: Vec3::new(0.12, 0.35, 0.12), half_height: 0.06, flat_resting: true, buoyancy: 18.0, ..default() }),
            ItemType::StoneShovel => ("Stone Shovel", ItemKind::Tool { tool_type: ToolType::Shovel, tier: ToolTier::Stone, efficiency: 4.0, max_durability: 131 }, 1, ItemPhysicsProperties { scale: Vec3::new(0.12, 0.35, 0.12), half_height: 0.06, flat_resting: true, ..default() }),
            ItemType::IronShovel => ("Iron Shovel", ItemKind::Tool { tool_type: ToolType::Shovel, tier: ToolTier::Iron, efficiency: 6.0, max_durability: 250 }, 1, ItemPhysicsProperties { scale: Vec3::new(0.12, 0.35, 0.12), half_height: 0.06, flat_resting: true, ..default() }),
            ItemType::WoodenAxe => ("Wooden Axe", ItemKind::Tool { tool_type: ToolType::Axe, tier: ToolTier::Wood, efficiency: 2.0, max_durability: 59 }, 1, ItemPhysicsProperties { scale: Vec3::new(0.12, 0.35, 0.12), half_height: 0.06, flat_resting: true, buoyancy: 18.0, ..default() }),
            ItemType::StoneAxe => ("Stone Axe", ItemKind::Tool { tool_type: ToolType::Axe, tier: ToolTier::Stone, efficiency: 4.0, max_durability: 131 }, 1, ItemPhysicsProperties { scale: Vec3::new(0.12, 0.35, 0.12), half_height: 0.06, flat_resting: true, ..default() }),
            ItemType::IronAxe => ("Iron Axe", ItemKind::Tool { tool_type: ToolType::Axe, tier: ToolTier::Iron, efficiency: 6.0, max_durability: 250 }, 1, ItemPhysicsProperties { scale: Vec3::new(0.12, 0.35, 0.12), half_height: 0.06, flat_resting: true, ..default() }),
            ItemType::FlintAxe => ("Flint Axe", ItemKind::Tool { tool_type: ToolType::Axe, tier: ToolTier::Wood, efficiency: 2.0, max_durability: 45 }, 1, ItemPhysicsProperties { scale: Vec3::new(0.12, 0.35, 0.12), half_height: 0.06, flat_resting: true, buoyancy: 18.0, is_emissive: false }),
        };
        Some(ItemDefinition { id: item, name, kind, max_stack, physics })
    }
}

/// 物品堆疊資料結構 (ItemStack)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemStack {
    pub item_type: ItemType,
    pub count: u16,
    pub durability: Option<u16>,
}

impl ItemStack {
    pub fn new(item_type: ItemType, count: u16, registry: &ItemRegistry) -> Self {
        let durability = if let Some(def) = registry.get(item_type) {
            if let ItemKind::Tool { max_durability, .. } = def.kind {
                Some(max_durability)
            } else {
                None
            }
        } else {
            None
        };
        Self {
            item_type,
            count,
            durability,
        }
    }

    pub fn new_with_durability(item_type: ItemType, count: u16, durability: u16) -> Self {
        Self {
            item_type,
            count,
            durability: Some(durability),
        }
    }

    pub fn max_stack(&self, registry: &ItemRegistry) -> u16 {
        registry.get(self.item_type).map(|def| def.max_stack).unwrap_or(64)
    }

    pub fn max_durability(&self, registry: &ItemRegistry) -> Option<u16> {
        if let Some(def) = registry.get(self.item_type) {
            if let ItemKind::Tool { max_durability, .. } = def.kind {
                return Some(max_durability);
            }
        }
        None
    }

    pub fn is_empty(&self) -> bool {
        self.count == 0 || self.item_type == ItemType::Air
    }

    pub fn can_stack_with(&self, other: &ItemStack, registry: &ItemRegistry) -> bool {
        if self.item_type != other.item_type {
            return false;
        }
        if self.durability != other.durability {
            return false;
        }
        let max_st = self.max_stack(registry);
        max_st > 1
    }

    /// 增加數量並回傳無法放入的剩餘數量。
    pub fn add_count(&mut self, amount: u16, registry: &ItemRegistry) -> u16 {
        let max_st = self.max_stack(registry);
        let space = max_st.saturating_sub(self.count);
        let added = amount.min(space);
        self.count += added;
        amount - added
    }

    /// 扣減數量。若數量扣減至 <= 0，傳回 true (表示此 stack 應被清空)。
    pub fn consume(&mut self, amount: u16) -> bool {
        if amount >= self.count {
            self.count = 0;
            true
        } else {
            self.count -= amount;
            false
        }
    }

    /// 工具扣減耐久度。若 durability 扣至 0，傳回 true (表示工具碎裂毀損)。
    pub fn damage(&mut self, amount: u16) -> bool {
        if let Some(ref mut dur) = self.durability {
            if amount >= *dur {
                *dur = 0;
                true
            } else {
                *dur -= amount;
                false
            }
        } else {
            false
        }
    }
}

/// 通用背包組件 (Inventory Component)
#[derive(Component, Debug, Clone, Serialize, Deserialize)]
pub struct Inventory {
    pub slots: Vec<Option<ItemStack>>,
    pub selected_slot: usize,
    pub carried_item: Option<ItemStack>,
}

impl Inventory {
    pub fn new(capacity: usize) -> Self {
        Self {
            slots: vec![None; capacity],
            selected_slot: 0,
            carried_item: None,
        }
    }

    pub fn capacity(&self) -> usize {
        self.slots.len()
    }

    pub fn slot(&self, index: usize) -> Option<&ItemStack> {
        self.slots.get(index).and_then(|opt| opt.as_ref())
    }

    pub fn slot_mut(&mut self, index: usize) -> Option<&mut ItemStack> {
        self.slots.get_mut(index).and_then(|opt| opt.as_mut())
    }

    pub fn set_slot(&mut self, index: usize, stack: Option<ItemStack>) {
        if index < self.slots.len() {
            if let Some(ref s) = stack {
                if s.is_empty() {
                    self.slots[index] = None;
                    return;
                }
            }
            self.slots[index] = stack;
        }
    }

    pub fn selected_item(&self) -> Option<&ItemStack> {
        self.slot(self.selected_slot)
    }

    pub fn selected_item_mut(&mut self) -> Option<&mut ItemStack> {
        self.slot_mut(self.selected_slot)
    }

    /// 自動加入物品堆疊至背包。
    /// 1. 優先尋找相同 item_type 且未滿的槽位。
    /// 2. 剩餘未放下的物品放入第一個空槽位 (None)。
    /// 3. 若全部放滿，回傳剩餘無法入包的 ItemStack；否則回傳 None。
    pub fn add_item(&mut self, mut stack: ItemStack, registry: &ItemRegistry) -> Option<ItemStack> {
        if stack.is_empty() {
            return None;
        }

        // 階段 1：嘗試與現有相同物品堆疊
        if stack.max_stack(registry) > 1 {
            for slot in self.slots.iter_mut().flatten() {
                if slot.can_stack_with(&stack, registry) {
                    let rem = slot.add_count(stack.count, registry);
                    stack.count = rem;
                    if stack.count == 0 {
                        return None;
                    }
                }
            }
        }

        // 階段 2：放入第一個空槽位
        for slot in self.slots.iter_mut() {
            if slot.is_none() {
                let max_st = stack.max_stack(registry);
                if stack.count <= max_st {
                    *slot = Some(stack);
                    return None;
                } else {
                    let mut fitted = stack.clone();
                    fitted.count = max_st;
                    stack.count -= max_st;
                    *slot = Some(fitted);
                }
            }
        }

        // 階段 3：背包已滿，傳回溢出的 stack
        Some(stack)
    }

    /// 扣減 selected_slot 的物品數量。若數量降為 0，剛性置為 None。
    pub fn consume_selected(&mut self, count: u16) -> bool {
        let selected = self.selected_slot;
        if let Some(stack) = self.slots.get_mut(selected).and_then(|opt| opt.as_mut()) {
            let depleted = stack.consume(count);
            if depleted {
                self.slots[selected] = None;
            }
            true
        } else {
            false
        }
    }

    /// 從背包中扣除指定類型的物品數量。若數量足夠扣除則傳回 true，不足則傳回 false 且不扣除。
    pub fn consume_item(&mut self, item_type: ItemType, mut amount: u16) -> bool {
        let total: u16 = self.slots.iter().flatten().filter(|s| s.item_type == item_type).map(|s| s.count).sum();
        if total < amount {
            return false;
        }
        for slot in self.slots.iter_mut() {
            if let Some(stack) = slot {
                if stack.item_type == item_type {
                    if stack.count <= amount {
                        amount -= stack.count;
                        *slot = None;
                    } else {
                        stack.count -= amount;
                        amount = 0;
                    }
                    if amount == 0 {
                        return true;
                    }
                }
            }
        }
        true
    }

    /// 扣減 selected_slot 工具的耐久度。若耐久度降為 0 (碎裂)，剛性置為 None。
    pub fn damage_selected_tool(&mut self, amount: u16) -> bool {
        let selected = self.selected_slot;
        if let Some(stack) = self.slots.get_mut(selected).and_then(|opt| opt.as_mut()) {
            let broken = stack.damage(amount);
            if broken {
                info!("【物品系統】工具 {:?} 耐久度歸零，已碎裂毀損！", stack.item_type);
                self.slots[selected] = None;
                return true;
            }
        }
        false
    }

    /// 交換兩個槽位的物品堆疊
    pub fn swap_slots(&mut self, a: usize, b: usize) {
        if a < self.slots.len() && b < self.slots.len() {
            self.slots.swap(a, b);
        }
    }

    /// 點擊背包槽位：與游標攜帶之物品進行交互 (拿起/放下/堆疊/置換)
    pub fn click_slot(&mut self, index: usize, registry: &ItemRegistry) {
        if index >= self.slots.len() { return; }

        match (self.carried_item.take(), self.slots[index].take()) {
            (None, None) => {},
            (None, Some(slot_stack)) => {
                // 游標為空，槽位有物 -> 拿起槽位物品
                self.carried_item = Some(slot_stack);
            },
            (Some(carried), None) => {
                // 游標有物，槽位為空 -> 放下游標物品
                self.slots[index] = Some(carried);
            },
            (Some(mut carried), Some(mut slot_stack)) => {
                if slot_stack.can_stack_with(&carried, registry) {
                    // 同類物品 -> 嘗試合併堆疊
                    let rem = slot_stack.add_count(carried.count, registry);
                    self.slots[index] = Some(slot_stack);
                    if rem > 0 {
                        carried.count = rem;
                        self.carried_item = Some(carried);
                    } else {
                        self.carried_item = None;
                    }
                } else {
                    // 不同物品 -> 相互置換 (Swap)
                    self.slots[index] = Some(carried);
                    self.carried_item = Some(slot_stack);
                }
            }
        }
    }

    /// 關閉介面或重置時，自動將游標攜帶之物品返還至背包
    pub fn return_carried_item(&mut self, registry: &ItemRegistry) {
        if let Some(carried) = self.carried_item.take() {
            if let Some(overflow) = self.add_item(carried, registry) {
                // 若背包全部放滿，強行放回快捷列選中槽
                self.set_slot(self.selected_slot, Some(overflow));
            }
        }
    }
}

/// 寫實 3D 物理掉落物實體 (Realistic Dropz-Style Physical Item Entity)
/// 模擬物品在世界中受重力拋射、全向體素碰撞反彈、平躺伏地休止、水體浮力、即時環境光採樣與磁吸吸附
#[derive(Component, Debug, Clone)]
pub struct ItemDrop {
    pub item_stack: ItemStack,
    pub velocity: Vec3,
    pub pickup_delay: f32,    // 初始 0.5s 防立即拾取
    pub lifetime: f32,        // 300.0s 超時銷毀防內存膨脹
    pub on_ground: bool,      // 是否已穩固落地平躺 (Dropz Flat Resting)
    pub resting_yaw: f32,     // 落地休止時的偏轉角
    pub roll_angle: f32,      // 空中翻滾旋轉角
    pub current_voxel: IVec3, // 當前採樣光照之體素座標
}

pub fn spawn_item_drop(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    material: Handle<crate::render::material::VoxelMaterial>,
    stack: ItemStack,
    pos: Vec3,
    velocity: Vec3,
    world: Option<&crate::world::WorldManager>,
) -> Entity {
    let registry = ItemRegistry;
    let def = registry.get(stack.item_type);
    let physics = def.map(|d| d.physics).unwrap_or_default();
    
    // 即時依據世界座標採樣生成點真實環境光，徹底根除生成瞬間極亮閃白！
    let cur_voxel = IVec3::new(
        pos.x.floor() as i32,
        pos.y.floor() as i32,
        pos.z.floor() as i32,
    );
    let (sky_light, block_light) = if let Some(wm) = world {
        let sl = wm.get_sky_light_global(cur_voxel);
        let bl = if physics.is_emissive {
            15
        } else {
            wm.get_block_light_global(cur_voxel)
        };
        (sl, bl)
    } else {
        (15, 0)
    };

    let mesh = meshes.add(crate::render::greedy::build_item_drop_mesh(stack.item_type, sky_light, block_light));
    let random_yaw = ((pos.x * 37.0 + pos.z * 19.0 + pos.y * 11.0).sin() * 3.14159).abs();
    commands.spawn((
        SpatialBundle {
            transform: Transform::from_translation(pos),
            ..default()
        },
        ItemDrop {
            item_stack: stack,
            velocity,
            pickup_delay: 0.5,
            lifetime: 0.0,
            on_ground: false,
            resting_yaw: random_yaw,
            roll_angle: 0.0,
            current_voxel: cur_voxel, // 初始直接鎖定為當前體素座標，杜絕次幀無謂重算
        },
        mesh.clone(),
    )).with_children(|parent| {
        // 透過 -0.5 * scale 局部偏移將體素模型 [0, 1] 原點置中，使所有旋轉與翻滾皆以物品物理質心為軸心！
        parent.spawn(MaterialMeshBundle {
            mesh,
            material,
            transform: Transform::from_translation(-0.5 * physics.scale).with_scale(physics.scale),
            ..default()
        });
    }).id()
}

pub fn update_item_drops(
    mut commands: Commands,
    time: Res<Time>,
    world: Res<crate::world::WorldManager>,
    registry: Res<ItemRegistry>,
    mut q_drops: Query<(Entity, &mut Transform, &mut ItemDrop, Option<&Handle<Mesh>>)>,
    mut q_player: Query<(&Transform, &mut Inventory, &crate::player::Player), Without<ItemDrop>>,
    mut particle_mgr: ResMut<crate::render::particles::ParticleManager>,
    mut meshes: Option<ResMut<Assets<Mesh>>>,
) {
    let dt = time.delta_seconds();
    if dt <= 0.00001 { return; }

    let Ok((p_transform, mut inventory, player)) = q_player.get_single_mut() else { return; };
    let player_head = p_transform.translation + Vec3::new(0.0, 0.9, 0.0);

    for (entity, mut transform, mut drop, maybe_mesh) in q_drops.iter_mut() {
        drop.lifetime += dt;
        drop.pickup_delay = (drop.pickup_delay - dt).max(0.0);

        // 1. 超時 300 秒自動銷毀防內存膨脹
        if drop.lifetime > 300.0 {
            commands.entity(entity).despawn_recursive();
            continue;
        }

        let item_physics = registry.get(drop.item_stack.item_type).map(|d| d.physics).unwrap_or_default();
        let half_h = item_physics.half_height;
        let radius = 0.15;

        // 2. 即時環境光動態更新 (Real-time Voxel Lighting Sampling)
        let cur_voxel = IVec3::new(
            transform.translation.x.floor() as i32,
            transform.translation.y.floor() as i32,
            transform.translation.z.floor() as i32,
        );
        if drop.current_voxel != cur_voxel {
            drop.current_voxel = cur_voxel;
            let sl = world.get_sky_light_global(cur_voxel);
            let bl = if item_physics.is_emissive {
                15 // 註冊表配置為自發光物品 (如火把)
            } else {
                world.get_block_light_global(cur_voxel)
            };
            if let Some(ref mut meshes_asset) = meshes {
                if let Some(mesh_handle) = maybe_mesh {
                    if let Some(mesh) = meshes_asset.get_mut(mesh_handle) {
                        crate::render::greedy::update_single_voxel_mesh_light(mesh, sl, bl);
                    }
                }
            }
        }

        // 3. 水體浮力與阻力 (Fluid Physics / Buoyancy via Registry)
        let is_in_water = world.get_fluid_global(cur_voxel) > 0;
        if is_in_water {
            drop.on_ground = false;
            // 水中阻力減速
            drop.velocity *= (1.0 - 3.0 * dt).max(0.0);
            if item_physics.buoyancy > 0.0 {
                // 具備向上浮力：浮向水面
                let voxel_above = cur_voxel + IVec3::Y;
                let is_water_surface = world.get_fluid_global(voxel_above) == 0;
                if is_water_surface && transform.translation.y >= cur_voxel.y as f32 + 0.85 {
                    drop.velocity.y = (cur_voxel.y as f32 + 0.85 - transform.translation.y) * 3.0;
                } else {
                    drop.velocity.y += item_physics.buoyancy * dt;
                    drop.velocity.y = drop.velocity.y.min(1.5);
                }
            } else {
                // 重物（負浮力）沉底
                drop.velocity.y += item_physics.buoyancy * dt;
                drop.velocity.y = drop.velocity.y.max(-2.0);
            }
        } else if !drop.on_ground {
            // 常規空氣重力
            drop.velocity.y -= 18.0 * dt;
            drop.velocity.y = drop.velocity.y.max(-20.0);
        }

        // 4. 全向 3D 體素 AABB 碰撞與地面檢測 (Voxel AABB Wall Rebound & Ground Collision)
        // (a) X 軸水平碰撞
        if drop.velocity.x.abs() > 0.001 {
            let next_x = transform.translation.x + drop.velocity.x * dt;
            let check_x_voxel = IVec3::new(
                (if drop.velocity.x > 0.0 { next_x + radius } else { next_x - radius }).floor() as i32,
                transform.translation.y.floor() as i32,
                transform.translation.z.floor() as i32,
            );
            if world.get_block_global(check_x_voxel).is_solid() {
                drop.velocity.x = -drop.velocity.x * 0.3; // 彈壁
            } else {
                transform.translation.x = next_x;
            }
        }

        // (b) Z 軸水平碰撞
        if drop.velocity.z.abs() > 0.001 {
            let next_z = transform.translation.z + drop.velocity.z * dt;
            let check_z_voxel = IVec3::new(
                transform.translation.x.floor() as i32,
                transform.translation.y.floor() as i32,
                (if drop.velocity.z > 0.0 { next_z + radius } else { next_z - radius }).floor() as i32,
            );
            if world.get_block_global(check_z_voxel).is_solid() {
                drop.velocity.z = -drop.velocity.z * 0.3; // 彈壁
            } else {
                transform.translation.z = next_z;
            }
        }

        // (c) Y 軸垂直地面碰撞與著地穩定維持 (Stable Ground Resting)
        if drop.on_ground {
            // 已在地面：探測腳底下方支撐體素 (向下探測 0.05 格，防止邊界浮點數向下取整跳變)
            let support_voxel = IVec3::new(
                transform.translation.x.floor() as i32,
                (transform.translation.y - half_h - 0.05).floor() as i32,
                transform.translation.z.floor() as i32,
            );
            let support_block = world.get_block_global(support_voxel);
            if support_block.is_solid() {
                let floor_y = support_voxel.y as f32 + 1.0;
                transform.translation.y = floor_y + half_h;
                drop.velocity.y = 0.0;
                drop.velocity.x *= 0.65;
                drop.velocity.z *= 0.65;
                if drop.velocity.length_squared() < 0.001 {
                    drop.velocity = Vec3::ZERO;
                }
            } else {
                drop.on_ground = false;
            }
        } else {
            let next_y = transform.translation.y + drop.velocity.y * dt;
            let foot_voxel = IVec3::new(
                transform.translation.x.floor() as i32,
                (next_y - half_h).floor() as i32,
                transform.translation.z.floor() as i32,
            );
            let block_below = world.get_block_global(foot_voxel);
            if block_below.is_solid() {
                let floor_y = foot_voxel.y as f32 + 1.0;
                transform.translation.y = floor_y + half_h;
                if drop.velocity.y < -3.5 {
                    drop.velocity.y = -drop.velocity.y * 0.22; // 著地小彈跳
                } else {
                    drop.velocity.y = 0.0;
                    drop.on_ground = true;
                }
                drop.velocity.x *= 0.65;
                drop.velocity.z *= 0.65;
            } else {
                transform.translation.y = next_y;
            }
        }

        // 5. 磁吸拾取 (半徑 1.8 格)
        let mut being_magnetized = false;
        if !player.is_spectator && drop.pickup_delay <= 0.0 {
            let dist = transform.translation.distance(player_head);
            if dist <= 1.8 {
                being_magnetized = true;
                drop.on_ground = false; // 喚醒吸附
                let dir = (player_head - transform.translation).normalize_or_zero();
                let move_dist = (9.0 * dt).min(dist);
                transform.translation += dir * move_dist;

                let current_dist = transform.translation.distance(player_head);
                if current_dist <= 0.45 {
                    if let Some(overflow) = inventory.add_item(drop.item_stack.clone(), &registry) {
                        drop.item_stack = overflow;
                    } else {
                        // 全數拾取完畢
                        let color = crate::render::particles::get_block_debris_color(
                            drop.item_stack.item_type.to_block().unwrap_or(BlockType::OakLog)
                        );
                        particle_mgr.spawn_debris(transform.translation, color, 4);
                        commands.entity(entity).despawn_recursive();
                        continue;
                    }
                }
            }
        }

        // 6. 寫實平躺伏地視覺 (Dropz Flat Resting via Registry)
        if drop.on_ground && !being_magnetized {
            if item_physics.flat_resting {
                // 註冊表配置 flat_resting: 橫臥平躺於地面，不再豎直自轉
                let flat_rot = Quat::from_rotation_x(std::f32::consts::FRAC_PI_2);
                transform.rotation = Quat::from_rotation_y(drop.resting_yaw) * flat_rot;
            } else {
                // 方塊平穩坐落於地面，靜止不旋轉
                transform.rotation = Quat::from_rotation_y(drop.resting_yaw);
            }
        } else {
            // 空中翻滾
            drop.roll_angle += (drop.velocity.length() * 2.0 + 2.0) * dt;
            let tumble = Quat::from_rotation_x(drop.roll_angle);
            transform.rotation = Quat::from_rotation_y(drop.resting_yaw) * tumble;
        }
    }
}

pub struct ItemPlugin;

impl Plugin for ItemPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ItemRegistry>();
        app.add_systems(Update, update_item_drops.run_if(in_state(crate::GameState::InGame)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_item_registry_lookup() {
        let registry = ItemRegistry;
        assert!(registry.get(ItemType::Air).is_none());

        let stone = registry.get(ItemType::Stone).unwrap();
        assert_eq!(stone.name, "Stone");
        assert_eq!(stone.kind, ItemKind::Block(BlockType::Stone));
        assert_eq!(stone.max_stack, 64);

        let iron_pickaxe = registry.get(ItemType::IronPickaxe).unwrap();
        assert_eq!(iron_pickaxe.name, "Iron Pickaxe");
        assert_eq!(iron_pickaxe.max_stack, 1);
        if let ItemKind::Tool { tool_type, tier, efficiency, max_durability } = iron_pickaxe.kind {
            assert_eq!(tool_type, ToolType::Pickaxe);
            assert_eq!(tier, ToolTier::Iron);
            assert_eq!(efficiency, 6.0);
            assert_eq!(max_durability, 250);
        } else {
            panic!("Expected Tool ItemKind");
        }
    }

    #[test]
    fn test_item_stack_creation_and_stacking() {
        let registry = ItemRegistry;
        let mut stone_stack = ItemStack::new(ItemType::Stone, 60, &registry);
        assert_eq!(stone_stack.count, 60);
        assert_eq!(stone_stack.max_stack(&registry), 64);
        assert!(stone_stack.durability.is_none());

        let rem = stone_stack.add_count(10, &registry);
        assert_eq!(stone_stack.count, 64);
        assert_eq!(rem, 6);
    }

    #[test]
    fn test_zero_count_cleanup() {
        let registry = ItemRegistry;
        let mut inv = Inventory::new(9);
        inv.set_slot(0, Some(ItemStack::new(ItemType::Dirt, 2, &registry)));
        assert_eq!(inv.slot(0).unwrap().count, 2);

        inv.consume_selected(1);
        assert_eq!(inv.slot(0).unwrap().count, 1);

        inv.consume_selected(1);
        assert!(inv.slot(0).is_none(), "Count reached 0, slot must be cleaned up to None!");
    }

    #[test]
    fn test_tool_durability_cleanup() {
        let registry = ItemRegistry;
        let mut inv = Inventory::new(9);
        let mut pickaxe = ItemStack::new(ItemType::IronPickaxe, 1, &registry);
        pickaxe.durability = Some(2);
        inv.set_slot(0, Some(pickaxe));

        assert!(!inv.damage_selected_tool(1));
        assert_eq!(inv.slot(0).unwrap().durability, Some(1));

        assert!(inv.damage_selected_tool(1), "Tool should break when durability hits 0");
        assert!(inv.slot(0).is_none(), "Broken tool slot must be cleaned up to None!");
    }

    #[test]
    fn test_inventory_add_item_auto_pickup() {
        let registry = ItemRegistry;
        let mut inv = Inventory::new(9);

        // Add 40 Stone
        let rem1 = inv.add_item(ItemStack::new(ItemType::Stone, 40, &registry), &registry);
        assert!(rem1.is_none());
        assert_eq!(inv.slot(0).unwrap().count, 40);

        // Add another 40 Stone: should stack 24 into slot 0 (up to 64) and put 16 into slot 1
        let rem2 = inv.add_item(ItemStack::new(ItemType::Stone, 40, &registry), &registry);
        assert!(rem2.is_none());
        assert_eq!(inv.slot(0).unwrap().count, 64);
        assert_eq!(inv.slot(1).unwrap().count, 16);
    }

    #[test]
    fn test_block_drops_honor_drop_table() {
        assert_eq!(get_block_drop(BlockType::Glass), None, "Glass should drop nothing");
        assert_eq!(get_block_drop(BlockType::Air), None, "Air should drop nothing");
        assert_eq!(get_block_drop(BlockType::Torch), Some(ItemType::Torch));
        assert_eq!(get_block_drop(BlockType::TorchWallN), Some(ItemType::Torch));
        assert_eq!(get_block_drop(BlockType::Grass), Some(ItemType::Dirt));
    }

    #[test]
    fn test_inventory_click_slot_pickup_and_swap() {
        let registry = ItemRegistry;
        let mut inv = Inventory::new(36);
        inv.set_slot(0, Some(ItemStack::new(ItemType::Stone, 32, &registry)));
        inv.set_slot(1, Some(ItemStack::new(ItemType::Dirt, 10, &registry)));

        // 1. Click slot 0 with empty cursor -> picks up 32 Stone
        inv.click_slot(0, &registry);
        assert!(inv.slot(0).is_none());
        assert_eq!(inv.carried_item.as_ref().unwrap().item_type, ItemType::Stone);
        assert_eq!(inv.carried_item.as_ref().unwrap().count, 32);

        // 2. Click slot 1 (Dirt) while holding Stone -> swaps Stone and Dirt
        inv.click_slot(1, &registry);
        assert_eq!(inv.slot(1).as_ref().unwrap().item_type, ItemType::Stone);
        assert_eq!(inv.slot(1).as_ref().unwrap().count, 32);
        assert_eq!(inv.carried_item.as_ref().unwrap().item_type, ItemType::Dirt);
        assert_eq!(inv.carried_item.as_ref().unwrap().count, 10);

        // 3. Click empty slot 0 -> places Dirt
        inv.click_slot(0, &registry);
        assert_eq!(inv.slot(0).as_ref().unwrap().item_type, ItemType::Dirt);
        assert!(inv.carried_item.is_none());
    }

    #[test]
    fn test_item_drop_physics_and_magnetic_pickup() {
        use bevy::ecs::world::{World as BevyWorld, CommandQueue};
        use crate::world::{WorldManager, ChunkEntry, ChunkLightBuffer, generator};
        use crate::render::particles::ParticleManager;
        use crate::player::Player;

        let mut world = BevyWorld::new();
        world.init_resource::<Time>();
        world.init_resource::<ItemRegistry>();
        world.init_resource::<ParticleManager>();

        let mut wm = WorldManager::default();
        let chunk_pos = IVec3::new(0, 0, 0);
        wm.chunks.insert(chunk_pos, ChunkEntry {
            buffer: std::sync::Arc::new(generator::ChunkBuffer { blocks: [BlockType::Air; 32768] }),
            light_buffer: std::sync::Arc::new(ChunkLightBuffer::default()),
            fluid_buffer: None,
            entity: None,
            is_modified: false,
            is_lighting_ready: true,
        });

        // 鋪設地面 (y = 0 為固體石頭) 防止物品掉入虛空
        let ecs_world = BevyWorld::new();
        let mut queue = CommandQueue::default();
        let mut commands = Commands::new(&mut queue, &ecs_world);
        for x in -2..=2 {
            for z in -2..=2 {
                wm.set_block_global(IVec3::new(x, 0, z), BlockType::Stone, &mut commands);
            }
        }
        world.insert_resource(wm);

        let registry = ItemRegistry;

        // 玩家站立於 (0.0, 1.0, 0.0)，頭部座標為 (0.0, 1.9, 0.0)
        let player_ent = world.spawn((
            Player::default(),
            Inventory::new(36),
            Transform::from_xyz(0.0, 1.0, 0.0),
        )).id();

        // 掉落物生成於 (0.0, 1.5, 0.3)（處於玩家前上方，落地於 y=1.14，處於 1.8m 磁吸半徑內）
        // 初始設置 pickup_delay = 0.5s 防立即拾取
        let drop_ent = world.spawn((
            Transform::from_xyz(0.0, 1.5, 0.3),
            ItemDrop {
                item_stack: ItemStack::new(ItemType::OakLog, 2, &registry),
                velocity: Vec3::ZERO,
                pickup_delay: 0.5,
                lifetime: 0.0,
                on_ground: false,
                resting_yaw: 0.0,
                roll_angle: 0.0,
                current_voxel: IVec3::splat(i32::MIN),
            },
        )).id();

        let mut schedule = Schedule::default();
        schedule.add_systems(update_item_drops);

        // 前 4 個 tick (每個 0.05s，共 0.2s)：掉落物受重力自然落體至地面 (y = 1.14)，此時 pickup_delay 仍在冷卻 (剩餘 0.3s)
        for _ in 0..4 {
            let mut time = world.resource_mut::<Time>();
            time.advance_by(std::time::Duration::from_millis(50));
            schedule.run(&mut world);
        }

        assert!(world.get::<ItemDrop>(drop_ent).is_some(), "在 pickup_delay 冷卻期間掉落物不應被拾取");
        let p_inv = world.get::<Inventory>(player_ent).unwrap();
        assert!(p_inv.slot(0).is_none(), "背包不應提前收納物品");

        // 接續 8 個 tick (每個 0.05s，共 0.4s)：冷卻解除 (pickup_delay 歸零)，掉落物磁吸飛向玩家並納入背包
        for _ in 0..8 {
            let mut time = world.resource_mut::<Time>();
            time.advance_by(std::time::Duration::from_millis(50));
            schedule.run(&mut world);
        }

        // 掉落物已被吸入並銷毀
        assert!(world.get::<ItemDrop>(drop_ent).is_none(), "磁吸完畢後掉落物實體應銷毀");

        // 背包槽位 0 應已收到 2 個 OakLog
        let p_inv = world.get::<Inventory>(player_ent).unwrap();
        let slot = p_inv.slot(0).expect("Slot 0 should contain picked up item");
        assert_eq!(slot.item_type, ItemType::OakLog);
        assert_eq!(slot.count, 2);
    }

    #[test]
    fn test_item_drop_flat_resting_stick_and_block() {
        use bevy::ecs::world::{World as BevyWorld, CommandQueue};
        use crate::world::{WorldManager, ChunkEntry, ChunkLightBuffer, generator};
        use crate::render::particles::ParticleManager;
        use crate::player::Player;

        let mut world = BevyWorld::new();
        world.init_resource::<Time>();
        world.init_resource::<ItemRegistry>();
        world.init_resource::<ParticleManager>();

        let mut wm = WorldManager::default();
        let chunk_pos = IVec3::new(0, 0, 0);
        wm.chunks.insert(chunk_pos, ChunkEntry {
            buffer: std::sync::Arc::new(generator::ChunkBuffer { blocks: [BlockType::Air; 32768] }),
            light_buffer: std::sync::Arc::new(ChunkLightBuffer::default()),
            fluid_buffer: None,
            entity: None,
            is_modified: false,
            is_lighting_ready: true,
        });

        // 鋪設地面 y = 0
        let ecs_world = BevyWorld::new();
        let mut queue = CommandQueue::default();
        let mut commands = Commands::new(&mut queue, &ecs_world);
        for x in -2..=2 {
            for z in -2..=2 {
                wm.set_block_global(IVec3::new(x, 0, z), BlockType::Stone, &mut commands);
            }
        }
        world.insert_resource(wm);

        let registry = ItemRegistry;

        // 玩家站遠離 10 格 (不觸發磁吸)
        world.spawn((
            Player::default(),
            Inventory::new(36),
            Transform::from_xyz(10.0, 10.0, 10.0),
        ));

        // 生成木棒掉落物（初始高度 1.5）
        let stick_ent = world.spawn((
            Transform::from_xyz(0.0, 1.5, 0.0),
            ItemDrop {
                item_stack: ItemStack::new(ItemType::Stick, 1, &registry),
                velocity: Vec3::ZERO,
                pickup_delay: 2.0,
                lifetime: 0.0,
                on_ground: false,
                resting_yaw: 0.5,
                roll_angle: 0.0,
                current_voxel: IVec3::splat(i32::MIN),
            },
        )).id();

        // 生成方塊掉落物（初始高度 1.5）
        let block_ent = world.spawn((
            Transform::from_xyz(1.0, 1.5, 0.0),
            ItemDrop {
                item_stack: ItemStack::new(ItemType::Stone, 1, &registry),
                velocity: Vec3::ZERO,
                pickup_delay: 2.0,
                lifetime: 0.0,
                on_ground: false,
                resting_yaw: 0.2,
                roll_angle: 0.0,
                current_voxel: IVec3::splat(i32::MIN),
            },
        )).id();

        let mut schedule = Schedule::default();
        schedule.add_systems(update_item_drops);

        // 推進 15 個 tick (共 0.75s) 使其著地彈跳後完全靜止休止
        for _ in 0..15 {
            let mut time = world.resource_mut::<Time>();
            time.advance_by(std::time::Duration::from_millis(50));
            schedule.run(&mut world);
        }

        // 驗證木棒平躺伏地：on_ground 為 true，高度貼合 1.04，旋轉為橫躺 (X 軸傾角 90 度)
        let stick_drop = world.get::<ItemDrop>(stick_ent).unwrap();
        let stick_trans = world.get::<Transform>(stick_ent).unwrap();
        assert!(stick_drop.on_ground, "木棒著地後應進入平躺伏地狀態");
        assert!((stick_trans.translation.y - 1.04).abs() < 0.01, "木棒伏地高度應為 1.04 (floor 1.0 + half_h 0.04)");

        // 驗證方塊平穩就坐：on_ground 為 true，高度貼合 1.14
        let block_drop = world.get::<ItemDrop>(block_ent).unwrap();
        let block_trans = world.get::<Transform>(block_ent).unwrap();
        assert!(block_drop.on_ground, "方塊著地後應穩固休止");
        assert!((block_trans.translation.y - 1.14).abs() < 0.01, "方塊坐落高度應為 1.14 (floor 1.0 + half_h 0.14)");

        // 🔥 關鍵防線：連續推進 30 個 tick (模擬 60Hz 運行 0.5 秒)，嚴格驗證 on_ground 恆定為 true，絕不出現邊界跳變或雙重虛影 (Zero-Flicker Ground Stability)
        for tick in 0..30 {
            let mut time = world.resource_mut::<Time>();
            time.advance_by(std::time::Duration::from_millis(16));
            schedule.run(&mut world);

            let s_drop = world.get::<ItemDrop>(stick_ent).unwrap();
            let b_drop = world.get::<ItemDrop>(block_ent).unwrap();
            let s_trans = world.get::<Transform>(stick_ent).unwrap();
            let b_trans = world.get::<Transform>(block_ent).unwrap();

            assert!(s_drop.on_ground, "第 {} tick 木棒應穩定伏地，絕不允許閃爍/跳變為空中翻滾狀態 (導致虛影)", tick);
            assert!(b_drop.on_ground, "第 {} tick 方塊應穩定就坐，絕不允許閃爍/跳變為空中翻滾狀態 (導致虛影)", tick);
            assert!((s_trans.translation.y - 1.04).abs() < 0.001, "木棒伏地高度應精準維持 1.04，不可上下抖動");
            assert!((b_trans.translation.y - 1.14).abs() < 0.001, "方塊伏地高度應精準維持 1.14，不可上下抖動");
        }
    }

    #[test]
    fn test_item_drop_horizontal_aabb_wall_rebound() {
        use bevy::ecs::world::{World as BevyWorld, CommandQueue};
        use crate::world::{WorldManager, ChunkEntry, ChunkLightBuffer, generator};
        use crate::render::particles::ParticleManager;
        use crate::player::Player;

        let mut world = BevyWorld::new();
        world.init_resource::<Time>();
        world.init_resource::<ItemRegistry>();
        world.init_resource::<ParticleManager>();

        let mut wm = WorldManager::default();
        let chunk_pos = IVec3::new(0, 0, 0);
        wm.chunks.insert(chunk_pos, ChunkEntry {
            buffer: std::sync::Arc::new(generator::ChunkBuffer { blocks: [BlockType::Air; 32768] }),
            light_buffer: std::sync::Arc::new(ChunkLightBuffer::default()),
            fluid_buffer: None,
            entity: None,
            is_modified: false,
            is_lighting_ready: true,
        });

        // 在 x=1, y=1 放置固體石牆，防止物品向 +X 穿透
        let ecs_world = BevyWorld::new();
        let mut queue = CommandQueue::default();
        let mut commands = Commands::new(&mut queue, &ecs_world);
        wm.set_block_global(IVec3::new(1, 1, 0), BlockType::Stone, &mut commands);
        world.insert_resource(wm);

        let registry = ItemRegistry;

        world.spawn((
            Player::default(),
            Inventory::new(36),
            Transform::from_xyz(10.0, 10.0, 10.0),
        ));

        // 掉落物從 x=0.6, y=1.2 帶著強烈向 +X 初速 (vx = 5.0) 飛向石牆
        let drop_ent = world.spawn((
            Transform::from_xyz(0.6, 1.2, 0.0),
            ItemDrop {
                item_stack: ItemStack::new(ItemType::Dirt, 1, &registry),
                velocity: Vec3::new(5.0, 0.0, 0.0),
                pickup_delay: 2.0,
                lifetime: 0.0,
                on_ground: false,
                resting_yaw: 0.0,
                roll_angle: 0.0,
                current_voxel: IVec3::splat(i32::MIN),
            },
        )).id();

        let mut schedule = Schedule::default();
        schedule.add_systems(update_item_drops);

        // 推進 2 個 tick (共 0.1s)
        for _ in 0..2 {
            let mut time = world.resource_mut::<Time>();
            time.advance_by(std::time::Duration::from_millis(50));
            schedule.run(&mut world);
        }

        let drop = world.get::<ItemDrop>(drop_ent).unwrap();
        let trans = world.get::<Transform>(drop_ent).unwrap();

        // 驗證掉落物碰壁彈回：速度反向 (vx < 0)，且絕對未穿透進 x >= 1.0 的石牆內部
        assert!(drop.velocity.x < 0.0, "撞擊固體牆壁後速度應反彈 (vx < 0)");
        assert!(trans.translation.x < 1.0, "全向 AABB 體素碰撞嚴格防止物品穿透進固體方塊內部");
    }

    #[test]
    fn test_item_physics_registry_properties() {
        let registry = ItemRegistry;

        // 驗證木質浮力與平躺註冊
        let stick_def = registry.get(ItemType::Stick).expect("Stick definition exists");
        assert_eq!(stick_def.physics.buoyancy, 18.0, "木棒應具備 +18.0 水中浮力");
        assert!(stick_def.physics.flat_resting, "木棒應配置平躺橫臥姿勢");
        assert_eq!(stick_def.physics.half_height, 0.04, "木棒橫臥半高應為 0.04");

        let log_def = registry.get(ItemType::OakLog).expect("OakLog definition exists");
        assert_eq!(log_def.physics.buoyancy, 18.0, "原木應具備浮力");
        assert!(!log_def.physics.flat_resting, "原木方塊坐落不平躺");

        // 驗證火把自發光註冊
        let torch_def = registry.get(ItemType::Torch).expect("Torch definition exists");
        assert!(torch_def.physics.is_emissive, "火把應註冊為自體發光物品");

        // 驗證常規石頭沉底與無自發光
        let stone_def = registry.get(ItemType::Stone).expect("Stone definition exists");
        assert!(stone_def.physics.buoyancy < 0.0, "石頭應為負浮力沉底");
        assert!(!stone_def.physics.is_emissive, "石頭不自發光");
    }
}

