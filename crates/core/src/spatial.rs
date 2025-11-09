use nalgebra::Vector3;
use serde::{Deserialize, Serialize};

/// World position in 3D space
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Position {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Position {
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    pub fn distance_to(&self, other: &Position) -> f32 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        let dz = self.z - other.z;
        (dx * dx + dy * dy + dz * dz).sqrt()
    }

    pub fn to_vector3(&self) -> Vector3<f32> {
        Vector3::new(self.x, self.y, self.z)
    }

    pub fn to_grid_coord(&self) -> GridCoord {
        GridCoord {
            x: self.x.floor() as i32,
            y: self.y.floor() as i32,
            z: self.z.floor() as i32,
        }
    }
}

/// Integer grid coordinate
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GridCoord {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl GridCoord {
    pub fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }

    pub fn to_position(&self) -> Position {
        Position {
            x: self.x as f32 + 0.5,
            y: self.y as f32 + 0.5,
            z: self.z as f32 + 0.5,
        }
    }

    pub fn to_chunk_coord(&self, chunk_size: i32) -> ChunkCoord {
        ChunkCoord {
            x: self.x.div_euclid(chunk_size),
            y: self.y.div_euclid(chunk_size),
            z: self.z.div_euclid(chunk_size),
        }
    }

    pub fn manhattan_distance(&self, other: &GridCoord) -> i32 {
        (self.x - other.x).abs() + (self.y - other.y).abs() + (self.z - other.z).abs()
    }
}

/// Chunk coordinate for spatial partitioning
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChunkCoord {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl ChunkCoord {
    pub fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }
}

/// Bounding box in 3D space
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct BoundingBox {
    pub min: Position,
    pub max: Position,
}

impl BoundingBox {
    pub fn contains(&self, pos: &Position) -> bool {
        pos.x >= self.min.x
            && pos.x <= self.max.x
            && pos.y >= self.min.y
            && pos.y <= self.max.y
            && pos.z >= self.min.z
            && pos.z <= self.max.z
    }
}

