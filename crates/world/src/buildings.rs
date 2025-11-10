use ahash::AHashMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;
use world_sim_core::{AgentId, FactionId, Position, ResourceType};

/// A physical building in the world
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Building {
    pub id: Uuid,
    pub building_type: BuildingType,
    pub position: Position,
    pub name: String,
    pub owner: BuildingOwner,
    pub construction_progress: f32, // 0.0 to 1.0
    pub health: f32,
    pub storage: ResourceStorage,
    
    // Resource-based construction
    pub required_resources: HashMap<ResourceType, u32>, // Total resources needed
    pub current_resources: HashMap<ResourceType, u32>,  // Resources delivered so far
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BuildingType {
    Warehouse,      // Stores large quantities of resources
    Market,         // Already handled by market system
    Barracks,       // Military training and housing
    Workshop,       // Crafting and production
    Farm,           // Food production
    Mine,           // Resource extraction
    NobleEstate,    // Housing for nobles
    Church,         // Cleric activities
    Tavern,         // Social gathering
    Walls,          // Defensive structures
    PeasantHouse,   // Small dwelling for peasants
    FarmingShed,    // Storage for farm equipment
}

impl BuildingType {
    /// Get the resource requirements for constructing this building type
    pub fn required_resources(&self) -> HashMap<ResourceType, u32> {
        let mut requirements = HashMap::new();
        
        match self {
            BuildingType::Warehouse => {
                requirements.insert(ResourceType::Wood, 100);
                requirements.insert(ResourceType::Stone, 50);
                requirements.insert(ResourceType::Iron, 20);
            },
            BuildingType::Barracks => {
                requirements.insert(ResourceType::Wood, 80);
                requirements.insert(ResourceType::Stone, 60);
                requirements.insert(ResourceType::Iron, 30);
            },
            BuildingType::Workshop => {
                requirements.insert(ResourceType::Wood, 60);
                requirements.insert(ResourceType::Stone, 40);
                requirements.insert(ResourceType::Iron, 15);
            },
            BuildingType::Farm => {
                requirements.insert(ResourceType::Wood, 40);
                requirements.insert(ResourceType::Stone, 20);
                requirements.insert(ResourceType::Iron, 5);
            },
            BuildingType::Mine => {
                requirements.insert(ResourceType::Wood, 30);
                requirements.insert(ResourceType::Stone, 50);
                requirements.insert(ResourceType::Iron, 25);
            },
            BuildingType::NobleEstate => {
                requirements.insert(ResourceType::Wood, 150);
                requirements.insert(ResourceType::Stone, 100);
                requirements.insert(ResourceType::Iron, 40);
            },
            BuildingType::Church => {
                requirements.insert(ResourceType::Wood, 70);
                requirements.insert(ResourceType::Stone, 80);
                requirements.insert(ResourceType::Iron, 10);
            },
            BuildingType::Tavern => {
                requirements.insert(ResourceType::Wood, 50);
                requirements.insert(ResourceType::Stone, 30);
                requirements.insert(ResourceType::Iron, 5);
            },
            BuildingType::Walls => {
                requirements.insert(ResourceType::Wood, 20);
                requirements.insert(ResourceType::Stone, 200);
                requirements.insert(ResourceType::Iron, 50);
            },
            BuildingType::PeasantHouse => {
                requirements.insert(ResourceType::Wood, 30);
                requirements.insert(ResourceType::Stone, 10);
            },
            BuildingType::FarmingShed => {
                requirements.insert(ResourceType::Wood, 20);
                requirements.insert(ResourceType::Stone, 5);
            },
            BuildingType::Market => {
                requirements.insert(ResourceType::Wood, 90);
                requirements.insert(ResourceType::Stone, 70);
                requirements.insert(ResourceType::Iron, 15);
            },
        }
        
        requirements
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BuildingOwner {
    Faction(FactionId),
    Agent(AgentId),
    Public, // Shared/neutral
}

/// Resource storage within a building
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceStorage {
    pub capacity: u32,
    pub inventory: HashMap<ResourceType, u32>,
}

impl ResourceStorage {
    pub fn new(capacity: u32) -> Self {
        Self {
            capacity,
            inventory: HashMap::new(),
        }
    }
    
    pub fn current_usage(&self) -> u32 {
        self.inventory.values().sum()
    }
    
    pub fn available_space(&self) -> u32 {
        self.capacity.saturating_sub(self.current_usage())
    }
    
    pub fn can_store(&self, resource: ResourceType, quantity: u32) -> bool {
        self.available_space() >= quantity
    }
    
    pub fn store(&mut self, resource: ResourceType, quantity: u32) -> bool {
        if self.can_store(resource, quantity) {
            *self.inventory.entry(resource).or_insert(0) += quantity;
            true
        } else {
            false
        }
    }
    
    pub fn retrieve(&mut self, resource: ResourceType, quantity: u32) -> u32 {
        let available = self.inventory.get(&resource).copied().unwrap_or(0);
        let retrieved = available.min(quantity);
        
        if retrieved > 0 {
            *self.inventory.get_mut(&resource).unwrap() -= retrieved;
        }
        
        retrieved
    }
    
    pub fn get_quantity(&self, resource: ResourceType) -> u32 {
        self.inventory.get(&resource).copied().unwrap_or(0)
    }
}

impl Building {
    pub fn new(
        building_type: BuildingType,
        position: Position,
        name: String,
        owner: BuildingOwner,
    ) -> Self {
        let capacity = match building_type {
            BuildingType::Warehouse => 1000,
            BuildingType::Workshop => 200,
            BuildingType::Farm => 300,
            BuildingType::Mine => 500,
            BuildingType::Barracks => 100,
            BuildingType::NobleEstate => 200,
            BuildingType::Church => 50,
            BuildingType::Tavern => 100,
            _ => 0,
        };
        
        let required_resources = building_type.required_resources();
        
        Self {
            id: Uuid::new_v4(),
            building_type,
            position,
            name,
            owner,
            construction_progress: 0.0,
            health: 100.0,
            storage: ResourceStorage::new(capacity),
            required_resources: required_resources.clone(),
            current_resources: HashMap::new(), // Start empty
        }
    }
    
    /// Check if building has enough resources for construction
    pub fn has_sufficient_resources(&self) -> bool {
        for (resource_type, required) in &self.required_resources {
            let current = self.current_resources.get(resource_type).copied().unwrap_or(0);
            if current < *required {
                return false;
            }
        }
        true
    }
    
    /// Get percentage of resources delivered
    pub fn resource_completion_percent(&self) -> f32 {
        if self.required_resources.is_empty() {
            return 1.0;
        }
        
        let mut total_required = 0;
        let mut total_delivered = 0;
        
        for (resource_type, required) in &self.required_resources {
            total_required += required;
            total_delivered += self.current_resources.get(resource_type).copied().unwrap_or(0);
        }
        
        if total_required == 0 {
            1.0
        } else {
            total_delivered as f32 / total_required as f32
        }
    }
    
    /// Add resources to the building (from builder delivery)
    pub fn add_resources(&mut self, resource_type: ResourceType, quantity: u32) {
        *self.current_resources.entry(resource_type).or_insert(0) += quantity;
    }
    
    /// Get remaining resources needed
    pub fn remaining_resources(&self) -> HashMap<ResourceType, u32> {
        let mut remaining = HashMap::new();
        
        for (resource_type, required) in &self.required_resources {
            let current = self.current_resources.get(resource_type).copied().unwrap_or(0);
            if current < *required {
                remaining.insert(*resource_type, required - current);
            }
        }
        
        remaining
    }
    
    pub fn is_complete(&self) -> bool {
        self.construction_progress >= 1.0
    }
    
    pub fn add_construction_progress(&mut self, amount: f32) {
        self.construction_progress = (self.construction_progress + amount).min(1.0);
    }
    
    /// Construct with resource consumption (returns true if construction occurred)
    pub fn construct_with_resources(&mut self, progress_amount: f32) -> bool {
        // Check if we have enough resources for this tick
        let mut can_construct = true;
        
        for (resource_type, required) in &self.required_resources {
            let current = self.current_resources.get(resource_type).copied().unwrap_or(0);
            let needed_for_tick = (*required as f32 * progress_amount).ceil() as u32;
            
            if current < needed_for_tick {
                can_construct = false;
                break;
            }
        }
        
        if can_construct {
            // Consume resources proportional to progress
            for (resource_type, required) in &self.required_resources {
                let consumption = (*required as f32 * progress_amount).ceil() as u32;
                if let Some(current) = self.current_resources.get_mut(resource_type) {
                    *current = current.saturating_sub(consumption);
                }
            }
            
            // Increase construction progress
            self.construction_progress = (self.construction_progress + progress_amount).min(1.0);
            true
        } else {
            false
        }
    }
    
    pub fn damage(&mut self, amount: f32) {
        self.health = (self.health - amount).max(0.0);
    }
    
    pub fn is_destroyed(&self) -> bool {
        self.health <= 0.0
    }
}

/// Manager for all buildings in the world
pub struct BuildingManager {
    buildings: AHashMap<Uuid, Building>,
}

impl BuildingManager {
    pub fn new() -> Self {
        Self {
            buildings: AHashMap::new(),
        }
    }
    
    pub fn add_building(&mut self, building: Building) -> Uuid {
        let id = building.id;
        self.buildings.insert(id, building);
        id
    }
    
    pub fn get_building(&self, id: Uuid) -> Option<&Building> {
        self.buildings.get(&id)
    }
    
    pub fn get_building_mut(&mut self, id: Uuid) -> Option<&mut Building> {
        self.buildings.get_mut(&id)
    }
    
    pub fn get_all_buildings(&self) -> Vec<&Building> {
        self.buildings.values().collect()
    }
    
    pub fn find_nearest_building(
        &self,
        position: &Position,
        building_type: Option<BuildingType>,
        only_complete: bool,
    ) -> Option<&Building> {
        self.buildings
            .values()
            .filter(|b| {
                (building_type.is_none() || building_type == Some(b.building_type))
                    && (!only_complete || b.is_complete())
            })
            .min_by(|a, b| {
                let dist_a = a.position.distance_to(position);
                let dist_b = b.position.distance_to(position);
                dist_a.partial_cmp(&dist_b).unwrap_or(std::cmp::Ordering::Equal)
            })
    }
    
    pub fn remove_destroyed_buildings(&mut self) -> Vec<Uuid> {
        let destroyed: Vec<Uuid> = self
            .buildings
            .iter()
            .filter(|(_, b)| b.is_destroyed())
            .map(|(id, _)| *id)
            .collect();
        
        for id in &destroyed {
            self.buildings.remove(id);
        }
        
        destroyed
    }
}

impl Default for BuildingManager {
    fn default() -> Self {
        Self::new()
    }
}

