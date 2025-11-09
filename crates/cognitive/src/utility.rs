use serde::{Deserialize, Serialize};
use world_sim_agents::SimAgent;
use world_sim_core::math;

/// Utility AI - The "emotional engine" that decides what to want
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UtilityAI {
    pub urges: Vec<Urge>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Urge {
    pub urge_type: UrgeType,
    pub weight: f32,
    pub current_value: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UrgeType {
    Hunger,
    Thirst,
    Tiredness,
    Safety,
    PersonalWealth,
    FactionLoyalty,
    SocialConnection,
    Curiosity,
    Revenge,
    Comfort,
}

impl Urge {
    pub fn new(urge_type: UrgeType, weight: f32) -> Self {
        Self {
            urge_type,
            weight,
            current_value: 0.0,
        }
    }

    /// Calculate the score (utility) of this urge
    pub fn score(&self) -> f32 {
        // Use sigmoid curve for natural urgency
        let normalized = math::sigmoid(self.current_value - 5.0);
        normalized * self.weight
    }
}

impl UtilityAI {
    pub fn new() -> Self {
        Self {
            urges: vec![
                Urge::new(UrgeType::Hunger, 1.0),
                Urge::new(UrgeType::Thirst, 1.0),
                Urge::new(UrgeType::Tiredness, 0.8),
                Urge::new(UrgeType::Safety, 1.5),
                Urge::new(UrgeType::PersonalWealth, 0.5),
                Urge::new(UrgeType::FactionLoyalty, 0.3),
                Urge::new(UrgeType::SocialConnection, 0.6),
            ],
        }
    }

    /// Update urges based on agent state
    pub fn update(&mut self, agent: &SimAgent, delta_time: f32) {
        for urge in &mut self.urges {
            match urge.urge_type {
                UrgeType::Hunger => {
                    urge.current_value += delta_time * 0.1;
                }
                UrgeType::Thirst => {
                    urge.current_value += delta_time * 0.15;
                }
                UrgeType::Tiredness => {
                    urge.current_value += delta_time * 0.05;
                }
                UrgeType::Safety => {
                    // Safety urge increases if in danger
                    urge.current_value = if matches!(agent.state, world_sim_agents::AgentState::Fighting { .. }) {
                        10.0
                    } else {
                        0.0
                    };
                }
                UrgeType::PersonalWealth => {
                    // Modified by personality
                    if agent.has_trait(world_sim_core::Trait::Greedy) {
                        urge.weight = 2.0;
                    } else if agent.has_trait(world_sim_core::Trait::Generous) {
                        urge.weight = 0.2;
                    }
                }
                _ => {}
            }
            
            // Clamp values
            urge.current_value = math::clamp(urge.current_value, 0.0, 10.0);
        }
    }

    /// Get the highest priority goal based on urges
    pub fn get_top_goal(&self) -> Goal {
        let mut max_score = 0.0;
        let mut top_urge = UrgeType::Hunger;
        
        for urge in &self.urges {
            let score = urge.score();
            if score > max_score {
                max_score = score;
                top_urge = urge.urge_type;
            }
        }
        
        // Convert urge to goal
        match top_urge {
            UrgeType::Hunger => Goal::new("NotHungry"),
            UrgeType::Thirst => Goal::new("NotThirsty"),
            UrgeType::Tiredness => Goal::new("Rested"),
            UrgeType::Safety => Goal::new("Safe"),
            UrgeType::PersonalWealth => Goal::new("Wealthy"),
            UrgeType::FactionLoyalty => Goal::new("ServeFaction"),
            UrgeType::SocialConnection => Goal::new("HasFriends"),
            UrgeType::Curiosity => Goal::new("Explored"),
            UrgeType::Revenge => Goal::new("Avenged"),
            UrgeType::Comfort => Goal::new("Comfortable"),
        }
    }

    /// Satisfy an urge (reduce its value)
    pub fn satisfy(&mut self, urge_type: UrgeType, amount: f32) {
        if let Some(urge) = self.urges.iter_mut().find(|u| u.urge_type == urge_type) {
            urge.current_value -= amount;
            urge.current_value = urge.current_value.max(0.0);
        }
    }
}

impl Default for UtilityAI {
    fn default() -> Self {
        Self::new()
    }
}

/// A goal for the GOAP planner
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Goal {
    pub condition: String,
}

impl Goal {
    pub fn new(condition: &str) -> Self {
        Self {
            condition: condition.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_utility_ai() {
        let mut utility = UtilityAI::new();
        
        // Increase hunger
        if let Some(hunger) = utility.urges.iter_mut().find(|u| u.urge_type == UrgeType::Hunger) {
            hunger.current_value = 8.0;
        }
        
        let goal = utility.get_top_goal();
        assert_eq!(goal.condition, "NotHungry");
    }
}

