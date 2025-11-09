/// Admin API - HTTP/WebSocket server for external control and monitoring
mod routes;
mod handlers;
mod server;

pub use server::{AdminApiServer, SimulationMetrics, WorldState, AgentState};

