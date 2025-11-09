use anyhow::Result;
use parking_lot::RwLock;
use rand::Rng;
use std::sync::Arc;
use std::time::Instant;
use tracing::info;
use world_sim_admin_api::{AdminApiServer, AgentState as ApiAgentState, ResourceState, SimulationMetrics, WorldState};
use world_sim_agents::{AgentState, GlobalOwnershipRegistry, Job, LifecycleLayer};
use world_sim_cognitive::StimulusSubsystem;
use world_sim_core::{GridCoord, Position, SimTime};
use world_sim_event_bus::{get_event_bus, EventBus};
use world_sim_meta::DungeonMaster;
use world_sim_persistence::{Database, WorldSnapshot};
use world_sim_societal::{EconomySubsystem, PoliticalLayer, SocialLayer};
use world_sim_world::{ContentDefinitionLayer, EcologyLayer, GridLayer, ResourceManager, ResourceNodeType};

/// The main simulation orchestrator
pub struct Simulation {
    // Core infrastructure
    event_bus: Arc<EventBus>,
    database: Option<Arc<Database>>,
    
    // World layer
    grid: Arc<GridLayer>,
    ecology: EcologyLayer,
    resources: Arc<ResourceManager>,
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
        let resources = Arc::new(ResourceManager::new());
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
        
        // Generate resource nodes
        info!("Generating resource nodes...");
        resources.generate_random_nodes(50, 90.0);
        
        // Spawn initial agents with factions
        info!("Spawning initial population...");
        
        // Create two factions for drama
        let faction_a = politics.create_faction("Kingdom A".to_string(), world_sim_core::AgentId::new());
        let faction_b = politics.create_faction("Kingdom B".to_string(), world_sim_core::AgentId::new());
        
        // Declare war between them!
        politics.declare_war(faction_a, faction_b, "Border dispute".to_string()).await;
        
        for i in 0..100 {
            let x = (i % 10) as f32 * 10.0 - 50.0;
            let z = (i / 10) as f32 * 10.0 - 50.0;
            let mut agent = world_sim_agents::SimAgent::new(
                format!("Citizen_{}", i),
                Position::new(x, 1.0, z),
            );
            
            // Assign to factions (50/50 split)
            let faction = if i < 50 { faction_a } else { faction_b };
            agent.personality.beliefs.faction_loyalty = Some(faction);
            
            let agent_id = agent.id; // Save ID before moving
            lifecycle.spawn_agent(agent);
            politics.add_member(faction, agent_id);
        }
        
        info!("Initial population: {} agents in 2 warring factions", lifecycle.count_living());
        
        let metrics = Arc::new(RwLock::new(SimulationMetrics::default()));
        let world_state = Arc::new(RwLock::new(WorldState {
            agents: Vec::new(),
            resources: Vec::new(),
            terrain_size: 100,
        }));
        
        Ok(Self {
            event_bus,
            database,
            grid,
            ecology,
            resources,
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
        
        // Get all agents once for conflict checking
        let all_agents = self.lifecycle.get_agents();
        
        // Phase 3: Combat detection
        let mut combat_pairs = Vec::new();
        for i in 0..all_agents.len() {
            for j in (i+1)..all_agents.len() {
                let agent_a = &all_agents[i];
                let agent_b = &all_agents[j];
                
                if !agent_a.is_alive() || !agent_b.is_alive() {
                    continue;
                }
                
                // Check if they're in different factions (enemies)
                if let (Some(faction_a), Some(faction_b)) = (
                    agent_a.personality.beliefs.faction_loyalty,
                    agent_b.personality.beliefs.faction_loyalty,
                ) {
                    if faction_a != faction_b {
                        // Different factions = enemies!
                        let dist = agent_a.position.distance_to(&agent_b.position);
                        
                        if dist < 5.0 {
                            // Close enough to fight!
                            combat_pairs.push((agent_a.id, agent_b.id, dist));
                        }
                    }
                }
            }
        }
        
        // Process combat
        let mut rng = rand::thread_rng();
        for (id_a, id_b, dist) in combat_pairs {
            // 10% chance per tick that someone dies
            if dist < 3.0 && rng.gen::<f32>() < 0.1 {
                let loser = if rng.gen::<bool>() { id_a } else { id_b };
                self.lifecycle.kill_agent(loser, "Combat".to_string()).await;
            } else {
                // Set to fighting state
                self.lifecycle.update_agent_state(id_a, AgentState::Fighting { target: id_b });
                self.lifecycle.update_agent_state(id_b, AgentState::Fighting { target: id_a });
            }
        }
        
        // Phase 2: Job-based movement + Phase 3: Social attraction/repulsion
        let resources = self.resources.clone();
        let all_agents_for_movement = self.lifecycle.get_agents();
        
        self.lifecycle.update_living_agents(|agent| {
            // Phase 3: Social behavior
            let my_faction = agent.personality.beliefs.faction_loyalty;
            let my_pos = agent.position;
            
            // Find nearest ally and enemy
            let mut nearest_ally_pos: Option<Position> = None;
            let mut nearest_enemy_pos: Option<Position> = None;
            let mut nearest_ally_dist = f32::MAX;
            let mut nearest_enemy_dist = f32::MAX;
            
            for other in &all_agents_for_movement {
                if other.id == agent.id || !other.is_alive() {
                    continue;
                }
                
                let dist = my_pos.distance_to(&other.position);
                let other_faction = other.personality.beliefs.faction_loyalty;
                
                if let (Some(my_f), Some(other_f)) = (my_faction, other_faction) {
                    if my_f == other_f && dist < nearest_ally_dist {
                        // Same faction = ally
                        nearest_ally_pos = Some(other.position);
                        nearest_ally_dist = dist;
                    } else if my_f != other_f && dist < nearest_enemy_dist {
                        // Different faction = enemy
                        nearest_enemy_pos = Some(other.position);
                        nearest_enemy_dist = dist;
                    }
                }
            }
            
            // Movement priorities: 1) Flee from close enemies, 2) Move to job, 3) Stay near allies
            if let Some(enemy_pos) = nearest_enemy_pos {
                if nearest_enemy_dist < 10.0 {
                    // Enemy nearby! Flee or fight
                    if agent.has_trait(world_sim_core::Trait::Brave) || matches!(agent.state, AgentState::Fighting { .. }) {
                        // Move TOWARD enemy (brave or already fighting)
                        let dx = enemy_pos.x - agent.position.x;
                        let dz = enemy_pos.z - agent.position.z;
                        let dist = (dx * dx + dz * dz).sqrt();
                        if dist > 0.1 {
                            agent.position.x += (dx / dist) * 0.4;
                            agent.position.z += (dz / dist) * 0.4;
                        }
                        return; // Skip other movement
                    } else {
                        // Flee!
                        let dx = agent.position.x - enemy_pos.x;
                        let dz = agent.position.z - enemy_pos.z;
                        let dist = (dx * dx + dz * dz).sqrt();
                        if dist > 0.1 {
                            agent.position.x = (agent.position.x + (dx / dist) * 0.5).clamp(-95.0, 95.0);
                            agent.position.z = (agent.position.z + (dz / dist) * 0.5).clamp(-95.0, 95.0);
                        }
                        return; // Skip other movement
                    }
                }
            }
            
            // Normal job-based movement
            match agent.job {
                Job::Woodcutter => {
                    if let Some(tree) = resources.find_nearest(agent.position, ResourceNodeType::Tree) {
                        let dx = tree.position.x - agent.position.x;
                        let dz = tree.position.z - agent.position.z;
                        let dist = (dx * dx + dz * dz).sqrt();
                        
                        if dist > 2.0 {
                            agent.position.x += (dx / dist) * 0.3;
                            agent.position.z += (dz / dist) * 0.3;
                        } else {
                            agent.state = AgentState::Working { task: "chopping".to_string() };
                        }
                    }
                },
                Job::Miner => {
                    if let Some(rock) = resources.find_nearest(agent.position, ResourceNodeType::Rock) {
                        let dx = rock.position.x - agent.position.x;
                        let dz = rock.position.z - agent.position.z;
                        let dist = (dx * dx + dz * dz).sqrt();
                        
                        if dist > 2.0 {
                            agent.position.x += (dx / dist) * 0.3;
                            agent.position.z += (dz / dist) * 0.3;
                        } else {
                            agent.state = AgentState::Working { task: "mining".to_string() };
                        }
                    }
                },
                Job::Farmer => {
                    if let Some(farm) = resources.find_nearest(agent.position, ResourceNodeType::Farm) {
                        let dx = farm.position.x - agent.position.x;
                        let dz = farm.position.z - agent.position.z;
                        let dist = (dx * dx + dz * dz).sqrt();
                        
                        if dist > 2.0 {
                            agent.position.x += (dx / dist) * 0.3;
                            agent.position.z += (dz / dist) * 0.3;
                        } else {
                            agent.state = AgentState::Working { task: "farming".to_string() };
                        }
                    }
                },
                Job::Builder | Job::Unemployed => {
                    // Social grouping - move toward allies
                    if let Some(ally_pos) = nearest_ally_pos {
                        if nearest_ally_dist > 15.0 {
                            // Too far from allies, move toward them
                            let dx = ally_pos.x - agent.position.x;
                            let dz = ally_pos.z - agent.position.z;
                            let dist = (dx * dx + dz * dz).sqrt();
                            if dist > 0.1 {
                                agent.position.x += (dx / dist) * 0.2;
                                agent.position.z += (dz / dist) * 0.2;
                            }
                        } else {
                            // Near allies, random wander
                            let dx = rng.gen::<f32>() * 2.0 - 1.0;
                            let dz = rng.gen::<f32>() * 2.0 - 1.0;
                            agent.position.x = (agent.position.x + dx * 0.2).clamp(-95.0, 95.0);
                            agent.position.z = (agent.position.z + dz * 0.2).clamp(-95.0, 95.0);
                        }
                    } else {
                        // No allies found, random wander
                        let dx = rng.gen::<f32>() * 2.0 - 1.0;
                        let dz = rng.gen::<f32>() * 2.0 - 1.0;
                        agent.position.x = (agent.position.x + dx * 0.3).clamp(-95.0, 95.0);
                        agent.position.z = (agent.position.z + dz * 0.3).clamp(-95.0, 95.0);
                    }
                },
            }
        });
        
        Ok(())
    }
    
    /// Slow tick (1Hz) - economy, utility AI, behavior changes
    pub async fn tick_slow(&mut self, delta_seconds: f64) -> Result<()> {
        // Update economy
        self.economy.recalculate_prices().await;
        
        // Update dungeon master
        self.dungeon_master.tick(delta_seconds as f32).await;
        
        // Quick Win: Basic needs cycle for agents (runs every second)
        let mut rng = rand::thread_rng();
        
        self.lifecycle.update_living_agents(|agent| {
            // Simple state machine: Idle → Eating → Sleeping → Working → Idle
            let new_state = match &agent.state {
                AgentState::Idle => {
                    // Random chance to get hungry or tired
                    let roll = rng.gen::<f32>();
                    if roll < 0.3 {
                        AgentState::Eating
                    } else if roll < 0.5 {
                        AgentState::Sleeping
                    } else if roll < 0.7 {
                        AgentState::Working { task: "gathering".to_string() }
                    } else {
                        AgentState::Idle
                    }
                },
                AgentState::Eating => {
                    // After eating, might get tired
                    if rng.gen::<f32>() < 0.5 {
                        AgentState::Sleeping
                    } else {
                        AgentState::Idle
                    }
                },
                AgentState::Sleeping => {
                    // After sleeping, ready to work
                    AgentState::Working { task: "gathering".to_string() }
                },
                AgentState::Working { .. } => {
                    // After working, back to idle
                    AgentState::Idle
                },
                _ => agent.state.clone(),
            };
            
            agent.state = new_state;
        });
        
        // CRITICAL: Update world state for visualizer EVERY SECOND (not every 60 seconds!)
        self.sync_world_state_to_api();
        
        Ok(())
    }
    
    /// Very slow tick (1/minute) - ecology, demographics, metrics
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
        
        info!(
            "Sim Time: {:.0}s | Living Agents: {}",
            self.sim_time.seconds,
            agent_count
        );
        
        Ok(())
    }
    
    /// Sync agent positions and states to API (called every second)
    fn sync_world_state_to_api(&self) {
        let agents = self.lifecycle.get_agents();
        let resource_nodes = self.resources.get_nodes();
        
        let mut world_state = self.world_state.write();
        
        // Update agents
        world_state.agents = agents
            .iter()
            .filter(|a| a.is_alive())
            .map(|a| {
                let state_str = match &a.state {
                    AgentState::Idle => "Idle",
                    AgentState::Moving { .. } => "Moving",
                    AgentState::Working { .. } => "Working",
                    AgentState::Fighting { .. } => "Fighting",
                    AgentState::Sleeping => "Sleeping",
                    AgentState::Eating => "Eating",
                    AgentState::Dead => "Dead",
                };
                
                ApiAgentState {
                    id: format!("{:?}", a.id),
                    x: a.position.x,
                    y: a.position.y,
                    z: a.position.z,
                    name: a.name.clone(),
                    state: state_str.to_string(),
                    faction: a.personality.beliefs.faction_loyalty.map(|f| format!("{:?}", f)),
                }
            })
            .collect();
        
        // Update resources
        world_state.resources = resource_nodes
            .iter()
            .map(|r| {
                let type_str = match r.resource_type {
                    ResourceNodeType::Tree => "tree",
                    ResourceNodeType::Rock => "rock",
                    ResourceNodeType::Farm => "farm",
                    ResourceNodeType::IronDeposit => "iron",
                };
                
                ResourceState {
                    id: format!("{}", r.id),
                    resource_type: type_str.to_string(),
                    x: r.position.x,
                    y: r.position.y,
                    z: r.position.z,
                    quantity: r.quantity,
                }
            })
            .collect();
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

