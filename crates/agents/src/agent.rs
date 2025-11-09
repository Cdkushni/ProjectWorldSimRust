use serde::{Deserialize, Serialize};
use world_sim_core::{AgentId, Attributes, GridCoord, Position, Skill, Trait};
use crate::{AgentDomain, PersonalityProfile, SkillDatabase};

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
}

/// Current behavioral state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentState {
    Idle,
    Moving { destination: GridCoord },
    Working { task: String },
    Fighting { target: AgentId },
    Sleeping,
    Eating,
    Dead,
}

impl SimAgent {
    pub fn new(name: String, position: Position) -> Self {
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

