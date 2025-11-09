use serde::{Deserialize, Serialize};
use world_sim_core::{GridCoord, Position};
use parking_lot::RwLock;
use std::sync::Arc;

/// Types of harvestable resources in the world
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResourceNodeType {
    Tree,
    Rock,
    Farm,
    IronDeposit,
}

/// A resource node in the world
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceNode {
    pub id: uuid::Uuid,
    pub resource_type: ResourceNodeType,
    pub position: Position,
    pub quantity: u32,
}

impl ResourceNode {
    pub fn new(resource_type: ResourceNodeType, position: Position, quantity: u32) -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            resource_type,
            position,
            quantity,
        }
    }
}

/// Manages all resource nodes in the world
pub struct ResourceManager {
    nodes: Arc<RwLock<Vec<ResourceNode>>>,
}

impl ResourceManager {
    pub fn new() -> Self {
        Self {
            nodes: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Add a resource node
    pub fn add_node(&self, node: ResourceNode) {
        self.nodes.write().push(node);
    }

    /// Get all nodes
    pub fn get_nodes(&self) -> Vec<ResourceNode> {
        self.nodes.read().clone()
    }

    /// Get nodes of a specific type
    pub fn get_nodes_by_type(&self, resource_type: ResourceNodeType) -> Vec<ResourceNode> {
        self.nodes
            .read()
            .iter()
            .filter(|n| n.resource_type == resource_type)
            .cloned()
            .collect()
    }

    /// Find nearest node of a type
    pub fn find_nearest(&self, pos: Position, resource_type: ResourceNodeType) -> Option<ResourceNode> {
        let nodes = self.nodes.read();
        nodes
            .iter()
            .filter(|n| n.resource_type == resource_type && n.quantity > 0)
            .min_by(|a, b| {
                let dist_a = pos.distance_to(&a.position);
                let dist_b = pos.distance_to(&b.position);
                dist_a.partial_cmp(&dist_b).unwrap()
            })
            .cloned()
    }

    /// Harvest from a node
    pub fn harvest(&self, node_id: uuid::Uuid, amount: u32) -> Option<u32> {
        let mut nodes = self.nodes.write();
        if let Some(node) = nodes.iter_mut().find(|n| n.id == node_id) {
            let harvested = amount.min(node.quantity);
            node.quantity -= harvested;
            Some(harvested)
        } else {
            None
        }
    }

    /// Generate random resource nodes
    pub fn generate_random_nodes(&self, count: usize, world_size: f32) {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        
        let mut nodes = self.nodes.write();
        
        // Clear existing
        nodes.clear();
        
        // Generate new nodes
        for _ in 0..count {
            let resource_type = match rng.gen_range(0..4) {
                0 => ResourceNodeType::Tree,
                1 => ResourceNodeType::Rock,
                2 => ResourceNodeType::Farm,
                _ => ResourceNodeType::IronDeposit,
            };
            
            let position = Position::new(
                rng.gen_range(-world_size..world_size),
                1.0,
                rng.gen_range(-world_size..world_size),
            );
            
            let quantity = rng.gen_range(50..200);
            
            nodes.push(ResourceNode::new(resource_type, position, quantity));
        }
    }
}

impl Default for ResourceManager {
    fn default() -> Self {
        Self::new()
    }
}

