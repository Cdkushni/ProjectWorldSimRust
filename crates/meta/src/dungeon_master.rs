use parking_lot::RwLock;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use world_sim_core::{Position, ResourceType};
use world_sim_event_bus::{BlightStartedEvent, DungeonMasterEvent, DroughtStartedEvent, EventBus};

/// The Dungeon Master - AI storyteller that injects drama
pub struct DungeonMaster {
    event_bus: Arc<EventBus>,
    metrics: Arc<RwLock<WorldMetrics>>,
    story_events: Vec<StoryEvent>,
    boredom_threshold: f32,
}

/// Tracks world state for boredom detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldMetrics {
    pub average_price_volatility: f32,
    pub active_conflicts: u32,
    pub recent_deaths: u32,
    pub agent_activity_level: f32,
    pub time_since_last_event: f32,
}

impl Default for WorldMetrics {
    fn default() -> Self {
        Self {
            average_price_volatility: 0.0,
            active_conflicts: 0,
            recent_deaths: 0,
            agent_activity_level: 1.0,
            time_since_last_event: 0.0,
        }
    }
}

/// A story event that can be injected
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoryEvent {
    pub id: String,
    pub name: String,
    pub description: String,
    pub impact: ImpactType,
    pub cooldown: f32, // Seconds before can trigger again
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ImpactType {
    Blight { resource: ResourceType },
    Drought { severity: f32 },
    Plague { mortality_rate: f32 },
    Uprising { region: String },
    NaturalDisaster { disaster_type: String },
    Discovery { discovery_type: String },
}

impl DungeonMaster {
    pub fn new(event_bus: Arc<EventBus>) -> Self {
        let story_events = Self::initialize_story_events();
        
        Self {
            event_bus,
            metrics: Arc::new(RwLock::new(WorldMetrics::default())),
            story_events,
            boredom_threshold: 0.3,
        }
    }

    /// Initialize the library of possible story events
    fn initialize_story_events() -> Vec<StoryEvent> {
        vec![
            StoryEvent {
                id: "wood_blight".to_string(),
                name: "Forest Blight".to_string(),
                description: "A mysterious disease spreads through the forests, killing trees.".to_string(),
                impact: ImpactType::Blight {
                    resource: ResourceType::Wood,
                },
                cooldown: 300.0,
            },
            StoryEvent {
                id: "great_drought".to_string(),
                name: "Great Drought".to_string(),
                description: "The rains fail to come, and water becomes scarce.".to_string(),
                impact: ImpactType::Drought { severity: 0.8 },
                cooldown: 600.0,
            },
            StoryEvent {
                id: "plague".to_string(),
                name: "Plague Outbreak".to_string(),
                description: "A deadly plague sweeps through the population.".to_string(),
                impact: ImpactType::Plague {
                    mortality_rate: 0.2,
                },
                cooldown: 900.0,
            },
            StoryEvent {
                id: "peasant_revolt".to_string(),
                name: "Peasant Uprising".to_string(),
                description: "The common folk rise up against their rulers.".to_string(),
                impact: ImpactType::Uprising {
                    region: "central_kingdom".to_string(),
                },
                cooldown: 450.0,
            },
            StoryEvent {
                id: "earthquake".to_string(),
                name: "Earthquake".to_string(),
                description: "The earth shakes violently, destroying buildings.".to_string(),
                impact: ImpactType::NaturalDisaster {
                    disaster_type: "earthquake".to_string(),
                },
                cooldown: 1200.0,
            },
            StoryEvent {
                id: "gold_discovery".to_string(),
                name: "Gold Discovery".to_string(),
                description: "A vast gold deposit is discovered, sparking a rush.".to_string(),
                impact: ImpactType::Discovery {
                    discovery_type: "gold_vein".to_string(),
                },
                cooldown: 800.0,
            },
        ]
    }

    /// Update metrics and check for boredom
    pub fn update_metrics(&self, metrics: WorldMetrics) {
        *self.metrics.write() = metrics;
    }

    /// Calculate boredom score (0 = exciting, 1 = boring)
    pub fn calculate_boredom(&self) -> f32 {
        let metrics = self.metrics.read();
        
        let mut boredom: f32 = 0.0;

        // Low volatility = boring
        if metrics.average_price_volatility < 0.1 {
            boredom += 0.3;
        }

        // No conflicts = boring
        if metrics.active_conflicts == 0 {
            boredom += 0.2;
        }

        // Time since last event
        if metrics.time_since_last_event > 300.0 {
            boredom += 0.3;
        }

        // Low agent activity = boring
        if metrics.agent_activity_level < 0.3 {
            boredom += 0.2;
        }

        boredom.min(1.0)
    }

    /// Check if should inject an event
    pub async fn tick(&self, delta_time: f32) {
        // Update time since last event
        {
            let mut metrics = self.metrics.write();
            metrics.time_since_last_event += delta_time;
        }

        let boredom = self.calculate_boredom();
        
        if boredom > self.boredom_threshold {
            // The world is boring, inject some drama!
            self.inject_random_event().await;
        }
    }

    /// Inject a random story event
    pub async fn inject_random_event(&self) {
        let mut rng = rand::thread_rng();
        let event_index = rng.gen_range(0..self.story_events.len());
        let event = &self.story_events[event_index];

        self.inject_event(event).await;
    }

    /// Inject a specific story event
    pub async fn inject_event(&self, event: &StoryEvent) {
        // Reset time counter
        {
            let mut metrics = self.metrics.write();
            metrics.time_since_last_event = 0.0;
        }

        // Publish DM event
        self.event_bus
            .publish(&DungeonMasterEvent {
                event_name: event.name.clone(),
                description: event.description.clone(),
                impact: format!("{:?}", event.impact),
            })
            .await;

        // Inject the actual impact event
        match &event.impact {
            ImpactType::Blight { resource } => {
                self.event_bus
                    .publish(&BlightStartedEvent {
                        center: Position::new(0.0, 0.0, 0.0), // TODO: Choose strategically
                        radius: 100.0,
                        affected_resource: *resource,
                    })
                    .await;
            }
            ImpactType::Drought { severity } => {
                self.event_bus
                    .publish(&DroughtStartedEvent {
                        region: "global".to_string(),
                        severity: *severity,
                        expected_duration_days: 30,
                    })
                    .await;
            }
            ImpactType::Plague { mortality_rate: _ } => {
                // TODO: Implement plague system
            }
            ImpactType::Uprising { region: _ } => {
                // TODO: Implement uprising system
            }
            ImpactType::NaturalDisaster { disaster_type: _ } => {
                // TODO: Implement disaster system
            }
            ImpactType::Discovery { discovery_type: _ } => {
                // TODO: Implement discovery system
            }
        }
    }

    /// Manually inject an event by name (from Admin API)
    pub async fn inject_event_by_name(&self, event_name: &str) {
        if let Some(event) = self.story_events.iter().find(|e| e.id == event_name) {
            self.inject_event(event).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_dungeon_master() {
        let event_bus = Arc::new(EventBus::new());
        let dm = DungeonMaster::new(event_bus.clone());

        let mut metrics = WorldMetrics::default();
        metrics.time_since_last_event = 400.0;
        metrics.active_conflicts = 0;

        dm.update_metrics(metrics);

        let boredom = dm.calculate_boredom();
        assert!(boredom > 0.3); // Should be bored
    }
}

