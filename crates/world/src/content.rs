use ahash::AHashMap;
use serde::{Deserialize, Serialize};
use world_sim_core::{ResourceType, Skill, Trait};

/// Defines a GOAP action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionDefinition {
    pub id: String,
    pub name: String,
    pub base_cost: f32,
    pub intended_use: u8, // 0-100, for A* heuristic optimization
    pub required_skill: Option<(Skill, f32)>,
    pub preconditions: Vec<String>, // Placeholder for expression system
    pub effects: Vec<String>, // Placeholder for expression system
}

/// Defines an item type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemDefinition {
    pub id: String,
    pub name: String,
    pub resource_type: ResourceType,
    pub can_burn: bool,
    pub can_build: bool,
    pub weight: f32,
    pub base_value: f32,
}

/// Defines a crafting recipe
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recipe {
    pub id: String,
    pub output: String, // Item ID
    pub output_quantity: u32,
    pub inputs: Vec<(String, u32)>, // (Item ID, quantity)
    pub required_skill: Option<(Skill, f32)>,
    pub crafting_time: f32,
}

/// Defines a personality trait
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraitDefinition {
    pub trait_type: Trait,
    pub name: String,
    pub description: String,
    pub action_modifiers: Vec<(String, f32)>, // (action_id, cost_multiplier)
}

/// Central content database - the "schema" of all possible content
pub struct ContentDefinitionLayer {
    actions: AHashMap<String, ActionDefinition>,
    items: AHashMap<String, ItemDefinition>,
    recipes: AHashMap<String, Recipe>,
    traits: AHashMap<Trait, TraitDefinition>,
}

impl ContentDefinitionLayer {
    pub fn new() -> Self {
        let mut layer = Self {
            actions: AHashMap::new(),
            items: AHashMap::new(),
            recipes: AHashMap::new(),
            traits: AHashMap::new(),
        };
        
        layer.initialize_default_content();
        layer
    }

    /// Initialize with some default content
    fn initialize_default_content(&mut self) {
        // Add some basic items
        self.items.insert(
            "wood".to_string(),
            ItemDefinition {
                id: "wood".to_string(),
                name: "Wood".to_string(),
                resource_type: ResourceType::Wood,
                can_burn: true,
                can_build: true,
                weight: 10.0,
                base_value: 5.0,
            },
        );

        self.items.insert(
            "stone".to_string(),
            ItemDefinition {
                id: "stone".to_string(),
                name: "Stone".to_string(),
                resource_type: ResourceType::Stone,
                can_burn: false,
                can_build: true,
                weight: 20.0,
                base_value: 3.0,
            },
        );

        self.items.insert(
            "food".to_string(),
            ItemDefinition {
                id: "food".to_string(),
                name: "Food".to_string(),
                resource_type: ResourceType::Food,
                can_burn: false,
                can_build: false,
                weight: 1.0,
                base_value: 10.0,
            },
        );

        // Add basic actions
        self.actions.insert(
            "chop_wood".to_string(),
            ActionDefinition {
                id: "chop_wood".to_string(),
                name: "Chop Wood".to_string(),
                base_cost: 10.0,
                intended_use: 80,
                required_skill: Some((Skill::Woodcutting, 0.0)),
                preconditions: vec!["HasAxe".to_string(), "NearTree".to_string()],
                effects: vec!["HasWood".to_string()],
            },
        );

        self.actions.insert(
            "eat".to_string(),
            ActionDefinition {
                id: "eat".to_string(),
                name: "Eat".to_string(),
                base_cost: 1.0,
                intended_use: 95,
                required_skill: None,
                preconditions: vec!["HasFood".to_string()],
                effects: vec!["NotHungry".to_string()],
            },
        );

        // Add basic recipes
        self.recipes.insert(
            "wooden_plank".to_string(),
            Recipe {
                id: "wooden_plank".to_string(),
                output: "wooden_plank".to_string(),
                output_quantity: 4,
                inputs: vec![("wood".to_string(), 1)],
                required_skill: Some((Skill::Crafting, 5.0)),
                crafting_time: 5.0,
            },
        );

        // Add trait definitions
        self.traits.insert(
            Trait::Brave,
            TraitDefinition {
                trait_type: Trait::Brave,
                name: "Brave".to_string(),
                description: "Fearless in combat".to_string(),
                action_modifiers: vec![
                    ("fight".to_string(), 0.5),
                    ("run_away".to_string(), 2.0),
                ],
            },
        );
    }

    pub fn get_action(&self, id: &str) -> Option<&ActionDefinition> {
        self.actions.get(id)
    }

    pub fn get_item(&self, id: &str) -> Option<&ItemDefinition> {
        self.items.get(id)
    }

    pub fn get_recipe(&self, id: &str) -> Option<&Recipe> {
        self.recipes.get(id)
    }

    pub fn get_trait(&self, trait_type: &Trait) -> Option<&TraitDefinition> {
        self.traits.get(trait_type)
    }

    pub fn all_actions(&self) -> Vec<&ActionDefinition> {
        self.actions.values().collect()
    }

    pub fn all_items(&self) -> Vec<&ItemDefinition> {
        self.items.values().collect()
    }
}

impl Default for ContentDefinitionLayer {
    fn default() -> Self {
        Self::new()
    }
}

