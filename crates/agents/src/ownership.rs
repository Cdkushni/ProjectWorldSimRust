use ahash::AHashMap;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use world_sim_core::{AgentId, GridCoord, ItemId};

/// Global registry of who owns what
pub struct GlobalOwnershipRegistry {
    ownership: Arc<RwLock<AHashMap<ItemId, AgentId>>>,
}

impl GlobalOwnershipRegistry {
    pub fn new() -> Self {
        Self {
            ownership: Arc::new(RwLock::new(AHashMap::new())),
        }
    }

    /// Register an item as owned by an agent
    pub fn set_owner(&self, item_id: ItemId, owner_id: AgentId) {
        self.ownership.write().insert(item_id, owner_id);
    }

    /// Get the owner of an item
    pub fn get_owner(&self, item_id: ItemId) -> Option<AgentId> {
        self.ownership.read().get(&item_id).copied()
    }

    /// Transfer ownership
    pub fn transfer(&self, item_id: ItemId, new_owner: AgentId) {
        self.set_owner(item_id, new_owner);
    }

    /// Remove ownership (item destroyed or lost)
    pub fn remove(&self, item_id: ItemId) {
        self.ownership.write().remove(&item_id);
    }

    /// Get all items owned by an agent
    pub fn get_items_owned_by(&self, owner_id: AgentId) -> Vec<ItemId> {
        self.ownership
            .read()
            .iter()
            .filter(|(_, &owner)| owner == owner_id)
            .map(|(item_id, _)| *item_id)
            .collect()
    }
}

impl Default for GlobalOwnershipRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// An agent's "domain" - their personal space and social network
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentDomain {
    pub home_location: Option<GridCoord>,
    pub sleep_location: Option<GridCoord>,
    pub work_location: Option<GridCoord>,
    pub safe_zones: Vec<GridCoord>,
    pub social_cache: SocialCache,
}

/// Quick-reference social network
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocialCache {
    pub inner_circle: Vec<AgentId>,  // Family, close friends
    pub allies: Vec<AgentId>,
    pub superiors: Vec<AgentId>,     // Boss, king, etc.
    pub rivals: Vec<AgentId>,
    pub enemies: Vec<AgentId>,
}

impl AgentDomain {
    pub fn new() -> Self {
        Self {
            home_location: None,
            sleep_location: None,
            work_location: None,
            safe_zones: Vec::new(),
            social_cache: SocialCache {
                inner_circle: Vec::new(),
                allies: Vec::new(),
                superiors: Vec::new(),
                rivals: Vec::new(),
                enemies: Vec::new(),
            },
        }
    }

    /// Set home location
    pub fn set_home(&mut self, location: GridCoord) {
        self.home_location = Some(location);
        if self.sleep_location.is_none() {
            self.sleep_location = Some(location);
        }
    }

    /// Add to social circle
    pub fn add_to_inner_circle(&mut self, agent_id: AgentId) {
        if !self.social_cache.inner_circle.contains(&agent_id) {
            self.social_cache.inner_circle.push(agent_id);
        }
    }

    /// Add enemy
    pub fn add_enemy(&mut self, agent_id: AgentId) {
        if !self.social_cache.enemies.contains(&agent_id) {
            self.social_cache.enemies.push(agent_id);
        }
    }

    /// Check if agent is an enemy
    pub fn is_enemy(&self, agent_id: AgentId) -> bool {
        self.social_cache.enemies.contains(&agent_id)
    }
}

impl Default for AgentDomain {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ownership() {
        let registry = GlobalOwnershipRegistry::new();
        let agent = AgentId::new();
        let item = ItemId::new();
        
        registry.set_owner(item, agent);
        assert_eq!(registry.get_owner(item), Some(agent));
        
        let items = registry.get_items_owned_by(agent);
        assert_eq!(items.len(), 1);
    }
}

