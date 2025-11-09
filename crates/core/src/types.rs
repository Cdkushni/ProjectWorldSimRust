use serde::{Deserialize, Serialize};
use std::fmt;

/// Block types in the voxel grid
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BlockType {
    Air,
    WallStone,
    WallWood,
    Water,
    Dirt,
    Grass,
    Wood,
    BurningWood,
    Stone,
    Iron,
    Gold,
}

impl BlockType {
    pub fn is_solid(&self) -> bool {
        !matches!(self, BlockType::Air | BlockType::Water)
    }

    pub fn is_walkable(&self) -> bool {
        matches!(self, BlockType::Air | BlockType::Grass)
    }
}

/// Resource types for the economy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ResourceType {
    Wood,
    Stone,
    Iron,
    Gold,
    Food,
    Water,
    Cloth,
    Tool,
    Weapon,
    Coin,
}

impl fmt::Display for ResourceType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// Skills that agents can learn
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Skill {
    Mining,
    Woodcutting,
    Farming,
    Blacksmithing,
    Crafting,
    Combat,
    Diplomacy,
    Trading,
    Construction,
    Medicine,
}

/// Personality traits
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Trait {
    Brave,
    Cowardly,
    Greedy,
    Generous,
    Honest,
    Deceptive,
    Loyal,
    Rebellious,
    Ambitious,
    Content,
    Aggressive,
    Peaceful,
}

impl Trait {
    /// Get the cost multiplier for actions affected by this trait
    pub fn action_cost_modifier(&self, action_type: &str) -> f32 {
        match (self, action_type) {
            (Trait::Brave, "Fight") => 0.5,
            (Trait::Brave, "RunAway") => 2.0,
            (Trait::Cowardly, "Fight") => 2.0,
            (Trait::Cowardly, "RunAway") => 0.5,
            (Trait::Greedy, "GiveItem") => 2.0,
            (Trait::Greedy, "TakeItem") => 0.7,
            (Trait::Generous, "GiveItem") => 0.5,
            (Trait::Honest, "Lie") => 3.0,
            (Trait::Deceptive, "Lie") => 0.5,
            _ => 1.0,
        }
    }
}

/// Agent attributes (stats)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attributes {
    pub strength: f32,
    pub intelligence: f32,
    pub charisma: f32,
    pub constitution: f32,
    pub agility: f32,
}

impl Default for Attributes {
    fn default() -> Self {
        Self {
            strength: 10.0,
            intelligence: 10.0,
            charisma: 10.0,
            constitution: 10.0,
            agility: 10.0,
        }
    }
}

/// Time representation
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct SimTime {
    pub ticks: u64,
    pub seconds: f64,
}

impl SimTime {
    pub fn new() -> Self {
        Self {
            ticks: 0,
            seconds: 0.0,
        }
    }

    pub fn advance(&mut self, delta_seconds: f64) {
        self.seconds += delta_seconds;
        self.ticks += 1;
    }
}

impl Default for SimTime {
    fn default() -> Self {
        Self::new()
    }
}

