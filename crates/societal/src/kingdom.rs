use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;
use world_sim_core::{AgentId, FactionId, Position};
use world_sim_world::BuildingType;

/// Strategic goals for a kingdom/faction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KingdomGoal {
    DefendTerritory,       // Build walls, barracks, prepare defenses
    ExpandResources,       // Build mines, farms, resource infrastructure
    PrepareForWar,         // Build barracks, weapons, military readiness
    GrowPopulation,        // Build houses, farms, support population
    ImproveInfrastructure, // Build markets, workshops, taverns
    Consolidate,           // Maintain current state, no major projects
}

/// A kingdom or organized faction with strategic direction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Kingdom {
    pub id: Uuid,
    pub faction_id: Option<FactionId>,
    pub king_id: AgentId,
    pub nobles: Vec<AgentId>,
    pub current_goal: KingdomGoal,
    pub goal_set_time: f64,      // When goal was set
    pub goal_priority: f32,      // 0.0-1.0, how urgent
    pub territory_center: Position,
    pub territory_radius: f32,
}

impl Kingdom {
    pub fn new(king_id: AgentId, territory_center: Position) -> Self {
        Self {
            id: Uuid::new_v4(),
            faction_id: None,
            king_id,
            nobles: Vec::new(),
            current_goal: KingdomGoal::Consolidate,
            goal_set_time: 0.0,
            goal_priority: 0.5,
            territory_center,
            territory_radius: 50.0,
        }
    }
    
    pub fn add_noble(&mut self, noble_id: AgentId) {
        if !self.nobles.contains(&noble_id) {
            self.nobles.push(noble_id);
        }
    }
    
    pub fn set_goal(&mut self, goal: KingdomGoal, priority: f32, current_time: f64) {
        self.current_goal = goal;
        self.goal_priority = priority.clamp(0.0, 1.0);
        self.goal_set_time = current_time;
    }
}

/// An order from a noble to construct a building
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NobleOrder {
    pub id: Uuid,
    pub noble_id: AgentId,
    pub building_type: BuildingType,
    pub location: Position,
    pub priority: f32,           // 0.0-1.0
    pub assigned_builders: Vec<AgentId>,
    pub building_id: Option<Uuid>, // Set when building is created
    pub status: OrderStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderStatus {
    Pending,      // Order issued, not started
    InProgress,   // Building created, under construction
    Completed,    // Building finished
    Cancelled,    // Order cancelled
}

impl NobleOrder {
    pub fn new(noble_id: AgentId, building_type: BuildingType, location: Position, priority: f32) -> Self {
        Self {
            id: Uuid::new_v4(),
            noble_id,
            building_type,
            location,
            priority: priority.clamp(0.0, 1.0),
            assigned_builders: Vec::new(),
            building_id: None,
            status: OrderStatus::Pending,
        }
    }
}

/// Manager for kingdom-level strategic planning
pub struct KingdomManager {
    kingdoms: HashMap<Uuid, Kingdom>,
    noble_orders: HashMap<Uuid, NobleOrder>,
}

impl KingdomManager {
    pub fn new() -> Self {
        Self {
            kingdoms: HashMap::new(),
            noble_orders: HashMap::new(),
        }
    }
    
    pub fn create_kingdom(&mut self, king_id: AgentId, territory_center: Position) -> Uuid {
        let kingdom = Kingdom::new(king_id, territory_center);
        let id = kingdom.id;
        self.kingdoms.insert(id, kingdom);
        id
    }
    
    pub fn get_kingdom(&self, id: Uuid) -> Option<&Kingdom> {
        self.kingdoms.get(&id)
    }
    
    pub fn get_kingdom_mut(&mut self, id: Uuid) -> Option<&mut Kingdom> {
        self.kingdoms.get_mut(&id)
    }
    
    pub fn get_kingdom_by_king(&self, king_id: AgentId) -> Option<&Kingdom> {
        self.kingdoms.values().find(|k| k.king_id == king_id)
    }
    
    pub fn get_kingdom_by_king_mut(&mut self, king_id: AgentId) -> Option<&mut Kingdom> {
        self.kingdoms.values_mut().find(|k| k.king_id == king_id)
    }
    
    pub fn add_noble_order(&mut self, order: NobleOrder) -> Uuid {
        let id = order.id;
        self.noble_orders.insert(id, order);
        id
    }
    
    pub fn get_order(&self, id: Uuid) -> Option<&NobleOrder> {
        self.noble_orders.get(&id)
    }
    
    pub fn get_order_mut(&mut self, id: Uuid) -> Option<&mut NobleOrder> {
        self.noble_orders.get_mut(&id)
    }
    
    pub fn get_pending_orders(&self) -> Vec<&NobleOrder> {
        self.noble_orders.values()
            .filter(|o| matches!(o.status, OrderStatus::Pending))
            .collect()
    }
    
    pub fn get_orders_by_noble(&self, noble_id: AgentId) -> Vec<&NobleOrder> {
        self.noble_orders.values()
            .filter(|o| o.noble_id == noble_id)
            .collect()
    }
    
    pub fn complete_order(&mut self, order_id: Uuid) {
        if let Some(order) = self.noble_orders.get_mut(&order_id) {
            order.status = OrderStatus::Completed;
        }
    }
}

