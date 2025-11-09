use ahash::AHashMap;
use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use world_sim_core::AgentId;

/// Manages all agent-to-agent relationships
pub struct SocialLayer {
    relationships: Arc<RwLock<RelationshipManager>>,
    memories: Arc<RwLock<MemoryManager>>,
}

impl SocialLayer {
    pub fn new() -> Self {
        Self {
            relationships: Arc::new(RwLock::new(RelationshipManager::new())),
            memories: Arc::new(RwLock::new(MemoryManager::new())),
        }
    }

    /// Get relationship between two agents
    pub fn get_relationship(&self, agent_a: AgentId, agent_b: AgentId) -> Option<Relationship> {
        self.relationships.read().get(agent_a, agent_b)
    }

    /// Set or update relationship
    pub fn set_relationship(&self, agent_a: AgentId, agent_b: AgentId, relationship: Relationship) {
        self.relationships.write().set(agent_a, agent_b, relationship);
    }

    /// Modify affinity between two agents
    pub fn modify_affinity(&self, agent_a: AgentId, agent_b: AgentId, delta: f32) {
        self.relationships.write().modify_affinity(agent_a, agent_b, delta);
    }

    /// Add a memory to an agent
    pub fn add_memory(&self, agent_id: AgentId, memory: MemoryFact) {
        self.memories.write().add(agent_id, memory);
    }

    /// Get all memories for an agent
    pub fn get_memories(&self, agent_id: AgentId) -> Vec<MemoryFact> {
        self.memories.read().get(agent_id)
    }

    /// Process agent death - decay relationships
    pub fn on_agent_died(&self, agent_id: AgentId) {
        self.relationships.write().decay_relationships_with(agent_id, 0.5);
    }
}

impl Default for SocialLayer {
    fn default() -> Self {
        Self::new()
    }
}

/// Manages relationships between agents
pub struct RelationshipManager {
    relationships: AHashMap<AgentId, AHashMap<AgentId, Relationship>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relationship {
    pub affinity: f32,  // -100 to +100
    pub trust: f32,     // 0 to 100
    pub last_interaction: DateTime<Utc>,
}

impl Relationship {
    pub fn new() -> Self {
        Self {
            affinity: 0.0,
            trust: 50.0,
            last_interaction: Utc::now(),
        }
    }

    pub fn is_friendly(&self) -> bool {
        self.affinity > 50.0
    }

    pub fn is_hostile(&self) -> bool {
        self.affinity < -50.0
    }
}

impl Default for Relationship {
    fn default() -> Self {
        Self::new()
    }
}

impl RelationshipManager {
    pub fn new() -> Self {
        Self {
            relationships: AHashMap::new(),
        }
    }

    pub fn get(&self, agent_a: AgentId, agent_b: AgentId) -> Option<Relationship> {
        self.relationships
            .get(&agent_a)
            .and_then(|map| map.get(&agent_b))
            .cloned()
    }

    pub fn set(&mut self, agent_a: AgentId, agent_b: AgentId, relationship: Relationship) {
        self.relationships
            .entry(agent_a)
            .or_insert_with(AHashMap::new)
            .insert(agent_b, relationship);
    }

    pub fn modify_affinity(&mut self, agent_a: AgentId, agent_b: AgentId, delta: f32) {
        let relationship = self
            .relationships
            .entry(agent_a)
            .or_insert_with(AHashMap::new)
            .entry(agent_b)
            .or_insert_with(Relationship::new);

        relationship.affinity += delta;
        relationship.affinity = relationship.affinity.clamp(-100.0, 100.0);
        relationship.last_interaction = Utc::now();
    }

    pub fn decay_relationships_with(&mut self, agent_id: AgentId, decay_factor: f32) {
        // Decay all relationships involving this agent
        for relationships in self.relationships.values_mut() {
            if let Some(rel) = relationships.get_mut(&agent_id) {
                rel.affinity *= decay_factor;
            }
        }
    }
}

impl Default for RelationshipManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Manages agent memories
pub struct MemoryManager {
    memories: AHashMap<AgentId, Vec<MemoryFact>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryFact {
    pub fact: String,
    pub timestamp: DateTime<Utc>,
    pub source: MemorySource,
    pub importance: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MemorySource {
    Witnessed,
    Told(AgentId),
    Inferred,
    Fabricated, // False memory (from admin API)
}

impl MemoryManager {
    pub fn new() -> Self {
        Self {
            memories: AHashMap::new(),
        }
    }

    pub fn add(&mut self, agent_id: AgentId, memory: MemoryFact) {
        self.memories
            .entry(agent_id)
            .or_insert_with(Vec::new)
            .push(memory);
    }

    pub fn get(&self, agent_id: AgentId) -> Vec<MemoryFact> {
        self.memories.get(&agent_id).cloned().unwrap_or_default()
    }

    /// Get memories filtered by importance
    pub fn get_important_memories(&self, agent_id: AgentId, min_importance: f32) -> Vec<MemoryFact> {
        self.memories
            .get(&agent_id)
            .map(|memories| {
                memories
                    .iter()
                    .filter(|m| m.importance >= min_importance)
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }
}

impl Default for MemoryManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_relationships() {
        let mut manager = RelationshipManager::new();
        let agent_a = AgentId::new();
        let agent_b = AgentId::new();

        manager.modify_affinity(agent_a, agent_b, 30.0);
        let rel = manager.get(agent_a, agent_b).unwrap();
        assert_eq!(rel.affinity, 30.0);
    }
}

