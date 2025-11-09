use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use world_sim_core::Skill;

/// Tracks learned skills and knowledge for an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillDatabase {
    skills: HashMap<Skill, SkillData>,
    knowledge: Vec<Knowledge>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillData {
    pub level: f32,
    pub experience: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Knowledge {
    pub id: String,
    pub knowledge_type: KnowledgeType,
    pub acquired_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum KnowledgeType {
    Recipe(String),
    Secret(String),
    Location(String),
    Rumor(String),
}

impl SkillDatabase {
    pub fn new() -> Self {
        Self {
            skills: HashMap::new(),
            knowledge: Vec::new(),
        }
    }

    /// Get skill level (0.0 if not learned)
    pub fn get_level(&self, skill: Skill) -> f32 {
        self.skills.get(&skill).map(|s| s.level).unwrap_or(0.0)
    }

    /// Add experience to a skill
    pub fn add_experience(&mut self, skill: Skill, amount: f32) {
        let data = self.skills.entry(skill).or_insert(SkillData {
            level: 0.0,
            experience: 0.0,
        });
        
        data.experience += amount;
        
        // Level up formula: level = sqrt(experience / 10)
        data.level = (data.experience / 10.0).sqrt();
    }

    /// Check if agent knows something
    pub fn has_knowledge(&self, knowledge_id: &str) -> bool {
        self.knowledge.iter().any(|k| k.id == knowledge_id)
    }

    /// Learn new knowledge
    pub fn learn_knowledge(&mut self, knowledge: Knowledge) {
        if !self.has_knowledge(&knowledge.id) {
            self.knowledge.push(knowledge);
        }
    }

    /// Get all knowledge of a specific type
    pub fn get_knowledge_by_type(&self, knowledge_type: &str) -> Vec<&Knowledge> {
        self.knowledge
            .iter()
            .filter(|k| match &k.knowledge_type {
                KnowledgeType::Recipe(_) => knowledge_type == "Recipe",
                KnowledgeType::Secret(_) => knowledge_type == "Secret",
                KnowledgeType::Location(_) => knowledge_type == "Location",
                KnowledgeType::Rumor(_) => knowledge_type == "Rumor",
            })
            .collect()
    }
}

impl Default for SkillDatabase {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skill_progression() {
        let mut db = SkillDatabase::new();
        assert_eq!(db.get_level(Skill::Mining), 0.0);
        
        db.add_experience(Skill::Mining, 100.0);
        assert!(db.get_level(Skill::Mining) > 0.0);
        
        db.add_experience(Skill::Mining, 400.0);
        assert!(db.get_level(Skill::Mining) > 5.0);
    }
}

