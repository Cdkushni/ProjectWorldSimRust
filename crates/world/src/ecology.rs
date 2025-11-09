use rand::Rng;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;
use world_sim_core::{BlockType, GridCoord};
use world_sim_event_bus::{DroughtStartedEvent, EventBus, Season, SeasonChangeEvent};

use crate::GridLayer;

/// Manages seasons and their effects
pub struct SeasonalSubsystem {
    current_season: Season,
    days_in_season: u32,
    day_counter: u32,
}

impl SeasonalSubsystem {
    pub fn new() -> Self {
        Self {
            current_season: Season::Spring,
            days_in_season: 90,
            day_counter: 0,
        }
    }

    pub fn current_season(&self) -> Season {
        self.current_season
    }

    /// Advance time and check for season change
    pub async fn tick(&mut self, event_bus: &Arc<EventBus>) {
        self.day_counter += 1;
        
        if self.day_counter >= self.days_in_season {
            let old_season = self.current_season;
            self.current_season = match self.current_season {
                Season::Spring => Season::Summer,
                Season::Summer => Season::Autumn,
                Season::Autumn => Season::Winter,
                Season::Winter => Season::Spring,
            };
            self.day_counter = 0;
            
            event_bus
                .publish(&SeasonChangeEvent {
                    old_season,
                    new_season: self.current_season,
                })
                .await;
        }
    }
}

impl Default for SeasonalSubsystem {
    fn default() -> Self {
        Self::new()
    }
}

/// Weather states
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum WeatherState {
    Clear,
    Rain,
    Drought,
    Storm,
}

/// Manages weather patterns
pub struct WeatherSubsystem {
    current_weather: WeatherState,
    duration_remaining: u32,
}

impl WeatherSubsystem {
    pub fn new() -> Self {
        Self {
            current_weather: WeatherState::Clear,
            duration_remaining: 30,
        }
    }

    pub fn current_weather(&self) -> WeatherState {
        self.current_weather
    }

    pub async fn tick(&mut self, event_bus: &Arc<EventBus>) {
        self.duration_remaining = self.duration_remaining.saturating_sub(1);
        
        if self.duration_remaining == 0 {
            // Change weather randomly
            let mut rng = rand::thread_rng();
            self.current_weather = match rng.gen_range(0..4) {
                0 => WeatherState::Clear,
                1 => WeatherState::Rain,
                2 => WeatherState::Drought,
                _ => WeatherState::Storm,
            };
            self.duration_remaining = rng.gen_range(10..60);
            
            // Publish drought event if applicable
            if matches!(self.current_weather, WeatherState::Drought) {
                event_bus
                    .publish(&DroughtStartedEvent {
                        region: "global".to_string(),
                        severity: 0.7,
                        expected_duration_days: self.duration_remaining,
                    })
                    .await;
            }
        }
    }
}

impl Default for WeatherSubsystem {
    fn default() -> Self {
        Self::new()
    }
}

/// Manages natural resource growth and decay
pub struct ResourceLifeCycle {
    grid: Arc<GridLayer>,
    growth_rate: f32,
}

impl ResourceLifeCycle {
    pub fn new(grid: Arc<GridLayer>) -> Self {
        Self {
            grid,
            growth_rate: 0.01,
        }
    }

    /// Process natural growth (trees, grass, etc.)
    pub fn tick(&self) {
        let chunks = self.grid.get_loaded_chunks();
        let mut rng = rand::thread_rng();
        
        for chunk_coord in chunks {
            // Randomly grow trees in this chunk
            if rng.gen::<f32>() < self.growth_rate {
                let x = chunk_coord.x * crate::grid::CHUNK_SIZE + rng.gen_range(0..32);
                let z = chunk_coord.z * crate::grid::CHUNK_SIZE + rng.gen_range(0..32);
                let coord = GridCoord::new(x, 1, z);
                
                // If there's grass, maybe grow a tree
                if self.grid.get_block(coord) == BlockType::Air {
                    let below = GridCoord::new(x, 0, z);
                    if self.grid.get_block(below) == BlockType::Grass {
                        self.grid.set_block(coord, BlockType::Wood);
                    }
                }
            }
        }
    }
}

/// Simple fauna (non-GOAP animals)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaunaAgent {
    pub id: Uuid,
    pub species: String,
    pub position: GridCoord,
    pub health: f32,
}

pub struct FaunaSubsystem {
    agents: Vec<FaunaAgent>,
}

impl FaunaSubsystem {
    pub fn new() -> Self {
        Self {
            agents: Vec::new(),
        }
    }

    pub fn spawn_animal(&mut self, species: String, position: GridCoord) {
        self.agents.push(FaunaAgent {
            id: Uuid::new_v4(),
            species,
            position,
            health: 100.0,
        });
    }

    pub fn tick(&mut self, grid: &GridLayer) {
        let mut rng = rand::thread_rng();
        
        for agent in &mut self.agents {
            // Simple random movement
            let dx = rng.gen_range(-1..=1);
            let dz = rng.gen_range(-1..=1);
            let new_pos = GridCoord::new(
                agent.position.x + dx,
                agent.position.y,
                agent.position.z + dz,
            );
            
            if grid.is_walkable(new_pos) {
                agent.position = new_pos;
            }
        }
    }

    pub fn get_agents(&self) -> &[FaunaAgent] {
        &self.agents
    }
}

impl Default for FaunaSubsystem {
    fn default() -> Self {
        Self::new()
    }
}

/// Combined ecology layer
pub struct EcologyLayer {
    pub seasons: SeasonalSubsystem,
    pub weather: WeatherSubsystem,
    pub resources: ResourceLifeCycle,
    pub fauna: FaunaSubsystem,
}

impl EcologyLayer {
    pub fn new(grid: Arc<GridLayer>) -> Self {
        Self {
            seasons: SeasonalSubsystem::new(),
            weather: WeatherSubsystem::new(),
            resources: ResourceLifeCycle::new(grid),
            fauna: FaunaSubsystem::new(),
        }
    }

    pub async fn tick(&mut self, event_bus: &Arc<EventBus>, grid: &GridLayer) {
        self.seasons.tick(event_bus).await;
        self.weather.tick(event_bus).await;
        self.resources.tick();
        self.fauna.tick(grid);
    }
}

