use ahash::AHashMap;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use world_sim_core::{AgentId, ChunkCoord, FactionId};
use world_sim_event_bus::{EventBus, WarDeclaredEvent, PeaceTreatyEvent};

/// Manages factions and political relationships
pub struct PoliticalLayer {
    factions: Arc<RwLock<AHashMap<FactionId, Faction>>>,
    territory: Arc<RwLock<TerritoryManager>>,
    event_bus: Arc<EventBus>,
}

impl PoliticalLayer {
    pub fn new(event_bus: Arc<EventBus>) -> Self {
        Self {
            factions: Arc::new(RwLock::new(AHashMap::new())),
            territory: Arc::new(RwLock::new(TerritoryManager::new())),
            event_bus,
        }
    }

    /// Create a new faction
    pub fn create_faction(&self, name: String, leader: AgentId) -> FactionId {
        let id = FactionId::new();
        let faction = Faction {
            id,
            name,
            leader,
            members: vec![leader],
            policies: Policies::default(),
            relations: HashMap::new(),
        };

        self.factions.write().insert(id, faction);
        id
    }

    /// Get faction by ID
    pub fn get_faction(&self, id: FactionId) -> Option<Faction> {
        self.factions.read().get(&id).cloned()
    }
    
    /// Get all factions
    pub fn get_all_factions(&self) -> Vec<Faction> {
        self.factions.read().values().cloned().collect()
    }

    /// Add agent to faction
    pub fn add_member(&self, faction_id: FactionId, agent_id: AgentId) {
        if let Some(faction) = self.factions.write().get_mut(&faction_id) {
            if !faction.members.contains(&agent_id) {
                faction.members.push(agent_id);
            }
        }
    }

    /// Declare war between factions
    pub async fn declare_war(&self, aggressor: FactionId, defender: FactionId, reason: String) {
        // Update relations
        {
            let mut factions = self.factions.write();
            if let Some(faction) = factions.get_mut(&aggressor) {
                faction.relations.insert(defender, FactionRelation::War);
            }
            if let Some(faction) = factions.get_mut(&defender) {
                faction.relations.insert(aggressor, FactionRelation::War);
            }
        }

        // Publish event
        self.event_bus
            .publish(&WarDeclaredEvent {
                aggressor,
                defender,
                reason,
            })
            .await;
    }

    /// Make peace between factions
    pub async fn make_peace(&self, faction_a: FactionId, faction_b: FactionId, terms: String) {
        // Update relations
        {
            let mut factions = self.factions.write();
            if let Some(faction) = factions.get_mut(&faction_a) {
                faction.relations.insert(faction_b, FactionRelation::Neutral);
            }
            if let Some(faction) = factions.get_mut(&faction_b) {
                faction.relations.insert(faction_a, FactionRelation::Neutral);
            }
        }

        // Publish event
        self.event_bus
            .publish(&PeaceTreatyEvent {
                faction_a,
                faction_b,
                terms,
            })
            .await;
    }

    /// Claim territory
    pub fn claim_territory(&self, faction_id: FactionId, chunk: ChunkCoord) {
        self.territory.write().claim(chunk, faction_id);
    }

    /// Get faction controlling a territory
    pub fn get_territory_owner(&self, chunk: ChunkCoord) -> Option<FactionId> {
        self.territory.read().get_owner(chunk)
    }
}

/// A political faction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Faction {
    pub id: FactionId,
    pub name: String,
    pub leader: AgentId,
    pub members: Vec<AgentId>,
    pub policies: Policies,
    pub relations: HashMap<FactionId, FactionRelation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Policies {
    pub tax_rate: f32,
    pub military_focus: f32,
    pub trade_openness: f32,
}

impl Default for Policies {
    fn default() -> Self {
        Self {
            tax_rate: 0.1,
            military_focus: 0.5,
            trade_openness: 0.7,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FactionRelation {
    Allied,
    Friendly,
    Neutral,
    Hostile,
    War,
}

/// Manages territorial control
pub struct TerritoryManager {
    territory_map: AHashMap<ChunkCoord, FactionId>,
}

impl TerritoryManager {
    pub fn new() -> Self {
        Self {
            territory_map: AHashMap::new(),
        }
    }

    pub fn claim(&mut self, chunk: ChunkCoord, faction_id: FactionId) {
        self.territory_map.insert(chunk, faction_id);
    }

    pub fn get_owner(&self, chunk: ChunkCoord) -> Option<FactionId> {
        self.territory_map.get(&chunk).copied()
    }

    pub fn get_all_territory(&self, faction_id: FactionId) -> Vec<ChunkCoord> {
        self.territory_map
            .iter()
            .filter(|(_, &owner)| owner == faction_id)
            .map(|(chunk, _)| *chunk)
            .collect()
    }
}

impl Default for TerritoryManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_politics() {
        let event_bus = Arc::new(EventBus::new());
        let politics = PoliticalLayer::new(event_bus);

        let leader_a = AgentId::new();
        let leader_b = AgentId::new();

        let faction_a = politics.create_faction("Kingdom A".to_string(), leader_a);
        let faction_b = politics.create_faction("Kingdom B".to_string(), leader_b);

        politics.declare_war(faction_a, faction_b, "Border dispute".to_string()).await;

        let faction = politics.get_faction(faction_a).unwrap();
        assert_eq!(faction.relations.get(&faction_b), Some(&FactionRelation::War));
    }
}

