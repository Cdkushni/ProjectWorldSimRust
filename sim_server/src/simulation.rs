use anyhow::Result;
use parking_lot::RwLock;
use std::sync::Arc;
use std::time::Instant;
use tracing::info;
use world_sim_admin_api::{AdminApiServer, AgentState, SimulationMetrics, WorldState};
use world_sim_agents::{GlobalOwnershipRegistry, LifecycleLayer};
use world_sim_cognitive::StimulusSubsystem;
use world_sim_core::{GridCoord, Position, SimTime};
use world_sim_event_bus::{get_event_bus, EventBus};
use world_sim_meta::DungeonMaster;
use world_sim_persistence::{Database, WorldSnapshot};
use world_sim_societal::{EconomySubsystem, PoliticalLayer, SocialLayer};
use world_sim_world::{ContentDefinitionLayer, EcologyLayer, GridLayer};

/// The main simulation orchestrator
pub struct Simulation {
    // Core infrastructure
    event_bus: Arc<EventBus>,
    database: Option<Arc<Database>>,
    
    // World layer
    grid: Arc<GridLayer>,
    ecology: EcologyLayer,
    #[allow(dead_code)]
    content: Arc<ContentDefinitionLayer>,
    
    // Agent layer
    lifecycle: Arc<LifecycleLayer>,
    #[allow(dead_code)]
    ownership: Arc<GlobalOwnershipRegistry>,
    
    // Cognitive layer
    #[allow(dead_code)]
    stimulus: Arc<StimulusSubsystem>,
    
    // Societal layer
    #[allow(dead_code)]
    social: Arc<SocialLayer>,
    economy: Arc<EconomySubsystem>,
    #[allow(dead_code)]
    politics: Arc<PoliticalLayer>,
    
    // Meta layer
    dungeon_master: Arc<DungeonMaster>,
    
    // Simulation state
    sim_time: SimTime,
    start_time: Instant,
    metrics: Arc<RwLock<SimulationMetrics>>,
    world_state: Arc<RwLock<WorldState>>,
}

impl Simulation {
    pub async fn new() -> Result<Self> {
        let event_bus = get_event_bus();
        
        // Initialize database (optional - can run without persistence)
        let database = match std::env::var("DATABASE_URL") {
            Ok(url) => {
                info!("Connecting to database...");
                let db = Database::new(&url).await?;
                db.initialize_schema().await?;
                Some(Arc::new(db))
            }
            Err(_) => {
                info!("No DATABASE_URL provided, running without persistence");
                None
            }
        };
        
        // World layer
        let grid = Arc::new(GridLayer::new());
        let ecology = EcologyLayer::new(grid.clone());
        let content = Arc::new(ContentDefinitionLayer::new());
        
        // Agent layer
        let lifecycle = Arc::new(LifecycleLayer::new(event_bus.clone()));
        let ownership = Arc::new(GlobalOwnershipRegistry::new());
        
        // Cognitive layer
        let stimulus = Arc::new(StimulusSubsystem::new());
        
        // Societal layer
        let social = Arc::new(SocialLayer::new());
        let economy = Arc::new(EconomySubsystem::new(event_bus.clone()));
        let politics = Arc::new(PoliticalLayer::new(event_bus.clone()));
        
        // Meta layer
        let dungeon_master = Arc::new(DungeonMaster::new(event_bus.clone()));
        
        // Generate initial world
        info!("Generating initial world...");
        grid.generate_simple_terrain(
            GridCoord::new(-50, 0, -50),
            GridCoord::new(50, 0, 50),
        );
        
        // Spawn initial agents
        info!("Spawning initial population...");
        for i in 0..100 {
            let x = (i % 10) as f32 * 10.0 - 50.0;
            let z = (i / 10) as f32 * 10.0 - 50.0;
            let agent = world_sim_agents::SimAgent::new(
                format!("Citizen_{}", i),
                Position::new(x, 1.0, z),
            );
            lifecycle.spawn_agent(agent);
        }
        
        info!("Initial population: {} agents", lifecycle.count_living());
        
        let metrics = Arc::new(RwLock::new(SimulationMetrics::default()));
        let world_state = Arc::new(RwLock::new(WorldState {
            agents: Vec::new(),
            terrain_size: 100,
        }));
        
        Ok(Self {
            event_bus,
            database,
            grid,
            ecology,
            content,
            lifecycle,
            ownership,
            stimulus,
            social,
            economy,
            politics,
            dungeon_master,
            sim_time: SimTime::new(),
            start_time: Instant::now(),
            metrics,
            world_state,
        })
    }
    
    /// Fast tick (10Hz) - real-time systems
    pub async fn tick_fast(&mut self, delta_seconds: f64) -> Result<()> {
        self.sim_time.advance(delta_seconds);
        
        // Perception and GOAP planning happen here (for active agents)
        // TODO: Implement AI LOD system
        
        Ok(())
    }
    
    /// Slow tick (1Hz) - economy, utility AI
    pub async fn tick_slow(&mut self, delta_seconds: f64) -> Result<()> {
        // Update economy
        self.economy.recalculate_prices().await;
        
        // Update dungeon master
        self.dungeon_master.tick(delta_seconds as f32).await;
        
        Ok(())
    }
    
    /// Very slow tick (1/minute) - ecology, demographics
    pub async fn tick_very_slow(&mut self, _delta_seconds: f64) -> Result<()> {
        // Update ecology
        self.ecology.tick(&self.event_bus, &self.grid).await;
        
        // Update demographics
        self.lifecycle.tick().await;
        
        let agent_count = self.lifecycle.count_living();
        
        // Update metrics
        {
            let mut metrics = self.metrics.write();
            metrics.agent_count = agent_count;
            metrics.uptime_seconds = self.start_time.elapsed().as_secs();
            metrics.events_processed += 1;
        }
        
        // Update world state for visualizer
        {
            let agents = self.lifecycle.get_agents();
            let mut world_state = self.world_state.write();
            world_state.agents = agents
                .iter()
                .filter(|a| a.is_alive())
                .map(|a| AgentState {
                    id: format!("{:?}", a.id),
                    x: a.position.x,
                    y: a.position.y,
                    z: a.position.z,
                    name: a.name.clone(),
                })
                .collect();
        }
        
        info!(
            "Sim Time: {:.0}s | Living Agents: {}",
            self.sim_time.seconds,
            agent_count
        );
        
        Ok(())
    }
    
    /// Save a world snapshot
    pub async fn save_snapshot(&self) -> Result<()> {
        if let Some(db) = &self.database {
            let snapshot = WorldSnapshot::new("AutoSave".to_string());
            let data = snapshot.to_bytes()?;
            let id = db.save_snapshot("AutoSave", data).await?;
            info!("Snapshot saved: {}", id);
        }
        Ok(())
    }
    
    /// Get the admin API server
    pub fn get_admin_api_server(&self) -> AdminApiServer {
        let mut server = AdminApiServer::new(self.event_bus.clone());
        if let Some(db) = &self.database {
            server = server.with_database(db.clone());
        }
        server = server.with_metrics(self.metrics.clone());
        server = server.with_world_state(self.world_state.clone());
        server
    }
}

