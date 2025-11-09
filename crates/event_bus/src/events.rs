use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use world_sim_core::{AgentId, FactionId, Position, ResourceType};

/// Base trait for all events
pub trait Event: Send + Sync + std::fmt::Debug {
    fn event_type(&self) -> &'static str;
    fn as_any(&self) -> &dyn std::any::Any;
}

/// Event wrapper with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventEnvelope {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub event_type: String,
    pub source: String,
    pub payload: serde_json::Value,
}

impl EventEnvelope {
    pub fn new(event_type: String, source: String, payload: serde_json::Value) -> Self {
        Self {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            event_type,
            source,
            payload,
        }
    }
}

// ===== Economic Events =====

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceChangeEvent {
    pub resource: ResourceType,
    pub old_price: f32,
    pub new_price: f32,
    pub total_supply: u32,
    pub total_demand: u32,
}

impl Event for PriceChangeEvent {
    fn event_type(&self) -> &'static str {
        "PriceChange"
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeExecutedEvent {
    pub seller_id: AgentId,
    pub buyer_id: AgentId,
    pub resource: ResourceType,
    pub quantity: u32,
    pub price: f32,
    pub location: Position,
}

impl Event for TradeExecutedEvent {
    fn event_type(&self) -> &'static str {
        "TradeExecuted"
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

// ===== Political Events =====

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WarDeclaredEvent {
    pub aggressor: FactionId,
    pub defender: FactionId,
    pub reason: String,
}

impl Event for WarDeclaredEvent {
    fn event_type(&self) -> &'static str {
        "WarDeclared"
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeaceTreatyEvent {
    pub faction_a: FactionId,
    pub faction_b: FactionId,
    pub terms: String,
}

impl Event for PeaceTreatyEvent {
    fn event_type(&self) -> &'static str {
        "PeaceTreaty"
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

// ===== Environmental Events =====

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlightStartedEvent {
    pub center: Position,
    pub radius: f32,
    pub affected_resource: ResourceType,
}

impl Event for BlightStartedEvent {
    fn event_type(&self) -> &'static str {
        "BlightStarted"
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DroughtStartedEvent {
    pub region: String,
    pub severity: f32,
    pub expected_duration_days: u32,
}

impl Event for DroughtStartedEvent {
    fn event_type(&self) -> &'static str {
        "DroughtStarted"
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeasonChangeEvent {
    pub old_season: Season,
    pub new_season: Season,
}

impl Event for SeasonChangeEvent {
    fn event_type(&self) -> &'static str {
        "SeasonChange"
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Season {
    Spring,
    Summer,
    Autumn,
    Winter,
}

// ===== Agent Events =====

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentDiedEvent {
    pub agent_id: AgentId,
    pub cause: String,
    pub location: Position,
}

impl Event for AgentDiedEvent {
    fn event_type(&self) -> &'static str {
        "AgentDied"
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentBornEvent {
    pub agent_id: AgentId,
    pub parent_ids: Vec<AgentId>,
    pub location: Position,
}

impl Event for AgentBornEvent {
    fn event_type(&self) -> &'static str {
        "AgentBorn"
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

// ===== Dungeon Master Events =====

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DungeonMasterEvent {
    pub event_name: String,
    pub description: String,
    pub impact: String,
}

impl Event for DungeonMasterEvent {
    fn event_type(&self) -> &'static str {
        "DungeonMaster"
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

