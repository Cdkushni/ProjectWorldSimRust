use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use std::sync::Arc;

use crate::server::ApiState;

/// Health check endpoint
pub async fn health_check() -> &'static str {
    "OK"
}

/// Get event history
#[derive(Deserialize)]
pub struct HistoryQuery {
    pub event_type: Option<String>,
    pub limit: Option<i64>,
}

pub async fn get_event_history(
    State(state): State<Arc<ApiState>>,
    Query(query): Query<HistoryQuery>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    if let Some(db) = &state.database {
        let limit = query.limit.unwrap_or(100);
        let events = db
            .query_events(query.event_type.as_deref(), limit)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        
        Ok(Json(serde_json::json!({ "events": events })))
    } else {
        // No database - return empty with message
        Ok(Json(serde_json::json!({ 
            "events": [],
            "message": "Database not configured. Set DATABASE_URL to enable event history."
        })))
    }
}

/// Inject a custom event (Dungeon Master control)
#[derive(Deserialize)]
pub struct InjectEventRequest {
    pub event_type: String,
    pub payload: serde_json::Value,
}

pub async fn inject_event(
    State(state): State<Arc<ApiState>>,
    Json(request): Json<InjectEventRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Create event envelope
    let envelope = world_sim_event_bus::EventEnvelope::new(
        request.event_type,
        "admin_api".to_string(),
        request.payload,
    );
    
    // Publish to event bus
    state.event_bus.publish_envelope(envelope.clone()).await;
    
    Ok(Json(serde_json::json!({
        "success": true,
        "event_id": envelope.id
    })))
}

/// Add a false memory to an agent
#[derive(Deserialize)]
pub struct AddMemoryRequest {
    pub fact: String,
    #[allow(dead_code)]
    pub source: Option<String>,
}

pub async fn add_agent_memory(
    Path(agent_id): Path<String>,
    Json(request): Json<AddMemoryRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // TODO: Integrate with actual agent memory system
    Ok(Json(serde_json::json!({
        "success": true,
        "agent_id": agent_id,
        "memory": request.fact
    })))
}

/// Get agent information
pub async fn get_agent_info(
    Path(agent_id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // TODO: Integrate with actual agent system
    Ok(Json(serde_json::json!({
        "agent_id": agent_id,
        "status": "placeholder"
    })))
}

/// Create a world snapshot
pub async fn create_snapshot(
    State(_state): State<Arc<ApiState>>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // TODO: Integrate with actual world state
    Ok(Json(serde_json::json!({
        "success": true,
        "message": "Snapshot created (placeholder)"
    })))
}

/// List all snapshots
pub async fn list_snapshots(
    State(state): State<Arc<ApiState>>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    if let Some(db) = &state.database {
        let snapshots = db
            .list_snapshots()
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        
        Ok(Json(serde_json::json!({ "snapshots": snapshots })))
    } else {
        Err(StatusCode::SERVICE_UNAVAILABLE)
    }
}

/// Get simulation metrics
pub async fn get_metrics(
    State(state): State<Arc<ApiState>>,
) -> Json<serde_json::Value> {
    let metrics = state.metrics.read();
    Json(serde_json::json!({
        "uptime_seconds": metrics.uptime_seconds,
        "agent_count": metrics.agent_count,
        "events_processed": metrics.events_processed
    }))
}

/// Get world state
pub async fn get_world_state(
    State(state): State<Arc<ApiState>>,
) -> Json<serde_json::Value> {
    let world_state = state.world_state.read();
    Json(serde_json::json!({
        "agents": world_state.agents,
        "terrain_size": world_state.terrain_size
    }))
}

