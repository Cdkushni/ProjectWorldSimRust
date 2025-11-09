use ahash::AHashMap;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;
use world_sim_core::{BlockType, ChunkCoord, GridCoord, Position};

/// Size of each chunk (32x32x32 blocks)
pub const CHUNK_SIZE: i32 = 32;

/// A chunk of blocks in the world
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chunk {
    pub coord: ChunkCoord,
    pub blocks: Vec<BlockType>, // Flattened 3D array [CHUNK_SIZE³]
}

impl Chunk {
    pub fn new(coord: ChunkCoord) -> Self {
        let size = (CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE) as usize;
        Self {
            coord,
            blocks: vec![BlockType::Air; size],
        }
    }

    /// Get the index in the flattened array
    fn index(x: i32, y: i32, z: i32) -> usize {
        ((z * CHUNK_SIZE * CHUNK_SIZE) + (y * CHUNK_SIZE) + x) as usize
    }

    /// Get block at local chunk coordinates (0-31)
    pub fn get(&self, x: i32, y: i32, z: i32) -> BlockType {
        if x < 0 || x >= CHUNK_SIZE || y < 0 || y >= CHUNK_SIZE || z < 0 || z >= CHUNK_SIZE {
            return BlockType::Air;
        }
        self.blocks[Self::index(x, y, z)]
    }

    /// Set block at local chunk coordinates
    pub fn set(&mut self, x: i32, y: i32, z: i32, block: BlockType) {
        if x < 0 || x >= CHUNK_SIZE || y < 0 || y >= CHUNK_SIZE || z < 0 || z >= CHUNK_SIZE {
            return;
        }
        self.blocks[Self::index(x, y, z)] = block;
    }
}

/// The 3D voxel grid - the physical world
pub struct GridLayer {
    chunks: Arc<RwLock<AHashMap<ChunkCoord, Chunk>>>,
}

impl GridLayer {
    pub fn new() -> Self {
        Self {
            chunks: Arc::new(RwLock::new(AHashMap::new())),
        }
    }

    /// Get or create a chunk
    fn get_or_create_chunk(&self, chunk_coord: ChunkCoord) -> Chunk {
        let mut chunks = self.chunks.write();
        chunks
            .entry(chunk_coord)
            .or_insert_with(|| Chunk::new(chunk_coord))
            .clone()
    }

    /// Get block at world coordinates
    pub fn get_block(&self, coord: GridCoord) -> BlockType {
        let chunk_coord = coord.to_chunk_coord(CHUNK_SIZE);
        let chunks = self.chunks.read();
        
        if let Some(chunk) = chunks.get(&chunk_coord) {
            let local_x = coord.x.rem_euclid(CHUNK_SIZE);
            let local_y = coord.y.rem_euclid(CHUNK_SIZE);
            let local_z = coord.z.rem_euclid(CHUNK_SIZE);
            chunk.get(local_x, local_y, local_z)
        } else {
            BlockType::Air
        }
    }

    /// Set block at world coordinates
    pub fn set_block(&self, coord: GridCoord, block: BlockType) {
        let chunk_coord = coord.to_chunk_coord(CHUNK_SIZE);
        let mut chunk = self.get_or_create_chunk(chunk_coord);
        
        let local_x = coord.x.rem_euclid(CHUNK_SIZE);
        let local_y = coord.y.rem_euclid(CHUNK_SIZE);
        let local_z = coord.z.rem_euclid(CHUNK_SIZE);
        chunk.set(local_x, local_y, local_z, block);
        
        let mut chunks = self.chunks.write();
        chunks.insert(chunk_coord, chunk);
    }

    /// Check if a position is walkable
    pub fn is_walkable(&self, coord: GridCoord) -> bool {
        let block = self.get_block(coord);
        block.is_walkable()
    }

    /// Get all loaded chunks
    pub fn get_loaded_chunks(&self) -> Vec<ChunkCoord> {
        self.chunks.read().keys().copied().collect()
    }

    /// Generate simple terrain (for testing)
    pub fn generate_simple_terrain(&self, min: GridCoord, max: GridCoord) {
        for x in min.x..=max.x {
            for z in min.z..=max.z {
                // Ground level
                self.set_block(GridCoord::new(x, 0, z), BlockType::Grass);
                
                // Below ground
                for y in -5..0 {
                    self.set_block(GridCoord::new(x, y, z), BlockType::Dirt);
                }
            }
        }
    }
}

impl Default for GridLayer {
    fn default() -> Self {
        Self::new()
    }
}

/// Dynamic objects (non-voxel entities like ships, catapults)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DynamicObject {
    pub id: Uuid,
    pub object_type: String,
    pub position: Position,
    pub rotation: f32,
    pub data: serde_json::Value,
}

pub struct DynamicObjectList {
    objects: Arc<RwLock<Vec<DynamicObject>>>,
}

impl DynamicObjectList {
    pub fn new() -> Self {
        Self {
            objects: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub fn add(&self, object: DynamicObject) {
        self.objects.write().push(object);
    }

    pub fn get_all(&self) -> Vec<DynamicObject> {
        self.objects.read().clone()
    }

    pub fn remove(&self, id: Uuid) {
        self.objects.write().retain(|obj| obj.id != id);
    }
}

impl Default for DynamicObjectList {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_indexing() {
        let mut chunk = Chunk::new(ChunkCoord::new(0, 0, 0));
        chunk.set(5, 10, 15, BlockType::Stone);
        assert_eq!(chunk.get(5, 10, 15), BlockType::Stone);
    }

    #[test]
    fn test_grid_layer() {
        let grid = GridLayer::new();
        let coord = GridCoord::new(100, 50, -30);
        grid.set_block(coord, BlockType::Wood);
        assert_eq!(grid.get_block(coord), BlockType::Wood);
    }
}

