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
}
