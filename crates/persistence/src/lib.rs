/// Persistence layer for saving/loading simulation state
mod database;
mod snapshot;

pub use database::*;
pub use snapshot::*;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum PersistenceError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    
    #[error("Bincode error: {0}")]
    Bincode(#[from] bincode::Error),
    
    #[error("Not found: {0}")]
    NotFound(String),
}

pub type Result<T> = std::result::Result<T, PersistenceError>;

