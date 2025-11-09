use crate::routes;
use axum::{
    routing::{get, post},
    Router,
};
use parking_lot::RwLock;
use serde::Serialize;
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use world_sim_event_bus::EventBus;
use world_sim_persistence::Database;

/// Simulation metrics for API
#[derive(Clone, Default)]
pub struct SimulationMetrics {
    pub agent_count: usize,
    pub events_processed: u64,
    pub uptime_seconds: u64,
}

/// World state for visualization
#[derive(Clone, Default)]
pub struct WorldState {
    pub agents: Vec<AgentState>,
    pub terrain_size: i32,
}

/// Agent state for visualization
#[derive(Clone, Serialize)]
pub struct AgentState {
    pub id: String,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub name: String,
}

/// Admin API server
pub struct AdminApiServer {
    event_bus: Arc<EventBus>,
    database: Option<Arc<Database>>,
    metrics: Arc<RwLock<SimulationMetrics>>,
    world_state: Arc<RwLock<WorldState>>,
}

impl AdminApiServer {
    pub fn new(event_bus: Arc<EventBus>) -> Self {
        Self {
            event_bus,
            database: None,
            metrics: Arc::new(RwLock::new(SimulationMetrics::default())),
            world_state: Arc::new(RwLock::new(WorldState::default())),
        }
    }

    pub fn with_database(mut self, database: Arc<Database>) -> Self {
        self.database = Some(database);
        self
    }

    pub fn with_metrics(mut self, metrics: Arc<RwLock<SimulationMetrics>>) -> Self {
        self.metrics = metrics;
        self
    }

    pub fn with_world_state(mut self, world_state: Arc<RwLock<WorldState>>) -> Self {
        self.world_state = world_state;
        self
    }

    /// Build the router
    pub fn build_router(self) -> Router {
        let state = Arc::new(ApiState {
            event_bus: self.event_bus,
            database: self.database,
            metrics: self.metrics,
            world_state: self.world_state,
        });

        Router::new()
            // Health check
            .route("/health", get(routes::health_check))
            
            // Event history
            .route("/api/history", get(routes::get_event_history))
            
            // Dungeon Master controls
            .route("/api/dm/inject_event", post(routes::inject_event))
            
            // Agent manipulation
            .route("/api/agent/:id/add_memory", post(routes::add_agent_memory))
            .route("/api/agent/:id", get(routes::get_agent_info))
            
            // World state
            .route("/api/world/snapshot", get(routes::create_snapshot))
            .route("/api/world/snapshots", get(routes::list_snapshots))
            
            // Metrics
            .route("/api/metrics", get(routes::get_metrics))
            
            // World state
            .route("/api/world/state", get(routes::get_world_state))
            
            .layer(CorsLayer::permissive())
            .with_state(state)
    }

    /// Start the server
    pub async fn serve(self, addr: &str) -> anyhow::Result<()> {
        let router = self.build_router();
        let listener = tokio::net::TcpListener::bind(addr).await?;
        
        tracing::info!("Admin API server listening on {}", addr);
        
        axum::serve(listener, router).await?;
        Ok(())
    }
}

/// Shared state for API handlers
pub struct ApiState {
    pub event_bus: Arc<EventBus>,
    pub database: Option<Arc<Database>>,
    pub metrics: Arc<RwLock<SimulationMetrics>>,
    pub world_state: Arc<RwLock<WorldState>>,
}

