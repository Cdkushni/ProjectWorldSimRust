use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use world_sim_core::{AgentId, Attributes, GridCoord, Position, ResourceType, Skill, Trait};
use crate::{AgentDomain, PersonalityProfile, SkillDatabase};

/// Resources an agent is carrying to a construction site
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildingResources {
    pub wood: u32,
    pub stone: u32,
    pub iron: u32,
    pub target_building_id: uuid::Uuid, // Which building they're delivering to
}

/// The core agent structure - represents a single individual
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimAgent {
    pub id: AgentId,
    pub name: String,
    pub position: Position,
    pub age: u32,
    pub attributes: Attributes,
    pub personality: PersonalityProfile,
    pub skills: SkillDatabase,
    pub domain: AgentDomain,
    pub state: AgentState,
    pub job: Job,
    pub social_class: SocialClass,
    pub leader_id: Option<AgentId>, // For followers (knights follow king, etc.)
    
    // Economic system
    pub wallet: f64,                                  // Money owned
    pub inventory: HashMap<ResourceType, u32>,        // Resources carried/owned
    pub needs: HashMap<ResourceType, u32>,            // What they want to buy
    pub carrying_resources: Option<BuildingResources>, // Resources being carried to build site
}

/// Current behavioral state
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AgentState {
    Idle,
    Moving { destination: GridCoord },
    Working { task: String },
    Fighting { target: AgentId },
    Sleeping,
    Eating,
    Dead,
    Talking { with: AgentId },        // Having a conversation
    Patrolling { route_index: usize }, // Soldiers patrolling
    Following { leader: AgentId },     // Knights following king
    Building { building_type: String }, // Constructing buildings
    Trading { with: AgentId },         // Merchants trading
}

/// Agent job/profession
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Job {
    Woodcutter,
    Miner,
    Farmer,
    Builder,
    Unemployed,
}

/// Social hierarchy class
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SocialClass {
    King,       // 1 per faction - leader
    Noble,      // ~5% - landowners, advisors
    Knight,     // ~8% - elite warriors, king's guard
    Soldier,    // ~15% - military, guards, patrol
    Merchant,   // ~12% - traders, shopkeepers
    Burgher,    // ~10% - craftsmen, guild members
    Cleric,     // ~5% - priests, healers
    Peasant,    // ~44% - farmers, laborers, commoners
}

impl SimAgent {
    pub fn new(name: String, position: Position) -> Self {
        Self::new_with_class(name, position, SocialClass::Peasant)
    }
    
    pub fn new_with_class(name: String, position: Position, social_class: SocialClass) -> Self {
        // Assign job based on social class
        let job = match social_class {
            SocialClass::King | SocialClass::Noble | SocialClass::Knight => Job::Unemployed,
            SocialClass::Soldier => Job::Unemployed, // Full-time military
            SocialClass::Merchant | SocialClass::Burgher => Job::Builder, // Craftsmen
            SocialClass::Cleric => Job::Unemployed, // Religious duties
            SocialClass::Peasant => {
                match rand::random::<u32>() % 3 {
                    0 => Job::Woodcutter,
                    1 => Job::Miner,
                    _ => Job::Farmer,
                }
            }
        };
        
        // Initial wallet based on social class
        let wallet = match social_class {
            SocialClass::King => 1000.0,
            SocialClass::Noble => 500.0,
            SocialClass::Knight => 200.0,
            SocialClass::Merchant | SocialClass::Burgher => 150.0,
            SocialClass::Soldier => 100.0,
            SocialClass::Cleric => 80.0,
            SocialClass::Peasant => 50.0,
        };
        
        // Basic needs for all agents
        let mut needs = HashMap::new();
        needs.insert(ResourceType::Food, 5);  // Everyone needs food
        
        // Job-specific needs
        match job {
            Job::Builder => {
                needs.insert(ResourceType::Wood, 10);
                needs.insert(ResourceType::Stone, 10);
            }
            Job::Farmer => {
                needs.insert(ResourceType::Wood, 2); // For tools
            }
            Job::Woodcutter => {
                needs.insert(ResourceType::Iron, 1); // For axe
            }
            Job::Miner => {
                needs.insert(ResourceType::Iron, 1); // For pickaxe
            }
            _ => {}
        }
        
        Self {
            id: AgentId::new(),
            name,
            position,
            age: rand::random::<u32>() % 60 + 18, // 18-78 years
            attributes: Attributes::default(),
            personality: PersonalityProfile::random(),
            skills: SkillDatabase::new(),
            domain: AgentDomain::new(),
            state: AgentState::Idle,
            job,
            social_class,
            leader_id: None,
            wallet,
            inventory: HashMap::new(),
            needs,
            carrying_resources: None,
        }
    }

    /// Check if the agent is alive
    pub fn is_alive(&self) -> bool {
        !matches!(self.state, AgentState::Dead)
    }

    /// Get skill level
    pub fn get_skill(&self, skill: Skill) -> f32 {
        self.skills.get_level(skill)
    }

    /// Increase skill level
    pub fn gain_skill_experience(&mut self, skill: Skill, amount: f32) {
        self.skills.add_experience(skill, amount);
    }

    /// Check if agent has a trait
    pub fn has_trait(&self, trait_type: Trait) -> bool {
        self.personality.has_trait(trait_type)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_creation() {
        let agent = SimAgent::new("Test Agent".to_string(), Position::new(0.0, 0.0, 0.0));
        assert!(agent.is_alive());
        assert_eq!(agent.name, "Test Agent");
    }
}

