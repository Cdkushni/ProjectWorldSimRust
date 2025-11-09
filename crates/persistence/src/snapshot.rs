use serde::{Deserialize, Serialize};
use world_sim_core::SimTime;

/// The master snapshot of the entire world state
/// This is what gets serialized for save/load
#[derive(Debug, Serialize, Deserialize)]
pub struct WorldSnapshot {
    pub version: u32,
    pub sim_time: SimTime,
    pub agents: Vec<u8>, // Placeholder for agent data
    pub world_state: Vec<u8>, // Placeholder for grid data
    pub metadata: SnapshotMetadata,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SnapshotMetadata {
    pub world_name: String,
    pub description: String,
    pub agent_count: usize,
    pub faction_count: usize,
}

impl WorldSnapshot {
    pub fn new(world_name: String) -> Self {
        Self {
            version: 1,
            sim_time: SimTime::new(),
            agents: Vec::new(),
            world_state: Vec::new(),
            metadata: SnapshotMetadata {
                world_name,
                description: String::new(),
                agent_count: 0,
                faction_count: 0,
            },
        }
    }

    /// Serialize to bytes
    pub fn to_bytes(&self) -> Result<Vec<u8>, bincode::Error> {
        bincode::serialize(self)
    }

    /// Deserialize from bytes
    pub fn from_bytes(data: &[u8]) -> Result<Self, bincode::Error> {
        bincode::deserialize(data)
    }
}

