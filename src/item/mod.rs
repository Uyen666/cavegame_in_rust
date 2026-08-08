#![allow(dead_code)]
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use crate::world::{BlockType, registry::{ToolTier, ToolType}};

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

pub fn get_block_drop(block: BlockType) -> Option<ItemType> {
    match block {
        BlockType::Grass => Some(ItemType::Dirt),
        BlockType::CoalOre => Some(ItemType::Coal),
        BlockType::Air => None,
        other => ItemType::from_block(other),
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

#[derive(Debug, Clone, Copy)]
pub struct ItemDefinition {
    pub id: ItemType,
    pub name: &'static str,
    pub kind: ItemKind,
    pub max_stack: u16,
}

#[derive(Resource, Debug, Clone, Default)]
pub struct ItemRegistry;

impl ItemRegistry {
    pub fn get(&self, item: ItemType) -> Option<ItemDefinition> {
        match item {
            ItemType::Air => None,
            ItemType::Grass => Some(ItemDefinition { id: item, name: "Grass Block", kind: ItemKind::Block(BlockType::Grass), max_stack: 64 }),
            ItemType::Stone => Some(ItemDefinition { id: item, name: "Stone", kind: ItemKind::Block(BlockType::Stone), max_stack: 64 }),
            ItemType::Dirt => Some(ItemDefinition { id: item, name: "Dirt", kind: ItemKind::Block(BlockType::Dirt), max_stack: 64 }),
            ItemType::OakLog => Some(ItemDefinition { id: item, name: "Oak Log", kind: ItemKind::Block(BlockType::OakLog), max_stack: 64 }),
            ItemType::OakLeaves => Some(ItemDefinition { id: item, name: "Oak Leaves", kind: ItemKind::Block(BlockType::OakLeaves), max_stack: 64 }),
            ItemType::Sand => Some(ItemDefinition { id: item, name: "Sand", kind: ItemKind::Block(BlockType::Sand), max_stack: 64 }),
            ItemType::Gravel => Some(ItemDefinition { id: item, name: "Gravel", kind: ItemKind::Block(BlockType::Gravel), max_stack: 64 }),
            ItemType::CoalOre => Some(ItemDefinition { id: item, name: "Coal Ore", kind: ItemKind::Block(BlockType::CoalOre), max_stack: 64 }),
            ItemType::IronOre => Some(ItemDefinition { id: item, name: "Iron Ore", kind: ItemKind::Block(BlockType::IronOre), max_stack: 64 }),
            ItemType::Glass => Some(ItemDefinition { id: item, name: "Glass", kind: ItemKind::Block(BlockType::Glass), max_stack: 64 }),
            ItemType::Torch => Some(ItemDefinition { id: item, name: "Torch", kind: ItemKind::Block(BlockType::Torch), max_stack: 64 }),

            ItemType::Coal => Some(ItemDefinition { id: item, name: "Coal", kind: ItemKind::Material, max_stack: 64 }),
            ItemType::IronIngot => Some(ItemDefinition { id: item, name: "Iron Ingot", kind: ItemKind::Material, max_stack: 64 }),
            ItemType::Stick => Some(ItemDefinition { id: item, name: "Stick", kind: ItemKind::Material, max_stack: 64 }),

            ItemType::WoodenPickaxe => Some(ItemDefinition { id: item, name: "Wooden Pickaxe", kind: ItemKind::Tool { tool_type: ToolType::Pickaxe, tier: ToolTier::Wood, efficiency: 2.0, max_durability: 59 }, max_stack: 1 }),
            ItemType::StonePickaxe => Some(ItemDefinition { id: item, name: "Stone Pickaxe", kind: ItemKind::Tool { tool_type: ToolType::Pickaxe, tier: ToolTier::Stone, efficiency: 4.0, max_durability: 131 }, max_stack: 1 }),
            ItemType::IronPickaxe => Some(ItemDefinition { id: item, name: "Iron Pickaxe", kind: ItemKind::Tool { tool_type: ToolType::Pickaxe, tier: ToolTier::Iron, efficiency: 6.0, max_durability: 250 }, max_stack: 1 }),
            ItemType::WoodenShovel => Some(ItemDefinition { id: item, name: "Wooden Shovel", kind: ItemKind::Tool { tool_type: ToolType::Shovel, tier: ToolTier::Wood, efficiency: 2.0, max_durability: 59 }, max_stack: 1 }),
            ItemType::StoneShovel => Some(ItemDefinition { id: item, name: "Stone Shovel", kind: ItemKind::Tool { tool_type: ToolType::Shovel, tier: ToolTier::Stone, efficiency: 4.0, max_durability: 131 }, max_stack: 1 }),
            ItemType::IronShovel => Some(ItemDefinition { id: item, name: "Iron Shovel", kind: ItemKind::Tool { tool_type: ToolType::Shovel, tier: ToolTier::Iron, efficiency: 6.0, max_durability: 250 }, max_stack: 1 }),
            ItemType::WoodenAxe => Some(ItemDefinition { id: item, name: "Wooden Axe", kind: ItemKind::Tool { tool_type: ToolType::Axe, tier: ToolTier::Wood, efficiency: 2.0, max_durability: 59 }, max_stack: 1 }),
            ItemType::StoneAxe => Some(ItemDefinition { id: item, name: "Stone Axe", kind: ItemKind::Tool { tool_type: ToolType::Axe, tier: ToolTier::Stone, efficiency: 4.0, max_durability: 131 }, max_stack: 1 }),
            ItemType::IronAxe => Some(ItemDefinition { id: item, name: "Iron Axe", kind: ItemKind::Tool { tool_type: ToolType::Axe, tier: ToolTier::Iron, efficiency: 6.0, max_durability: 250 }, max_stack: 1 }),
        }
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
}

impl Inventory {
    pub fn new(capacity: usize) -> Self {
        Self {
            slots: vec![None; capacity],
            selected_slot: 0,
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
}

pub struct ItemPlugin;

impl Plugin for ItemPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ItemRegistry>();
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
}

