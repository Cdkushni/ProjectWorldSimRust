use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Strongly-typed ID wrappers to prevent mixing different entity types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AgentId(pub Uuid);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ItemId(pub Uuid);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FactionId(pub Uuid);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChunkId(pub i32, pub i32, pub i32);

impl AgentId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl ItemId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl FactionId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for AgentId {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for ItemId {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for FactionId {
    fn default() -> Self {
        Self::new()
    }
}

