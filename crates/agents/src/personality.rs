use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use world_sim_core::Trait;

/// An agent's personality profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonalityProfile {
    pub traits: HashSet<Trait>,
    pub beliefs: Beliefs,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Beliefs {
    pub worldview: String,
    pub faction_loyalty: Option<world_sim_core::FactionId>,
    pub custom_beliefs: Vec<(String, String)>, // (subject, belief)
}

impl PersonalityProfile {
    pub fn new() -> Self {
        Self {
            traits: HashSet::new(),
            beliefs: Beliefs {
                worldview: "Neutral".to_string(),
                faction_loyalty: None,
                custom_beliefs: Vec::new(),
            },
        }
    }

    /// Generate a random personality
    pub fn random() -> Self {
        let mut rng = rand::thread_rng();
        let mut profile = Self::new();
        
        // Randomly add 2-4 traits
        let all_traits = [
            Trait::Brave,
            Trait::Cowardly,
            Trait::Greedy,
            Trait::Generous,
            Trait::Honest,
            Trait::Deceptive,
            Trait::Loyal,
            Trait::Rebellious,
            Trait::Ambitious,
            Trait::Content,
            Trait::Aggressive,
            Trait::Peaceful,
        ];
        
        let trait_count = rng.gen_range(2..=4);
        for _ in 0..trait_count {
            let trait_type = all_traits[rng.gen_range(0..all_traits.len())];
            profile.traits.insert(trait_type);
        }
        
        profile
    }

    /// Check if has a specific trait
    pub fn has_trait(&self, trait_type: Trait) -> bool {
        self.traits.contains(&trait_type)
    }

    /// Add a trait
    pub fn add_trait(&mut self, trait_type: Trait) {
        self.traits.insert(trait_type);
    }

    /// Get action cost modifier based on traits
    pub fn get_action_cost_modifier(&self, action_type: &str) -> f32 {
        let mut modifier = 1.0;
        for trait_type in &self.traits {
            modifier *= trait_type.action_cost_modifier(action_type);
        }
        modifier
    }

    /// Add a belief
    pub fn add_belief(&mut self, subject: String, belief: String) {
        self.beliefs.custom_beliefs.push((subject, belief));
    }

    /// Get belief about a subject
    pub fn get_belief(&self, subject: &str) -> Option<&String> {
        self.beliefs
            .custom_beliefs
            .iter()
            .find(|(s, _)| s == subject)
            .map(|(_, b)| b)
    }
}

impl Default for PersonalityProfile {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_personality_traits() {
        let mut profile = PersonalityProfile::new();
        assert!(!profile.has_trait(Trait::Brave));
        
        profile.add_trait(Trait::Brave);
        assert!(profile.has_trait(Trait::Brave));
        
        // Brave reduces fight cost
        assert!(profile.get_action_cost_modifier("Fight") < 1.0);
    }
}

