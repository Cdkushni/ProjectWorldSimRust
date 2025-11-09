use std::collections::{BinaryHeap, HashMap};
use std::cmp::Ordering;
use std::sync::Arc;
use world_sim_core::GridCoord;
use crate::GridLayer;

/// A* pathfinding node
#[derive(Clone, Eq, PartialEq)]
struct Node {
    coord: GridCoord,
    g_cost: i32, // Cost from start
    h_cost: i32, // Heuristic to goal
    parent: Option<GridCoord>,
}

impl Node {
    fn f_cost(&self) -> i32 {
        self.g_cost + self.h_cost
    }
}

impl Ord for Node {
    fn cmp(&self, other: &Self) -> Ordering {
        other.f_cost().cmp(&self.f_cost())
    }
}

impl PartialOrd for Node {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Simple A* pathfinding
pub fn find_path(grid: &GridLayer, start: GridCoord, goal: GridCoord, max_iterations: usize) -> Option<Vec<GridCoord>> {
    let mut open_set = BinaryHeap::new();
    let mut closed_set = HashMap::new();
    
    open_set.push(Node {
        coord: start,
        g_cost: 0,
        h_cost: start.manhattan_distance(&goal),
        parent: None,
    });
    
    let mut iterations = 0;
    
    while let Some(current) = open_set.pop() {
        iterations += 1;
        if iterations > max_iterations {
            return None; // Timeout
        }
        
        if current.coord == goal {
            // Reconstruct path
            let mut path = vec![current.coord];
            let mut current_coord = current.coord;
            
            while let Some(parent) = closed_set.get(&current_coord) {
                path.push(*parent);
                current_coord = *parent;
                if current_coord == start {
                    break;
                }
            }
            
            path.reverse();
            return Some(path);
        }
        
        // Check neighbors
        let neighbors = get_neighbors(current.coord);
        for neighbor in neighbors {
            if !grid.is_walkable(neighbor) {
                continue;
            }
            
            if closed_set.contains_key(&neighbor) {
                continue;
            }
            
            let g_cost = current.g_cost + 10; // Cost to move to neighbor
            let h_cost = neighbor.manhattan_distance(&goal);
            
            open_set.push(Node {
                coord: neighbor,
                g_cost,
                h_cost,
                parent: Some(current.coord),
            });
        }
        
        closed_set.insert(current.coord, current.parent.unwrap_or(start));
    }
    
    None // No path found
}

/// Get neighboring coordinates (6-directional for 3D)
fn get_neighbors(coord: GridCoord) -> Vec<GridCoord> {
    vec![
        GridCoord::new(coord.x + 1, coord.y, coord.z),
        GridCoord::new(coord.x - 1, coord.y, coord.z),
        GridCoord::new(coord.x, coord.y + 1, coord.z),
        GridCoord::new(coord.x, coord.y - 1, coord.z),
        GridCoord::new(coord.x, coord.y, coord.z + 1),
        GridCoord::new(coord.x, coord.y, coord.z - 1),
    ]
}

/// Hierarchical pathfinding structure (HPA*)
pub struct HierarchicalPathfinding {
    #[allow(dead_code)]
    chunk_graph: petgraph::Graph<world_sim_core::ChunkCoord, f32>,
    grid: Arc<GridLayer>,
}

impl HierarchicalPathfinding {
    pub fn new(grid: Arc<GridLayer>) -> Self {
        Self {
            chunk_graph: petgraph::Graph::new(),
            grid,
        }
    }
    
    /// Build high-level chunk connectivity graph
    pub fn build_abstract_graph(&mut self) {
        // TODO: Implement chunk-level pathfinding for optimization
        // For now, this is a placeholder for the HPA* optimization
    }
    
    /// Find a high-level path between chunks, then refine locally
    pub fn find_hierarchical_path(&self, start: GridCoord, goal: GridCoord) -> Option<Vec<GridCoord>> {
        // For now, just use regular A* (optimization can be added later)
        find_path(&self.grid, start, goal, 1000)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use world_sim_core::BlockType;

    #[test]
    fn test_simple_path() {
        let grid = GridLayer::new();
        
        // Create a simple walkable area
        for x in 0..10 {
            for z in 0..10 {
                grid.set_block(GridCoord::new(x, 0, z), BlockType::Grass);
                grid.set_block(GridCoord::new(x, 1, z), BlockType::Air);
            }
        }
        
        let start = GridCoord::new(0, 1, 0);
        let goal = GridCoord::new(5, 1, 5);
        
        let path = find_path(&grid, start, goal, 1000);
        assert!(path.is_some());
    }
}

