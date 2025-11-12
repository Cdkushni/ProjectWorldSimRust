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
use world_sim_societal::{CurrencySystem, EconomySubsystem, MarketSystem, MarketType, PoliticalLayer, SocialLayer};
use uuid::Uuid;
use world_sim_world::{Building, BuildingManager, BuildingOwner, BuildingType, ContentDefinitionLayer, EcologyLayer, GridLayer, ResourceManager, ResourceNodeType};

/// The main simulation orchestrator
pub struct Simulation {
    // Core infrastructure
    event_bus: Arc<EventBus>,
    database: Option<Arc<Database>>,
    
    // World layer
    grid: Arc<GridLayer>,
    ecology: EcologyLayer,
    resources: Arc<ResourceManager>,
    buildings: Arc<RwLock<BuildingManager>>,
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
    markets: Arc<RwLock<world_sim_societal::MarketSystem>>,
    currency: Arc<RwLock<world_sim_societal::CurrencySystem>>,
    kingdoms: Arc<RwLock<world_sim_societal::KingdomManager>>,
    
    // Meta layer
    dungeon_master: Arc<DungeonMaster>,
    
    // Simulation state
    sim_time: SimTime,
    start_time: Instant,
    metrics: Arc<RwLock<SimulationMetrics>>,
    world_state: Arc<RwLock<WorldState>>,
    
    // Economic timing
    wage_timer: std::sync::atomic::AtomicU64, // Seconds since last wage payment
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
        let buildings = Arc::new(RwLock::new(BuildingManager::new()));
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
        
        // Spawn initial agents WITHOUT factions - they will form organically
        info!("Spawning initial population without factions...");
        
        // Note: Factions will form organically through events (rebellions, coalitions, etc.)
        // For now, agents are just a unified population with social hierarchy
        
        // Class distribution (total 100 agents)
        let class_counts = vec![
            (world_sim_agents::SocialClass::King, 2),      // 2 potential leaders
            (world_sim_agents::SocialClass::Noble, 4),     // 4 nobles
            (world_sim_agents::SocialClass::Knight, 8),    // 8 knights
            (world_sim_agents::SocialClass::Soldier, 14),  // 14 soldiers
            (world_sim_agents::SocialClass::Merchant, 12), // 12 merchants
            (world_sim_agents::SocialClass::Burgher, 10),  // 10 burghers
            (world_sim_agents::SocialClass::Cleric, 4),    // 4 clerics
            (world_sim_agents::SocialClass::Peasant, 46),  // 46 peasants
        ];
        
        let mut agent_counter = 0;
        let mut king_ids = Vec::new();
        
        for (social_class, count) in class_counts {
            for j in 0..count {
                // Spread agents in a circular pattern around center
                let angle = (agent_counter as f32 * 2.0 * std::f32::consts::PI) / 100.0;
                let radius = 20.0 + (j as f32 * 3.0) + ((agent_counter as f32 % 5.0) * 10.0);
                let x = angle.cos() * radius;
                let z = angle.sin() * radius;
                
                let class_name = match social_class {
                    world_sim_agents::SocialClass::King => "King",
                    world_sim_agents::SocialClass::Noble => "Noble",
                    world_sim_agents::SocialClass::Knight => "Knight",
                    world_sim_agents::SocialClass::Soldier => "Soldier",
                    world_sim_agents::SocialClass::Merchant => "Merchant",
                    world_sim_agents::SocialClass::Burgher => "Burgher",
                    world_sim_agents::SocialClass::Cleric => "Cleric",
                    world_sim_agents::SocialClass::Peasant => "Peasant",
                };
                
                let agent = world_sim_agents::SimAgent::new_with_class(
                    format!("{}_{}", class_name, agent_counter),
                    Position::new(x, 1.0, z),
                    social_class,
                );
                
                // Track kings for potential future faction formation
                if matches!(social_class, world_sim_agents::SocialClass::King) {
                    king_ids.push(agent.id);
                }
                
                lifecycle.spawn_agent(agent);
                agent_counter += 1;
            }
        }
        
        info!("Initial population: {} agents WITHOUT factions - society will develop organically", lifecycle.count_living());
        info!("Social distribution: 2 Kings, 4 Nobles, 8 Knights, 14 Soldiers, 12 Merchants, 10 Burghers, 4 Clerics, 46 Peasants");
        info!("Note: Factions will form through events like rebellions or coalitions");
        
        // Initialize currency and market systems
        let currency = Arc::new(RwLock::new(CurrencySystem::new(20000.0))); // 20k starting money supply
        let mut market_system = MarketSystem::new();
        
        // Create initial public markets - neutral, available to all
        info!("Creating initial public markets...");
        
        // Central market (town square)
        market_system.create_market(
            "Central Market".to_string(),
            Position::new(0.0, 1.0, 0.0),
            MarketType::General,
        );
        
        // Food market (north)
        market_system.create_market(
            "Northern Food Market".to_string(),
            Position::new(0.0, 1.0, 40.0),
            MarketType::Food,
        );
        
        // Materials market (south)
        market_system.create_market(
            "Southern Materials Market".to_string(),
            Position::new(0.0, 1.0, -40.0),
            MarketType::Materials,
        );
        
        info!("Created {} public markets", market_system.get_all_markets().len());
        
        let markets = Arc::new(RwLock::new(market_system));
        let kingdoms = Arc::new(RwLock::new(world_sim_societal::KingdomManager::new()));
        
        // Create initial public buildings - neutral, community-owned
        info!("Creating initial public buildings...");
        let mut building_manager = buildings.write();
        
        // Central warehouse (public storage)
        let mut central_warehouse = Building::new(
            BuildingType::Warehouse,
            Position::new(-30.0, 1.0, 0.0),
            "Community Warehouse".to_string(),
            BuildingOwner::Public,
        );
        central_warehouse.construction_progress = 1.0; // Start complete
        building_manager.add_building(central_warehouse);
        
        // Barracks (public security)
        let mut public_barracks = Building::new(
            BuildingType::Barracks,
            Position::new(30.0, 1.0, 0.0),
            "Town Guard Barracks".to_string(),
            BuildingOwner::Public,
        );
        public_barracks.construction_progress = 1.0; // Start complete
        building_manager.add_building(public_barracks);
        
        info!("Created {} public buildings", building_manager.get_all_buildings().len());
        drop(building_manager);
        
        let metrics = Arc::new(RwLock::new(SimulationMetrics::default()));
        let world_state = Arc::new(RwLock::new(WorldState {
            agents: Vec::new(),
            resources: Vec::new(),
            markets: Vec::new(),
            buildings: Vec::new(),
            currency_info: world_sim_admin_api::CurrencyInfo::default(),
            terrain_size: 100,
        }));
        
        let mut simulation = Self {
            event_bus,
            database,
            grid,
            ecology,
            resources,
            buildings,
            content,
            lifecycle,
            ownership,
            stimulus,
            social,
            economy,
            politics,
            markets,
            currency,
            kingdoms,
            dungeon_master,
            sim_time: SimTime::new(),
            start_time: Instant::now(),
            metrics,
            world_state,
            wage_timer: std::sync::atomic::AtomicU64::new(0),
        };
        
        // IMMEDIATE LABOR REBALANCING on startup
        info!("🕐 Running INITIAL labor rebalancing...");
        simulation.rebalance_labor();
        info!("✅ Initial labor rebalancing complete");
        
        Ok(simulation)
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
                        
                        if dist < 15.0 {
                            // Close enough to fight! (increased from 5.0 for more frequent combat)
                            combat_pairs.push((agent_a.id, agent_b.id, dist));
                        }
                    }
                }
            }
        }
        
        // Process combat and resource raiding
        let mut rng = rand::thread_rng();
        for (id_a, id_b, dist) in combat_pairs {
            // Set to fighting state whenever enemies are in range
            self.lifecycle.update_agent_state(id_a, AgentState::Fighting { target: id_b });
            self.lifecycle.update_agent_state(id_b, AgentState::Fighting { target: id_a });
            
            // 15% chance per tick that someone dies when very close
            if dist < 5.0 && rng.gen::<f32>() < 0.15 {
                let loser = if rng.gen::<bool>() { id_a } else { id_b };
                let winner = if loser == id_a { id_b } else { id_a };
                
                // Get loser's position for resource raiding
                let loser_pos = all_agents.iter()
                    .find(|a| a.id == loser)
                    .map(|a| a.position);
                
                self.lifecycle.kill_agent(loser, "Combat".to_string()).await;
                
                // Resource raiding: Winner steals resources near combat location
                if let Some(combat_pos) = loser_pos {
                    // 30% chance to raid nearby resources after winning combat
                    if rng.gen::<f32>() < 0.3 {
                        let resource_nodes = self.resources.get_nodes();
                        
                        // Find nearest resource within 10 units
                        if let Some(resource) = resource_nodes.iter()
                            .filter(|r| r.position.distance_to(&combat_pos) < 10.0 && r.quantity > 0)
                            .min_by(|a, b| {
                                let dist_a = a.position.distance_to(&combat_pos);
                                let dist_b = b.position.distance_to(&combat_pos);
                                dist_a.partial_cmp(&dist_b).unwrap_or(std::cmp::Ordering::Equal)
                            })
                        {
                            // Raid 10-30% of the resource
                            let raid_percent = rng.gen_range(0.1..0.3);
                            let raid_amount = (resource.quantity as f32 * raid_percent) as u32;
                            let raid_amount = raid_amount.max(1).min(resource.quantity);
                            
                            if self.resources.harvest(resource.id, raid_amount).is_some() {
                                info!("⚔️ Resource raided! Winner took {} units from {:?}", raid_amount, resource.resource_type);
                                
                                // Winner's faction gains resources (stored in warehouse if available)
                                if let Some(winner_agent) = all_agents.iter().find(|a| a.id == winner) {
                                    if let Some(winner_faction) = winner_agent.personality.beliefs.faction_loyalty {
                                        // Find nearest faction warehouse
                                        let warehouse_id = {
                                            let buildings = self.buildings.read();
                                            buildings.get_all_buildings()
                                                .iter()
                                                .filter(|b| {
                                                    matches!(b.building_type, BuildingType::Warehouse)
                                                        && matches!(&b.owner, BuildingOwner::Faction(f) if *f == winner_faction)
                                                        && b.is_complete()
                                                })
                                                .min_by(|a, b| {
                                                    let dist_a = a.position.distance_to(&winner_agent.position);
                                                    let dist_b = b.position.distance_to(&winner_agent.position);
                                                    dist_a.partial_cmp(&dist_b).unwrap_or(std::cmp::Ordering::Equal)
                                                })
                                                .map(|w| w.id)
                                        }; // Drop read lock
                                        
                                        // Store raided resources in warehouse
                                        if let Some(warehouse_id) = warehouse_id {
                                            let mut buildings_mut = self.buildings.write();
                                            if let Some(wh) = buildings_mut.get_building_mut(warehouse_id) {
                                                // Convert resource node type to resource type (simplified mapping)
                                                let resource_type = match resource.resource_type {
                                                    ResourceNodeType::Tree => world_sim_core::ResourceType::Wood,
                                                    ResourceNodeType::Rock => world_sim_core::ResourceType::Stone,
                                                    ResourceNodeType::IronDeposit => world_sim_core::ResourceType::Iron,
                                                    ResourceNodeType::Farm => world_sim_core::ResourceType::Food,
                                                };
                                                
                                                wh.storage.store(resource_type, raid_amount);
                                                info!("📦 Raided resources stored in {}", wh.name);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        
        // Phase 2: Job-based movement + Phase 3: Social attraction/repulsion
        let resources = self.resources.clone();
        let all_agents_for_movement = self.lifecycle.get_agents();
        
        self.lifecycle.update_living_agents(|agent| {
            // Phase 0: Special class behaviors (highest priority)
            
            // Knights follow their leader (king)
            if let Some(leader_id) = agent.leader_id {
                // Find the leader
                if let Some(leader) = all_agents_for_movement.iter().find(|a| a.id == leader_id && a.is_alive()) {
                    let dist = agent.position.distance_to(&leader.position);
                    
                    // If too far from leader, move closer
                    if dist > 5.0 {
                        let dx = leader.position.x - agent.position.x;
                        let dz = leader.position.z - agent.position.z;
                        let dist = (dx * dx + dz * dz).sqrt();
                        if dist > 0.1 {
                            // Follow at good speed
                            agent.position.x += (dx / dist) * 0.8;
                            agent.position.z += (dz / dist) * 0.8;
                            agent.state = AgentState::Following { leader: leader_id };
                        }
                        return; // Skip other behaviors
                    } else if dist < 2.0 {
                        // Too close, back off slightly
                        let dx = agent.position.x - leader.position.x;
                        let dz = agent.position.z - leader.position.z;
                        let dist = (dx * dx + dz * dz).sqrt();
                        if dist > 0.1 {
                            agent.position.x += (dx / dist) * 0.2;
                            agent.position.z += (dz / dist) * 0.2;
                        }
                        agent.state = AgentState::Following { leader: leader_id };
                        return;
                    }
                    // If at perfect distance (2-5 units), just maintain position
                    agent.state = AgentState::Following { leader: leader_id };
                }
            }
            
            // HARVESTERS in Trading state: Move to nearest market to deposit resources
            if matches!(agent.state, AgentState::Trading { .. }) && matches!(agent.job, Job::Woodcutter | Job::Miner | Job::Farmer) {
                let markets_lock = self.markets.read();
                if let Some(market) = markets_lock.find_nearest_market(&agent.position, None) {
                    let dist = agent.position.distance_to(&market.position);
                    
                    if dist > 6.0 {
                        // Travel to market
                        let dx = market.position.x - agent.position.x;
                        let dz = market.position.z - agent.position.z;
                        let dist = (dx * dx + dz * dz).sqrt();
                        if dist > 0.1 {
                            agent.position.x += (dx / dist) * 0.8; // Fast movement when full
                            agent.position.z += (dz / dist) * 0.8;
                        }
                    }
                    // Note: Deposit happens in tick_slow, not here
                }
                drop(markets_lock);
                return; // Skip other movement when going to market
            }
            
            // BUILDERS: Move to market to get resources or to construction site to deliver
            if matches!(agent.job, Job::Builder) {
                if let Some(carrying) = &agent.carrying_resources {
                    // Check if builder has ACTUAL resources (not just a target building ID)
                    let has_resources = carrying.wood > 0 || carrying.stone > 0 || carrying.iron > 0;
                    
                    if has_resources {
                        // Carrying ACTUAL resources - move to construction site to deliver
                        let buildings_lock = self.buildings.read();
                        if let Some(building) = buildings_lock.get_building(carrying.target_building_id) {
                        let dist = agent.position.distance_to(&building.position);
                        
                        if dist > 5.0 {
                            // Move towards construction site
                            let dx = building.position.x - agent.position.x;
                            let dz = building.position.z - agent.position.z;
                            let dist = (dx * dx + dz * dz).sqrt();
                            if dist > 0.1 {
                                agent.position.x += (dx / dist) * 0.7; // Fast delivery
                                agent.position.z += (dz / dist) * 0.7;
                            }
                            agent.state = AgentState::Building { 
                                building_type: format!("{:?}", building.building_type)
                            };
                        }
                        // Note: Delivery happens in tick_slow when dist < 5.0
                    }
                        drop(buildings_lock);
                        return; // Skip other movement when delivering
                    } else {
                        // Has target but NO resources (wood=0, stone=0, iron=0) - go to market!
                        let markets_lock = self.markets.read();
                        if let Some(market) = markets_lock.find_nearest_market(&agent.position, None) {
                            let dist = agent.position.distance_to(&market.position);
                            
                            if dist > 6.0 {
                                // Travel to market to buy resources
                                let dx = market.position.x - agent.position.x;
                                let dz = market.position.z - agent.position.z;
                                let dist = (dx * dx + dz * dz).sqrt();
                                if dist > 0.1 {
                                    agent.position.x += (dx / dist) * 0.7; // Move fast
                                    agent.position.z += (dz / dist) * 0.7;
                                }
                            }
                            // Switch to Moving state so we're recognized as going to market
                            agent.state = AgentState::Moving { destination: world_sim_core::GridCoord { x: 0, y: 0, z: 0 } };
                        }
                        drop(markets_lock);
                        return; // Skip other movement when going to market
                    }
                } else if matches!(agent.state, AgentState::Moving { .. }) {
                    // In Moving state - heading to market to get resources
                    let markets_lock = self.markets.read();
                    if let Some(market) = markets_lock.find_nearest_market(&agent.position, None) {
                        let dist = agent.position.distance_to(&market.position);
                        
                        if dist > 6.0 {
                            // Travel to market
                            let dx = market.position.x - agent.position.x;
                            let dz = market.position.z - agent.position.z;
                            let dist = (dx * dx + dz * dz).sqrt();
                            if dist > 0.1 {
                                agent.position.x += (dx / dist) * 0.7; // Builders move fast to get materials
                                agent.position.z += (dz / dist) * 0.7;
                            }
                        }
                        // Note: Resource pickup happens in tick_slow, not here
                    }
                    drop(markets_lock);
                    return; // Skip other movement when going to market
                }
            }
            
            // Burghers and Merchants move to markets to facilitate trade
            if matches!(agent.social_class, world_sim_agents::SocialClass::Burgher | world_sim_agents::SocialClass::Merchant) {
                // Find nearest market
                let markets_lock = self.markets.read();
                if let Some(market) = markets_lock.find_nearest_market(&agent.position, None) {
                    let dist = agent.position.distance_to(&market.position);
                    
                    if dist > 5.0 {
                        // Travel to market
                        let dx = market.position.x - agent.position.x;
                        let dz = market.position.z - agent.position.z;
                        let dist = (dx * dx + dz * dz).sqrt();
                        if dist > 0.1 {
                            agent.position.x += (dx / dist) * 0.6; // Faster than peasants
                            agent.position.z += (dz / dist) * 0.6;
                        }
                        agent.state = AgentState::Moving { 
                            destination: GridCoord::new(market.position.x as i32, 0, market.position.z as i32)
                        };
                    } else {
                        // At market, set trading state
                        agent.state = AgentState::Trading { with: world_sim_core::AgentId::new() }; // Placeholder
                    }
                    return; // Skip other movement
                }
                drop(markets_lock); // Explicitly drop the lock
            }
            
            // Soldiers patrol the community (central area by default, or faction territory if assigned)
            if matches!(agent.social_class, world_sim_agents::SocialClass::Soldier) {
                // Define patrol routes based on faction (if they have one) or central area
                let patrol_routes = if let Some(faction) = agent.personality.beliefs.faction_loyalty {
                    // Patrol faction territory
                    let base_x = if format!("{:?}", faction).contains("2cf7e1d2") { -40.0 } else { 40.0 };
                    let base_z = if format!("{:?}", faction).contains("2cf7e1d2") { -40.0 } else { 40.0 };
                    
                    vec![
                        Position::new(base_x + 30.0, 1.0, base_z + 30.0),
                        Position::new(base_x + 30.0, 1.0, base_z - 30.0),
                        Position::new(base_x - 30.0, 1.0, base_z - 30.0),
                        Position::new(base_x - 30.0, 1.0, base_z + 30.0),
                    ]
                } else {
                    // No faction - patrol central community area
                    vec![
                        Position::new(50.0, 1.0, 50.0),
                        Position::new(50.0, 1.0, -50.0),
                        Position::new(-50.0, 1.0, -50.0),
                        Position::new(-50.0, 1.0, 50.0),
                    ]
                };
                
                // Get current waypoint index
                let route_index = if let AgentState::Patrolling { route_index } = agent.state {
                    route_index
                } else {
                    0
                };
                
                let waypoint = &patrol_routes[route_index % patrol_routes.len()];
                let dist = agent.position.distance_to(waypoint);
                
                if dist > 2.0 {
                    // Move toward waypoint
                    let dx = waypoint.x - agent.position.x;
                    let dz = waypoint.z - agent.position.z;
                    let dist = (dx * dx + dz * dz).sqrt();
                    if dist > 0.1 {
                        agent.position.x += (dx / dist) * 0.5;
                        agent.position.z += (dz / dist) * 0.5;
                    }
                    agent.state = AgentState::Patrolling { route_index };
                } else {
                    // Reached waypoint, move to next
                    agent.state = AgentState::Patrolling { 
                        route_index: (route_index + 1) % patrol_routes.len()
                    };
                }
                return; // Skip other movement
            }
            
            // Phase 1: Conversation behavior (before work/combat)
            // Non-soldiers have a chance to talk to nearby agents
            if !matches!(agent.social_class, world_sim_agents::SocialClass::Soldier) {
                // Look for nearby agents to talk to
                for other in &all_agents_for_movement {
                    if other.id == agent.id || !other.is_alive() {
                        continue;
                    }
                    
                    // Check if same faction (allies) - if no factions, everyone is friendly
                    let are_friendly = match (agent.personality.beliefs.faction_loyalty, other.personality.beliefs.faction_loyalty) {
                        (Some(f1), Some(f2)) => f1 == f2, // Same faction
                        (None, None) => true,              // Both have no faction = friendly
                        _ => false,                        // One has faction, one doesn't = neutral/avoid
                    };
                    
                    if are_friendly {
                        let dist = agent.position.distance_to(&other.position);
                        
                        // If very close (within 3 units) and not already talking, start conversation
                        if dist < 3.0 && dist > 0.5 {
                            // Random chance to initiate conversation (10% per tick)
                            if rand::random::<f32>() < 0.1 {
                                // Check if both agents are idle or already talking
                                let can_talk = matches!(agent.state, AgentState::Idle) 
                                    || matches!(agent.state, AgentState::Talking { .. });
                                    
                                if can_talk {
                                    agent.state = AgentState::Talking { with: other.id };
                                    return; // Skip other behaviors while talking
                                }
                            }
                        }
                    }
                }
                
                // If already in talking state but the conversation partner is gone or far away, go back to idle
                if let AgentState::Talking { with } = agent.state {
                    let partner_found = all_agents_for_movement.iter().any(|other| {
                        other.id == with 
                            && other.is_alive() 
                            && agent.position.distance_to(&other.position) < 5.0
                    });
                    
                    if !partner_found {
                        agent.state = AgentState::Idle;
                    } else {
                        // Stay in place while talking
                        return;
                    }
                }
            }
            
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
                if nearest_enemy_dist < 25.0 {
                    // Enemy nearby! Most agents are aggressive (increased from 10.0 for more combat)
                    if agent.has_trait(world_sim_core::Trait::Brave) 
                       || matches!(agent.state, AgentState::Fighting { .. })
                       || rand::random::<f32>() < 0.7 {  // 70% of agents are aggressive
                        // Move TOWARD enemy (brave, fighting, or randomly aggressive)
                        let dx = enemy_pos.x - agent.position.x;
                        let dz = enemy_pos.z - agent.position.z;
                        let dist = (dx * dx + dz * dz).sqrt();
                        if dist > 0.1 {
                            agent.position.x += (dx / dist) * 1.0; // Increased from 0.4 for faster pursuit
                            agent.position.z += (dz / dist) * 1.0;
                        }
                        return; // Skip other movement
                    } else {
                        // Flee! (only 30% of agents flee)
                        let dx = agent.position.x - enemy_pos.x;
                        let dz = agent.position.z - enemy_pos.z;
                        let dist = (dx * dx + dz * dz).sqrt();
                        if dist > 0.1 {
                            agent.position.x = (agent.position.x + (dx / dist) * 0.6).clamp(-95.0, 95.0);
                            agent.position.z = (agent.position.z + (dz / dist) * 0.6).clamp(-95.0, 95.0);
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
                    // Miners harvest from BOTH rocks (stone) and iron deposits (iron)
                    // Prioritize iron if scarce, otherwise rocks
                    let iron_deposit = resources.find_nearest(agent.position, ResourceNodeType::IronDeposit);
                    let rock = resources.find_nearest(agent.position, ResourceNodeType::Rock);
                    
                    let target = match (iron_deposit, rock) {
                        (Some(iron), Some(rock_node)) => {
                            let iron_dist = agent.position.distance_to(&iron.position);
                            let rock_dist = agent.position.distance_to(&rock_node.position);
                            // Choose iron if it's within reasonable distance, otherwise rock
                            if iron_dist < rock_dist * 1.5 { // Prefer iron if comparable distance
                                iron
                            } else {
                                rock_node
                            }
                        },
                        (Some(iron), None) => iron,
                        (None, Some(rock_node)) => rock_node,
                        (None, None) => {
                            agent.state = AgentState::Idle;
                            return; // No resources available
                        }
                    };
                    
                    let dx = target.position.x - agent.position.x;
                    let dz = target.position.z - agent.position.z;
                    let dist = (dx * dx + dz * dz).sqrt();
                    
                    if dist > 2.0 {
                        agent.position.x += (dx / dist) * 0.3;
                        agent.position.z += (dz / dist) * 0.3;
                    } else {
                        agent.state = AgentState::Working { task: "mining".to_string() };
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
                Job::Builder => {
                    // Builders handled by dedicated logic above (lines 476-522)
                    // Skip default movement to avoid conflicts
                },
                Job::Unemployed => {
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
        // EMERGENCY: Force labor check every 10 seconds to diagnose why harvesters disappear
        static mut TICK_COUNT: u32 = 0;
        unsafe {
            TICK_COUNT += 1;
            if TICK_COUNT % 10 == 0 { // Every 10 seconds
                let agents = self.lifecycle.get_agents();
                let woodcutters = agents.iter().filter(|a| matches!(a.job, Job::Woodcutter)).count();
                let miners = agents.iter().filter(|a| matches!(a.job, Job::Miner)).count();
                let farmers = agents.iter().filter(|a| matches!(a.job, Job::Farmer)).count();
                info!("🚨 EMERGENCY JOB CHECK: Woodcutters:{} Miners:{} Farmers:{} Total:{}", 
                      woodcutters, miners, farmers, woodcutters + miners + farmers);
                
                // If harvesters are critically low, rebalance immediately
                if woodcutters + miners + farmers < 20 {
                    info!("🚨 CRITICAL: Only {} harvesters! Forcing immediate rebalance!", woodcutters + miners + farmers);
                    drop(agents);
                    self.rebalance_labor();
                }
            }
        }
        
        // Update economy
        self.economy.recalculate_prices().await;
        
        // Update dungeon master
        self.dungeon_master.tick(delta_seconds as f32).await;
        
        // Quick Win: Basic needs cycle for agents (runs every second)
        let mut rng = rand::thread_rng();
        
        self.lifecycle.update_living_agents(|agent| {
            // Simple state machine: Idle → Eating → Sleeping → Working → Idle
            // BUT: Harvesters (Woodcutter/Miner/Farmer) should be Working 80% of the time!
            let new_state = match &agent.state {
                AgentState::Idle => {
                    // HARVESTERS: Prioritize Working state
                    if matches!(agent.job, Job::Woodcutter | Job::Miner | Job::Farmer) {
                        let roll = rng.gen::<f32>();
                        if roll < 0.85 {
                            // 85% chance to start working
                            AgentState::Working { task: "gathering".to_string() }
                        } else if roll < 0.92 {
                            // 7% chance to eat
                            AgentState::Eating
                        } else {
                            // 8% chance to sleep
                            AgentState::Sleeping
                        }
                    } else if matches!(agent.job, Job::Builder) {
                        // BUILDERS: Don't randomly wander - they'll be assigned to buildings below
                        AgentState::Idle
                    } else {
                        // OTHER NON-HARVESTERS: Original state machine
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
                    }
                },
                AgentState::Eating => {
                    // HARVESTERS: Quickly return to work after eating
                    if matches!(agent.job, Job::Woodcutter | Job::Miner | Job::Farmer) {
                        // 90% chance to go straight back to working
                        if rng.gen::<f32>() < 0.9 {
                            AgentState::Working { task: "gathering".to_string() }
                        } else {
                            AgentState::Sleeping
                        }
                    } else if matches!(agent.job, Job::Builder) {
                        // BUILDERS: Always return to Idle after eating (ready for assignment)
                        AgentState::Idle
                    } else {
                        // Non-harvesters: original logic
                        if rng.gen::<f32>() < 0.5 {
                            AgentState::Sleeping
                        } else {
                            AgentState::Idle
                        }
                    }
                },
                AgentState::Sleeping => {
                    // HARVESTERS: Immediately return to work after sleeping
                    if matches!(agent.job, Job::Woodcutter | Job::Miner | Job::Farmer) {
                        AgentState::Working { task: "gathering".to_string() }
                    } else if matches!(agent.job, Job::Builder) {
                        // BUILDERS: Always return to Idle after sleeping (ready for assignment)
                        AgentState::Idle
                    } else {
                        // Non-harvesters: go to idle
                        AgentState::Idle
                    }
                },
                AgentState::Working { .. } => {
                    // HARVESTERS: Stay in working longer (only 15% chance to stop)
                    if matches!(agent.job, Job::Woodcutter | Job::Miner | Job::Farmer) {
                        if rng.gen::<f32>() < 0.15 {
                            AgentState::Idle
                        } else {
                            // Stay working!
                            AgentState::Working { task: "gathering".to_string() }
                        }
                    } else if matches!(agent.social_class, world_sim_agents::SocialClass::Burgher | world_sim_agents::SocialClass::Merchant) {
                        // BURGHERS/MERCHANTS: Stay in Trading state (they live at markets)
                        AgentState::Trading { with: agent.id } // Placeholder ID
                    } else {
                        // Other non-harvesters: cycle back to idle
                        AgentState::Idle
                    }
                },
                _ => agent.state.clone(),
            };
            
            agent.state = new_state;
        }); // update_living_agents completes here, drops lock
        
        // BUILDER ASSIGNMENT SYSTEM: Assign idle builders to incomplete buildings
        self.assign_builders_to_buildings();
        
        // BURGHER BANKING SYSTEM: Process loans and market facilitation
        self.process_burgher_activities();
        
        // RESOURCE REGENERATION: Natural growth (trees regrow, farms produce, etc.)
        self.resources.regenerate();
        
        // ECONOMIC SYSTEM: Resource harvesting stores in agent inventory
        let mut agents = self.lifecycle.get_agents_mut(); // Now safe to acquire lock
        let resource_nodes = self.resources.get_nodes();
        
        // Removed spam log - working agents count and harvest counters
        
        for agent in agents.iter_mut() {
            // Only harvest if agent is Working near a resource node
            if matches!(agent.state, AgentState::Working { .. }) {
                
                // Miners can harvest from BOTH rocks (stone) and iron deposits (iron)
                let harvest_types: Vec<ResourceNodeType> = match agent.job {
                    Job::Woodcutter => vec![ResourceNodeType::Tree],
                    Job::Miner => vec![ResourceNodeType::Rock, ResourceNodeType::IronDeposit],
                    Job::Farmer => vec![ResourceNodeType::Farm],
                    _ => vec![],
                };
                
                if !harvest_types.is_empty() {
                    // Find nearest resource of the right types (miners can harvest multiple types)
                    let nearest_node = resource_nodes.iter()
                        .filter(|n| harvest_types.contains(&n.resource_type) && n.quantity > 0)
                        .min_by(|a, b| {
                            let dist_a = a.position.distance_to(&agent.position);
                            let dist_b = b.position.distance_to(&agent.position);
                            dist_a.partial_cmp(&dist_b).unwrap_or(std::cmp::Ordering::Equal)
                        });
                    
                    // Removed counter - no resource tracking
                    
                    if let Some(node) = nearest_node {
                        let dist = node.position.distance_to(&agent.position);
                        
                        // Removed counter - too far tracking
                        
                        // Only harvest if close enough (< 3.0 units)
                        if dist < 3.0 {
                            // Harvest 5 units per second (if capacity allows)
                            let harvest_amount = 5;
                            
                            // Check carrying capacity
                            if agent.can_carry_more(harvest_amount) {
                                if let Some(harvested) = self.resources.harvest(node.id, harvest_amount) {
                                    
                                    // Convert node type to resource type and store in inventory
                                    let resource_type = match node.resource_type {
                                        ResourceNodeType::Tree => world_sim_core::ResourceType::Wood,
                                        ResourceNodeType::Rock => world_sim_core::ResourceType::Stone,
                                        ResourceNodeType::Farm => world_sim_core::ResourceType::Food,
                                        ResourceNodeType::IronDeposit => world_sim_core::ResourceType::Iron,
                                    };
                                    
                                    *agent.inventory.entry(resource_type).or_insert(0) += harvested;
                                    
                                    // Set just_harvested for visualization (with current sim time)
                                    agent.just_harvested = Some((resource_type, harvested, self.sim_time.seconds));
                                    
                                    // Removed spam log - harvest inventory updates
                                }
                            } else {
                                // Inventory full - transition to Trading state to sell at market
                                agent.state = AgentState::Trading { with: agent.id }; // Placeholder - trading with self means "going to market"
                                info!("📦 {} inventory full ({} total items), heading to market to sell", 
                                      agent.name, agent.current_inventory_weight());
                            }
                        }
                    }
                }
            }
        }
        
        // Removed spam log - harvest summary
        
        drop(agents); // CRITICAL: Drop write lock before next section
        
        // ECONOMIC SYSTEM: Harvester deposits to market (direct transfer)
        let mut markets_lock = self.markets.write();
        let mut agents_mut = self.lifecycle.get_agents_mut();
        
        for agent in agents_mut.iter_mut() {
            // Only process agents at markets (in Trading state)
            if matches!(agent.state, AgentState::Trading { .. }) {
                // HARVESTERS: Direct deposit into market (like a warehouse)
                if matches!(agent.job, Job::Woodcutter | Job::Miner | Job::Farmer) {
                    // Find nearest market to deposit resources
                    if let Some(market) = markets_lock.find_nearest_market(&agent.position, None) {
                        let dist = agent.position.distance_to(&market.position);
                        
                        // Debug: Log distance check
                        if dist >= 6.0 {
                            // Too far - still traveling (movement logic in tick_fast will handle it)
                            continue;
                        }
                        
                        // At market! Deposit resources
                        // (Removed spam log)
                        
                        let market_id = market.id;
                        
                        if let Some(market) = markets_lock.get_market_mut(market_id) {
                            let mut deposited_total = 0;
                            let mut total_earned = 0.0;
                            
                            // SELL all resources from inventory to market (CURRENCY EXCHANGE!)
                            for (resource_type, quantity) in &agent.inventory {
                                if *quantity > 0 {
                                    // Get or create market good with price
                                    let base_price = match resource_type {
                                        world_sim_core::ResourceType::Food => 10.0,
                                        world_sim_core::ResourceType::Wood => 5.0,
                                        world_sim_core::ResourceType::Stone => 3.0,
                                        world_sim_core::ResourceType::Iron => 15.0,
                                        _ => 5.0,
                                    };
                                    
                                    // Calculate selling price (90% of base price - market takes 10% fee)
                                    let sell_price = base_price * 0.9;
                                    let earnings = sell_price * (*quantity as f64);
                                    
                                    // Add to market inventory
                                    market.inventory.entry(*resource_type)
                                        .and_modify(|good| good.quantity += quantity)
                                        .or_insert_with(|| {
                                            use world_sim_societal::MarketGood;
                                            MarketGood {
                                                resource_type: *resource_type,
                                                quantity: *quantity,
                                                base_price,
                                                current_price: base_price,
                                                sellers: vec![],
                                            }
                                        });
                                    
                                    deposited_total += quantity;
                                    total_earned += earnings;
                                    
                                    let market_quantity = market.inventory.get(resource_type).map(|g| g.quantity).unwrap_or(0);
                                    info!("💰 {} SOLD {} {:?} to {} for {:.1} gold (market now has: {})", 
                                          agent.name, quantity, resource_type, market.name, earnings, market_quantity);
                                }
                            }
                            
                            // PAY the agent for their goods
                            if total_earned > 0.0 {
                                agent.wallet += total_earned;
                                // Record transaction in currency system
                                self.currency.write().record_transaction(total_earned);
                                // Set just_transacted for visualization (gold earned)
                                agent.just_transacted = Some((world_sim_agents::TransactionType::Sold, total_earned, self.sim_time.seconds));
                                info!("💵 {} earned {:.1} gold total, wallet now: {:.1}", 
                                      agent.name, total_earned, agent.wallet);
                            }
                            
                            // Always return to Working state after arriving at market
                            if deposited_total > 0 {
                                // Clear agent's inventory after deposit
                                agent.inventory.clear();
                                info!("✅ {} deposited {} items total, returning to work", agent.name, deposited_total);
                            } else {
                                // Empty inventory - go back to harvesting (no log to avoid spam)
                            }
                            
                            // Return to Working state to harvest more
                            agent.state = AgentState::Working { task: "gathering".to_string() };
                        }
                    }
                    continue; // Skip normal trading logic for harvesters
                }
            }
        }
        drop(agents_mut); // Drop mutable lock before read-only access
        
        // ECONOMIC SYSTEM: Agent trading behavior at markets (non-harvesters)
        let agents = self.lifecycle.get_agents();
        
        for agent in agents.iter() {
            // Only trade if at a market (in Trading state)
            if matches!(agent.state, AgentState::Trading { .. }) {
                // Skip harvesters (already handled above)
                if matches!(agent.job, Job::Woodcutter | Job::Miner | Job::Farmer) {
                    continue;
                }
                
                // NON-HARVESTERS: Normal trading with orders
                // Find nearest market
                if let Some(market) = markets_lock.find_nearest_market(&agent.position, None) {
                    let dist = agent.position.distance_to(&market.position);
                    
                    // Only trade if close enough (< 6.0 units)
                    if dist < 6.0 {
                        let market_id = market.id;
                        
                        // Find the market to place orders
                        if let Some(market) = markets_lock.get_market_mut(market_id) {
                            // BUY what they need
                            for (resource_type, needed_amount) in &agent.needs {
                                let current_amount = agent.inventory.get(resource_type).copied().unwrap_or(0);
                                
                                if current_amount < *needed_amount {
                                    let deficit = needed_amount - current_amount;
                                    
                                    // Calculate max price willing to pay (higher for essentials)
                                    let base_price = match resource_type {
                                        world_sim_core::ResourceType::Food => 10.0,
                                        world_sim_core::ResourceType::Wood => 5.0,
                                        world_sim_core::ResourceType::Stone => 3.0,
                                        world_sim_core::ResourceType::Iron => 15.0,
                                        _ => 5.0,
                                    };
                                    
                                    // Pay more if desperate (low inventory)
                                    let desperation = if current_amount == 0 { 2.0 } else { 1.5 };
                                    let max_price = base_price * desperation;
                                    
                                    // Only buy if can afford
                                    if agent.wallet >= max_price * deficit as f64 {
                                        use world_sim_societal::{TradeOrder, OrderType};
                                        
                                        market.place_buy_order(TradeOrder {
                                            id: uuid::Uuid::new_v4(),
                                            agent_id: agent.id,
                                            resource: *resource_type,
                                            quantity: deficit,
                                            price_per_unit: max_price,
                                            order_type: OrderType::Buy,
                                        });
                                    }
                                }
                            }
                            
                            // SELL excess inventory
                            for (resource_type, quantity) in &agent.inventory {
                                let needed = agent.needs.get(resource_type).copied().unwrap_or(0);
                                
                                // Keep 2x what needed, sell the rest
                                if *quantity > needed * 2 {
                                    let excess = quantity - (needed * 2);
                                    
                                    // Calculate asking price
                                    let base_price = match resource_type {
                                        world_sim_core::ResourceType::Food => 10.0,
                                        world_sim_core::ResourceType::Wood => 5.0,
                                        world_sim_core::ResourceType::Stone => 3.0,
                                        world_sim_core::ResourceType::Iron => 15.0,
                                        _ => 5.0,
                                    };
                                    
                                    // Merchants sell for profit
                                    let profit_margin = if matches!(agent.social_class, world_sim_agents::SocialClass::Merchant | world_sim_agents::SocialClass::Burgher) {
                                        1.2 // 20% markup
                                    } else {
                                        1.0 // At base price
                                    };
                                    
                                    let asking_price = base_price * profit_margin;
                                    
                                    use world_sim_societal::{TradeOrder, OrderType};
                                    
                                    market.place_sell_order(TradeOrder {
                                        id: uuid::Uuid::new_v4(),
                                        agent_id: agent.id,
                                        resource: *resource_type,
                                        quantity: excess,
                                        price_per_unit: asking_price,
                                        order_type: OrderType::Sell,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
        drop(agents); // CRITICAL: Drop read lock before getting write lock
        
        // ECONOMIC SYSTEM: Match orders and execute trades at all markets
        let mut agents_mut = self.lifecycle.get_agents_mut();
        let mut currency_lock = self.currency.write();
        
        for market in markets_lock.get_all_markets_mut() {
            // Match buy and sell orders
            let trades = market.match_orders();
            
            // Execute each trade
            for trade in trades {
                // Find buyer and seller
                if let Some(buyer) = agents_mut.iter_mut().find(|a| a.id == trade.buyer_id) {
                    let total_cost = trade.price_per_unit * trade.quantity as f64;
                    
                    // Check if buyer can still afford (might have spent money already this tick)
                    if buyer.wallet >= total_cost {
                        // Deduct money from buyer
                        buyer.wallet -= total_cost;
                        
                        // Add resources to buyer inventory
                        *buyer.inventory.entry(trade.resource).or_insert(0) += trade.quantity;
                        
                        // Now find seller and complete trade
                        if let Some(seller) = agents_mut.iter_mut().find(|a| a.id == trade.seller_id) {
                            // Add money to seller
                            seller.wallet += total_cost;
                            
                            // Remove resources from seller inventory
                            if let Some(seller_amount) = seller.inventory.get_mut(&trade.resource) {
                                *seller_amount = seller_amount.saturating_sub(trade.quantity);
                            }
                            
                            // Record transaction in currency system
                            currency_lock.record_transaction(total_cost);
                            
                            info!("💰 Trade executed: {} {} for {:.2} ({:.2}/unit)", 
                                  trade.quantity, format!("{:?}", trade.resource), total_cost, trade.price_per_unit);
                        }
                    }
                }
            }
            
            // Update market prices based on supply/demand
            market.update_prices();
        }
        
        // INTER-MARKET TRADE: Balance inventories across markets
        // This prevents iron hoarding in one market
        self.balance_market_inventories(&mut markets_lock);
        
        drop(currency_lock);
        drop(agents_mut); // CRITICAL: Drop write lock from trade execution
        drop(markets_lock);
        
        // ECONOMIC SYSTEM: Wage system (pay workers every simulated hour)
        let elapsed = self.wage_timer.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        
        if elapsed >= 60 { // Pay wages every 60 seconds (1 simulated hour)
            self.wage_timer.store(0, std::sync::atomic::Ordering::Relaxed);
            
            let mut agents_mut = self.lifecycle.get_agents_mut();
            let mut currency_mut = self.currency.write();
            
            for agent in agents_mut.iter_mut() {
                // Calculate wage based on job
                let wage = match agent.job {
                    Job::Farmer => 5.0,
                    Job::Woodcutter => 6.0,
                    Job::Miner => 7.0,
                    Job::Builder => 8.0,
                    Job::Unemployed => {
                        // Social classes get stipends
                        match agent.social_class {
                            world_sim_agents::SocialClass::King => 50.0,
                            world_sim_agents::SocialClass::Noble => 20.0,
                            world_sim_agents::SocialClass::Knight => 15.0,
                            world_sim_agents::SocialClass::Soldier => 10.0,
                            world_sim_agents::SocialClass::Merchant => 12.0,
                            world_sim_agents::SocialClass::Cleric => 8.0,
                            _ => 0.0,
                        }
                    }
                };
                
                if wage > 0.0 {
                    agent.wallet += wage;
                    currency_mut.mint_currency(wage); // Creates new money (inflation)
                }
            }
            
            info!("💵 Wages paid to {} workers", agents_mut.len());
        }
        
        // Update building construction progress
        let agents = self.lifecycle.get_agents();
        
        // Collect buildings that need construction first (avoid borrow conflicts)
        let incomplete_buildings: Vec<(Uuid, Position)> = {
            let buildings = self.buildings.read();
            buildings.get_all_buildings()
                .iter()
                .filter(|b| !b.is_complete())
                .map(|b| (b.id, b.position))
                .collect()
        }; // Drop read lock
        
        // RESOURCE-BASED CONSTRUCTION: Builders deliver and consume resources
        if !incomplete_buildings.is_empty() {
            let mut agents_mut = self.lifecycle.get_agents_mut();
            
            for (building_id, building_pos) in incomplete_buildings {
                let mut buildings_write = self.buildings.write();
                
                if let Some(building) = buildings_write.get_building_mut(building_id) {
                    // Process builders at this building
                    for agent in agents_mut.iter_mut() {
                    if !matches!(agent.job, Job::Builder) {
                        continue;
                    }
                    
                    let dist_to_building = agent.position.distance_to(&building_pos);
                    
                    // Builder resource delivery and construction logic
                    if let Some(carrying) = &agent.carrying_resources {
                        // Carrying resources - deliver them
                        if carrying.target_building_id == building_id && dist_to_building < 5.0 {
                            // Deliver resources to building
                            if carrying.wood > 0 {
                                building.add_resources(world_sim_core::ResourceType::Wood, carrying.wood);
                            }
                            if carrying.stone > 0 {
                                building.add_resources(world_sim_core::ResourceType::Stone, carrying.stone);
                            }
                            if carrying.iron > 0 {
                                building.add_resources(world_sim_core::ResourceType::Iron, carrying.iron);
                            }
                            
                            info!("🚚 Builder delivered {} wood, {} stone, {} iron to {}", 
                                  carrying.wood, carrying.stone, carrying.iron, building.name);
                            
                            agent.carrying_resources = None;
                        }
                    } else if matches!(agent.state, AgentState::Moving { .. } | AgentState::Building { .. }) {
                        // Builder has target but no resources yet - check if this is their assigned building
                        if let Some(carrying) = &agent.carrying_resources {
                            if carrying.target_building_id == building_id {
                                // This IS the builder's assigned building!
                                let has_resources = carrying.wood > 0 || carrying.stone > 0 || carrying.iron > 0;
                                info!("🔍 BUILDER DEBUG: {} processing for {} | Has resources: {} (W:{} S:{} I:{}) | Dist to building: {:.1}", 
                                      agent.name, building.name, has_resources, carrying.wood, carrying.stone, carrying.iron, dist_to_building);
                                
                                let remaining = building.remaining_resources();
                                
                                if !remaining.is_empty() {
                                    // Building needs resources - check if builder is at a market
                                    let markets = self.markets.read();
                                    
                                    if let Some(market) = markets.find_nearest_market(&agent.position, None) {
                                        let dist_to_market = agent.position.distance_to(&market.position);
                                        info!("🔍 {} | Nearest market: {} | Dist: {:.1} | Remaining needs: {:?}", 
                                              agent.name, market.name, dist_to_market, remaining);
                                        
                                        // If close to market, pick up resources for THIS specific building
                                        if dist_to_market < 6.0 {
                                            info!("🛒 {} AT MARKET {} - attempting to buy resources!", agent.name, market.name);
                                    let market_id = market.id;
                                    drop(markets); // Drop read lock before acquiring write lock
                                    
                                    let mut markets_mut = self.markets.write();
                                    if let Some(market) = markets_mut.get_market_mut(market_id) {
                                        let mut resources_to_carry = world_sim_agents::BuildingResources {
                                            wood: 0,
                                            stone: 0,
                                            iron: 0,
                                            target_building_id: building_id,
                                        };
                                        
                                        let mut total_cost = 0.0;
                                        let mut found_but_cant_afford = false;
                                        
                                        // Get building's construction fund
                                        let building_fund = building.construction_fund;
                                        
                                        // BUY resources from market using BUILDING'S FUND (not builder's wallet)
                                        if let Some(needed_wood) = remaining.get(&world_sim_core::ResourceType::Wood) {
                                            if let Some(market_good) = market.inventory.get_mut(&world_sim_core::ResourceType::Wood) {
                                                let take = (*needed_wood).min(20).min(market_good.quantity);
                                                if take > 0 {
                                                    let cost = market_good.current_price * (take as f64);
                                                    // Check if BUILDING has sufficient funds
                                                    if building_fund >= cost {
                                                        market_good.quantity -= take;
                                                        resources_to_carry.wood = take;
                                                        total_cost += cost;
                                                        info!("🪵 Builder {} BOUGHT {} wood from {} for {:.1} gold (using building fund)", 
                                                              agent.name, take, market.name, cost);
                                                    } else {
                                                        found_but_cant_afford = true;
                                                        info!("💸 Building fund insufficient for {} wood ({:.1} gold needed, fund has {:.1})", 
                                                              take, cost, building_fund);
                                                    }
                                                }
                                            }
                                        }
                                        
                                        if let Some(needed_stone) = remaining.get(&world_sim_core::ResourceType::Stone) {
                                            if let Some(market_good) = market.inventory.get_mut(&world_sim_core::ResourceType::Stone) {
                                                let take = (*needed_stone).min(20).min(market_good.quantity);
                                                if take > 0 {
                                                    let cost = market_good.current_price * (take as f64);
                                                    // Check remaining building fund (after previous purchases)
                                                    if building_fund - total_cost >= cost {
                                                        market_good.quantity -= take;
                                                        resources_to_carry.stone = take;
                                                        total_cost += cost;
                                                        info!("🪨 Builder {} BOUGHT {} stone from {} for {:.1} gold (using building fund)", 
                                                              agent.name, take, market.name, cost);
                                                    } else {
                                                        found_but_cant_afford = true;
                                                        info!("💸 Building fund insufficient for {} stone ({:.1} gold needed, fund has {:.1} remaining)", 
                                                              take, cost, building_fund - total_cost);
                                                    }
                                                }
                                            }
                                        }
                                        
                                        if let Some(needed_iron) = remaining.get(&world_sim_core::ResourceType::Iron) {
                                            if let Some(market_good) = market.inventory.get_mut(&world_sim_core::ResourceType::Iron) {
                                                let take = (*needed_iron).min(10).min(market_good.quantity);
                                                if take > 0 {
                                                    let cost = market_good.current_price * (take as f64);
                                                    // Check remaining building fund (after previous purchases)
                                                    if building_fund - total_cost >= cost {
                                                        market_good.quantity -= take;
                                                        resources_to_carry.iron = take;
                                                        total_cost += cost;
                                                        info!("⛏️ Builder {} BOUGHT {} iron from {} for {:.1} gold (using building fund)", 
                                                              agent.name, take, market.name, cost);
                                                    } else {
                                                        found_but_cant_afford = true;
                                                        info!("💸 Building fund insufficient for {} iron ({:.1} gold needed, fund has {:.1} remaining)", 
                                                              take, cost, building_fund - total_cost);
                                                    }
                                                }
                                            }
                                        }
                                        
                                        if total_cost > 0.0 {
                                            // Record transaction in currency system
                                            self.currency.write().record_transaction(total_cost);
                                            // Set just_transacted for visualization (gold spent)
                                            agent.just_transacted = Some((world_sim_agents::TransactionType::Bought, total_cost, self.sim_time.seconds));
                                            
                                            // Deduct cost from building's construction fund
                                            building.construction_fund -= total_cost;
                                            
                                            info!("💵 Builder {} spent {:.1} gold from building fund (fund remaining: {:.1})", 
                                                  agent.name, total_cost, building.construction_fund);
                                        }
                                        
                                        // If we picked up any resources, start carrying them
                                        if resources_to_carry.wood > 0 || resources_to_carry.stone > 0 || resources_to_carry.iron > 0 {
                                            agent.carrying_resources = Some(resources_to_carry.clone());
                                            agent.state = AgentState::Building { 
                                                building_type: format!("{:?}", building.building_type)
                                            };
                                            info!("✅ Builder {} loaded {} wood, {} stone, {} iron for {} - heading to site!", 
                                                  agent.name, resources_to_carry.wood, resources_to_carry.stone, 
                                                  resources_to_carry.iron, building.name);
                                        } else {
                                            // Couldn't get resources - silent (no log spam)
                                            // Fund replenishment happens in tick_very_slow (every 60s)
                                        }
                                    }
                                } else {
                                    // Move towards market to pick up resources
                                    agent.state = AgentState::Moving { 
                                        destination: GridCoord::new(market.position.x as i32, 0, market.position.z as i32)
                                    };
                                }
                            } else {
                                info!("⚠️ Builder {}: No markets available to get resources for {}", 
                                      agent.name, building.name);
                            }
                                    }
                                }
                            }
                        }
                    
                    // Work on construction if at site (with resource consumption)
                    if dist_to_building < 5.0 && agent.carrying_resources.is_none() {
                        let progress_per_builder = 0.02; // 2% per builder per second
                        
                        if building.construct_with_resources(progress_per_builder) {
                            agent.state = AgentState::Building { 
                                building_type: format!("{:?}", building.building_type)
                            };
                            
                            if building.is_complete() {
                                info!("🏗️ Building completed: {}", building.name);
                            }
                        } else {
                            // Can't construct - need more resources
                            agent.state = AgentState::Idle;
                        }
                    }
                    }
                }
                drop(buildings_write);
            }
            drop(agents_mut); // Explicitly drop agents lock after all buildings processed
        }
        
        // CRITICAL: Update world state for visualizer EVERY SECOND (not every 60 seconds!)
        self.sync_world_state_to_api();
        
        Ok(())
    }
    
    /// Balance inventories across markets (merchants redistribute goods)
    fn balance_market_inventories(&self, markets: &mut parking_lot::RwLockWriteGuard<MarketSystem>) {
        // For each resource type, find imbalances and redistribute
        let resource_types = vec![
            world_sim_core::ResourceType::Wood,
            world_sim_core::ResourceType::Stone,
            world_sim_core::ResourceType::Iron,
            world_sim_core::ResourceType::Food,
        ];
        
        for resource_type in resource_types {
            // Calculate average inventory across all markets
            let all_markets = markets.get_all_markets();
            let mut total_inventory = 0u32;
            let mut market_count = 0;
            
            for market in all_markets {
                if let Some(good) = market.inventory.get(&resource_type) {
                    total_inventory += good.quantity;
                    market_count += 1;
                }
            }
            
            if market_count == 0 {
                continue;
            }
            
            let average_inventory = total_inventory / market_count as u32;
            
            // Find markets that are WAY above or below average
            let mut surplus_markets: Vec<(uuid::Uuid, u32)> = Vec::new(); // (id, surplus amount)
            let mut deficit_markets: Vec<(uuid::Uuid, u32)> = Vec::new(); // (id, deficit amount)
            
            for market in markets.get_all_markets() {
                if let Some(good) = market.inventory.get(&resource_type) {
                    if good.quantity > average_inventory + 50 {
                        // Has surplus
                        let surplus = good.quantity - average_inventory;
                        surplus_markets.push((market.id, surplus));
                    } else if good.quantity + 50 < average_inventory {
                        // Has deficit
                        let deficit = average_inventory - good.quantity;
                        deficit_markets.push((market.id, deficit));
                    }
                }
            }
            
            // Transfer goods from surplus to deficit markets
            // Merchants do this automatically (simulated)
            if !surplus_markets.is_empty() && !deficit_markets.is_empty() {
                // Collect transfer operations to avoid double mutable borrow
                let mut transfers: Vec<(uuid::Uuid, uuid::Uuid, u32)> = Vec::new(); // (from, to, amount)
                let mut remaining_surplus = surplus_markets.clone();
                
                for (deficit_market_id, deficit_amount) in &deficit_markets {
                    if remaining_surplus.is_empty() {
                        break;
                    }
                    
                    let (surplus_market_id, surplus_amount) = remaining_surplus[0];
                    let transfer_amount = (deficit_amount / 2).min(surplus_amount / 2).min(30); // Transfer up to 30 units
                    
                    if transfer_amount > 0 {
                        transfers.push((surplus_market_id, *deficit_market_id, transfer_amount));
                        
                        // Update remaining surplus
                        remaining_surplus[0].1 -= transfer_amount;
                        if remaining_surplus[0].1 == 0 {
                            remaining_surplus.remove(0);
                        }
                    }
                }
                
                // Execute transfers (now we can do sequential mutable borrows)
                for (from_id, to_id, amount) in transfers {
                    // Get names first (for logging)
                    let from_name = markets.get_all_markets().iter()
                        .find(|m| m.id == from_id)
                        .map(|m| m.name.clone())
                        .unwrap_or_else(|| "Unknown".to_string());
                    let to_name = markets.get_all_markets().iter()
                        .find(|m| m.id == to_id)
                        .map(|m| m.name.clone())
                        .unwrap_or_else(|| "Unknown".to_string());
                    
                    // Remove from source
                    let mut transfer_successful = false;
                    if let Some(from_market) = markets.get_market_mut(from_id) {
                        if let Some(good) = from_market.inventory.get_mut(&resource_type) {
                            if good.quantity >= amount {
                                good.quantity -= amount;
                                transfer_successful = true;
                            }
                        }
                    }
                    
                    // Add to destination (only if removal succeeded)
                    if transfer_successful {
                        if let Some(to_market) = markets.get_market_mut(to_id) {
                            to_market.inventory
                                .entry(resource_type)
                                .and_modify(|g| g.quantity += amount)
                                .or_insert_with(|| world_sim_societal::MarketGood {
                                    resource_type,
                                    quantity: amount,
                                    base_price: match resource_type {
                                        world_sim_core::ResourceType::Wood => 5.0,
                                        world_sim_core::ResourceType::Stone => 3.0,
                                        world_sim_core::ResourceType::Iron => 15.0,
                                        world_sim_core::ResourceType::Food => 10.0,
                                        _ => 5.0,
                                    },
                                    current_price: match resource_type {
                                        world_sim_core::ResourceType::Wood => 5.0,
                                        world_sim_core::ResourceType::Stone => 3.0,
                                        world_sim_core::ResourceType::Iron => 15.0,
                                        world_sim_core::ResourceType::Food => 10.0,
                                        _ => 5.0,
                                    },
                                    sellers: Vec::new(), // No specific seller for inter-market transfers
                                });
                            
                            info!("🚚 Merchants transferred {} {:?} from {} to {} (balancing markets)", 
                                  amount, resource_type, from_name, to_name);
                        }
                    }
                }
            }
        }
    }
    
    /// Replenish construction funds if prices rose (runs every 60s)
    fn replenish_construction_funds(&self) {
        // Step 1: Identify buildings that need more funding
        let buildings_lock = self.buildings.read();
        let all_buildings = buildings_lock.get_all_buildings();
        
        let mut funding_requests: Vec<(uuid::Uuid, String, world_sim_world::BuildingOwner, f64, f64)> = Vec::new();
        
        for building in all_buildings {
            if building.construction_progress < 1.0 {
                // Calculate remaining resource cost at CURRENT market prices
                let remaining = building.remaining_resources();
                let estimated_cost: f64 = remaining.iter()
                    .map(|(resource_type, qty)| {
                        self.get_market_price(*resource_type) * (*qty as f64)
                    })
                    .sum();
                
                // If construction fund is less than 50% of estimated remaining cost, request funding
                if building.construction_fund < estimated_cost * 0.5 {
                    let additional_needed = estimated_cost * 2.0; // 200% buffer
                    funding_requests.push((
                        building.id,
                        building.name.clone(),
                        building.owner.clone(),
                        building.construction_fund,
                        additional_needed,
                    ));
                }
            }
        }
        
        drop(buildings_lock); // CRITICAL: Drop read lock before getting agents lock
        
        if funding_requests.is_empty() {
            return;
        }
        
        // Step 2: Collect funding from owners (agents lock)
        let mut agents = self.lifecycle.get_agents_mut();
        let mut successful_funding: Vec<(uuid::Uuid, f64)> = Vec::new();
        
        for (building_id, building_name, owner, current_fund, additional_needed) in funding_requests {
            let mut funded_amount = 0.0;
            
            match owner {
                world_sim_world::BuildingOwner::Agent(owner_id) => {
                    // Get from agent who owns the building
                    if let Some(owner_agent) = agents.iter_mut().find(|a| a.id == owner_id) {
                        if owner_agent.wallet >= additional_needed {
                            owner_agent.wallet -= additional_needed;
                            funded_amount = additional_needed;
                            info!("💰 {} provides {:.1} gold to replenish {} fund (prices rose)", 
                                  owner_agent.name, additional_needed, building_name);
                        }
                    }
                },
                world_sim_world::BuildingOwner::Public => {
                    // Get from nobles/kings (public works)
                    let noble_ids: Vec<_> = agents.iter()
                        .filter(|a| matches!(a.social_class, 
                            world_sim_agents::SocialClass::King | 
                            world_sim_agents::SocialClass::Noble))
                        .map(|a| a.id)
                        .collect();
                    
                    if !noble_ids.is_empty() {
                        let per_noble = additional_needed / noble_ids.len() as f64;
                        let mut contributed = 0;
                        
                        for noble_id in noble_ids {
                            if let Some(noble) = agents.iter_mut().find(|a| a.id == noble_id) {
                                if noble.wallet >= per_noble {
                                    noble.wallet -= per_noble;
                                    funded_amount += per_noble;
                                    contributed += 1;
                                }
                            }
                        }
                        
                        if contributed > 0 {
                            info!("💰 {} nobles provide {:.1} gold to replenish {} fund (public building)", 
                                  contributed, funded_amount, building_name);
                        }
                    }
                },
                _ => {}
            }
            
            if funded_amount > 0.0 {
                successful_funding.push((building_id, funded_amount));
            }
        }
        
        drop(agents); // CRITICAL: Drop agents lock before getting buildings lock
        
        // Step 3: Apply funding to buildings (buildings lock)
        if !successful_funding.is_empty() {
            let mut buildings_write = self.buildings.write();
            for (building_id, funded_amount) in successful_funding {
                if let Some(building) = buildings_write.get_building_mut(building_id) {
                    let old_fund = building.construction_fund;
                    building.construction_fund += funded_amount;
                    info!("✅ {} fund replenished: {:.1} → {:.1} gold", 
                          building.name, old_fund, building.construction_fund);
                }
            }
            drop(buildings_write);
        }
    }
    
    /// Collect taxes from population to fund public works
    fn collect_taxes(&self) {
        let mut agents = self.lifecycle.get_agents_mut();
        
        // Tax rate: 5% of wallet for non-nobles
        let tax_rate = 0.05;
        let mut total_collected = 0.0;
        let mut taxpayers = 0;
        
        // Collect from peasants, burghers, merchants, clerics
        let taxable_classes = [
            world_sim_agents::SocialClass::Peasant,
            world_sim_agents::SocialClass::Burgher,
            world_sim_agents::SocialClass::Merchant,
            world_sim_agents::SocialClass::Cleric,
            world_sim_agents::SocialClass::Soldier,
        ];
        
        for agent in agents.iter_mut() {
            if taxable_classes.contains(&agent.social_class) && agent.wallet > 50.0 {
                let tax_amount = agent.wallet * tax_rate;
                agent.wallet -= tax_amount;
                total_collected += tax_amount;
                taxpayers += 1;
            }
        }
        
        if total_collected > 0.0 {
            // Distribute tax revenue to nobles and kings for public works
            let mut nobles: Vec<world_sim_core::AgentId> = Vec::new();
            
            for agent in agents.iter() {
                if matches!(agent.social_class, world_sim_agents::SocialClass::King | world_sim_agents::SocialClass::Noble) {
                    nobles.push(agent.id);
                }
            }
            
            if !nobles.is_empty() {
                let per_noble = total_collected / nobles.len() as f64;
                
                for agent in agents.iter_mut() {
                    if nobles.contains(&agent.id) {
                        agent.wallet += per_noble;
                    }
                }
                
                info!("💰 Tax collection: {:.1} gold from {} taxpayers → {} nobles ({:.1} each)", 
                      total_collected, taxpayers, nobles.len(), per_noble);
            }
        }
    }
    
    /// Get current market price for a resource (averaged across all markets)
    fn get_market_price(&self, resource_type: world_sim_core::ResourceType) -> f64 {
        let markets = self.markets.read();
        let all_markets = markets.get_all_markets();
        
        let mut total_price = 0.0;
        let mut count = 0;
        
        for market in all_markets {
            if let Some(good) = market.inventory.get(&resource_type) {
                total_price += good.current_price;
                count += 1;
            }
        }
        
        if count > 0 {
            total_price / count as f64
        } else {
            // Fallback to defaults if no market data
            match resource_type {
                world_sim_core::ResourceType::Wood => 5.0,
                world_sim_core::ResourceType::Stone => 3.0,
                world_sim_core::ResourceType::Iron => 15.0,
                world_sim_core::ResourceType::Food => 10.0,
                _ => 5.0,
            }
        }
    }
    
    /// Burgher banking and market facilitation system
    fn process_burgher_activities(&self) {
        let mut agents = self.lifecycle.get_agents_mut();
        
        // Find Burghers/Merchants with sufficient capital
        let mut wealthy_burghers: Vec<(world_sim_core::AgentId, String, f64)> = agents.iter()
            .filter(|a| matches!(a.social_class, world_sim_agents::SocialClass::Burgher | world_sim_agents::SocialClass::Merchant) && a.wallet >= 500.0)
            .map(|a| (a.id, a.name.clone(), a.wallet))
            .collect();
        
        if wealthy_burghers.is_empty() {
            drop(agents);
            return;
        }
        
        // LOAN SYSTEM: Burghers offer construction loans to peasants
        let wood_price = self.get_market_price(world_sim_core::ResourceType::Wood);
        let stone_price = self.get_market_price(world_sim_core::ResourceType::Stone);
        let house_cost = (30.0 * wood_price + 10.0 * stone_price) * 3.0;
        
        // Collect loan requests (avoid double borrow)
        let mut loan_requests: Vec<(world_sim_core::AgentId, String, f64)> = Vec::new(); // (borrower_id, name, amount_needed)
        
        for agent in agents.iter() {
            if !matches!(agent.social_class, world_sim_agents::SocialClass::Peasant) {
                continue;
            }
            
            // Peasant wants to build but doesn't have full amount
            if agent.wallet < house_cost && agent.wallet >= house_cost * 0.3 && agent.loans_owed.is_empty() {
                // Has 30%+ down payment AND no existing loans - eligible!
                let loan_amount = house_cost - agent.wallet;
                loan_requests.push((agent.id, agent.name.clone(), loan_amount));
            }
        }
        
        // Process loans (now we can safely mutate)
        let mut loans_issued = 0;
        for (borrower_id, borrower_name, loan_amount) in loan_requests {
            // Find a wealthy burgher
            if let Some(idx) = wealthy_burghers.iter().position(|(_, _, wallet)| *wallet >= loan_amount) {
                let (lender_id, lender_name, _) = wealthy_burghers[idx].clone();
                
                let loan = world_sim_agents::Loan {
                    lender_id,
                    borrower_id,
                    principal: loan_amount,
                    remaining: loan_amount * 1.05, // 5% interest
                    interest_rate: 0.05,
                    issued_time: self.sim_time.seconds,
                };
                
                // Update borrower
                if let Some(borrower) = agents.iter_mut().find(|a| a.id == borrower_id) {
                    borrower.wallet += loan_amount;
                    borrower.loans_owed.push(loan.clone());
                }
                
                // Update lender
                if let Some(lender) = agents.iter_mut().find(|a| a.id == lender_id) {
                    lender.wallet -= loan_amount;
                    lender.loans_given.push(loan);
                }
                
                info!("🏦 Burgher {} lent {:.1} gold to {} for construction (5% interest)", 
                      lender_name, loan_amount, borrower_name);
                
                loans_issued += 1;
                
                // Update cached wallet and remove if depleted
                wealthy_burghers[idx].2 -= loan_amount;
                if wealthy_burghers[idx].2 < 500.0 {
                    wealthy_burghers.remove(idx);
                }
                
                if loans_issued >= 3 {
                    break; // Max 3 loans per second to avoid spam
                }
            }
        }
        
        if loans_issued > 0 {
            info!("🏦 Issued {} construction loans this cycle", loans_issued);
        }
        
        drop(agents);
    }
    
    /// Assign idle builders to incomplete buildings (priority queue: oldest first)
    fn assign_builders_to_buildings(&self) {
        let buildings_lock = self.buildings.read();
        let all_buildings = buildings_lock.get_all_buildings();
        
        // Collect building info we need (clone to avoid borrow issues)
        let mut incomplete_buildings: Vec<(Uuid, String, world_sim_world::BuildingType, std::collections::HashMap<world_sim_core::ResourceType, u32>, f32)> = all_buildings.iter()
            .filter(|b| b.construction_progress < 1.0)
            .map(|b| (b.id, b.name.clone(), b.building_type, b.remaining_resources(), b.construction_progress))
            .collect();
        
        // Sort by progress (least complete first to focus efforts)
        incomplete_buildings.sort_by(|a, b| a.4.partial_cmp(&b.4).unwrap());
        
        drop(buildings_lock); // Release read lock before acquiring agents lock
        
        if incomplete_buildings.is_empty() {
            return; // No work to do
        }
        
        let mut agents = self.lifecycle.get_agents_mut();
        
        // Count all builders and their states for diagnostics
        let all_builders: Vec<_> = agents.iter()
            .filter(|a| matches!(a.job, Job::Builder))
            .collect();
        
        let total_builders = all_builders.len();
        let idle_count = all_builders.iter().filter(|a| matches!(a.state, AgentState::Idle)).count();
        let carrying_count = all_builders.iter().filter(|a| a.carrying_resources.is_some()).count();
        let building_state_count = all_builders.iter().filter(|a| matches!(a.state, AgentState::Building { .. })).count();
        let eating_count = all_builders.iter().filter(|a| matches!(a.state, AgentState::Eating)).count();
        let sleeping_count = all_builders.iter().filter(|a| matches!(a.state, AgentState::Sleeping)).count();
        let working_count = all_builders.iter().filter(|a| matches!(a.state, AgentState::Working { .. })).count();
        let trading_count = all_builders.iter().filter(|a| matches!(a.state, AgentState::Trading { .. })).count();
        let moving_count = all_builders.iter().filter(|a| matches!(a.state, AgentState::Moving { .. })).count();
        let fighting_count = all_builders.iter().filter(|a| matches!(a.state, AgentState::Fighting { .. })).count();
        let talking_count = all_builders.iter().filter(|a| matches!(a.state, AgentState::Talking { .. })).count();
        let patrolling_count = all_builders.iter().filter(|a| matches!(a.state, AgentState::Patrolling { .. })).count();
        let following_count = all_builders.iter().filter(|a| matches!(a.state, AgentState::Following { .. })).count();
        let dead_count = all_builders.iter().filter(|a| matches!(a.state, AgentState::Dead)).count();
        
        // Find all idle builders (with no resources)
        let idle_builders: Vec<_> = agents.iter()
            .filter(|a| matches!(a.job, Job::Builder) && matches!(a.state, AgentState::Idle) && a.carrying_resources.is_none())
            .map(|a| a.id)
            .collect();
        
        // ALWAYS log builder diagnostics when there are incomplete buildings
        info!("🔨 Builder assignment check: {} incomplete buildings | Builders: {} total | States: Idle:{} Eating:{} Sleeping:{} Working:{} Trading:{} Building:{} Moving:{} Fighting:{} Talking:{} Patrol:{} Follow:{} Dead:{} | Carrying:{}", 
              incomplete_buildings.len(), total_builders, idle_count, eating_count, sleeping_count, working_count, trading_count, building_state_count, moving_count, fighting_count, talking_count, patrolling_count, following_count, dead_count, carrying_count);
        
        if idle_builders.is_empty() {
            if total_builders == 0 {
                info!("⚠️ No builders exist! Need to assign more agents to Builder job.");
            } else {
                info!("⚠️ No idle builders available (all {} builders are busy or carrying resources)", total_builders);
            }
            return; // No idle builders
        }
        
        info!("🔨 Builder assignment: {} idle builders, {} incomplete buildings", 
              idle_builders.len(), incomplete_buildings.len());
        
        // Assign builders to buildings (round-robin to distribute workload)
        for (idx, builder_id) in idle_builders.iter().enumerate() {
            let building_idx = idx % incomplete_buildings.len();
            let (building_id, building_name, building_type, remaining, _progress) = &incomplete_buildings[building_idx];
            
            if let Some(agent) = agents.iter_mut().find(|a| a.id == *builder_id) {
                if remaining.values().sum::<u32>() == 0 {
                    // Building has all resources, can construct!
                    agent.state = AgentState::Building { building_type: format!("{:?}", building_type) };
                    // Set target building so builder knows where to go
                    agent.carrying_resources = Some(world_sim_agents::BuildingResources {
                        wood: 0,
                        stone: 0,
                        iron: 0,
                        target_building_id: *building_id,
                    });
                    info!("🔨 Builder {} assigned to {} (ID: {:?}, has all resources, can construct!)", 
                          agent.name, building_name, building_id);
                } else {
                    // Need to get resources from market - set target but with zero resources
                    agent.state = AgentState::Moving { destination: world_sim_core::GridCoord { x: 0, y: 0, z: 0 } }; // Will move to market
                    // Set target building ID, but keep resources at ZERO
                    agent.carrying_resources = Some(world_sim_agents::BuildingResources {
                        wood: 0,
                        stone: 0,
                        iron: 0,
                        target_building_id: *building_id,
                    });
                    info!("🔨 Builder {} assigned to {} (ID: {:?}, needs: {:?}) - heading to market to buy resources", 
                          agent.name, building_name, building_id, remaining);
                }
            }
        }
    }
    
    /// Rebalance labor force to ensure minimum harvesters
    fn rebalance_labor(&self) {
        let mut agents = self.lifecycle.get_agents_mut();
        let total = agents.len();
        
        if total == 0 {
            info!("⚖️ Labor rebalance: No agents to rebalance");
            return;
        }
        
        // Count current job distribution
        let woodcutters = agents.iter().filter(|a| matches!(a.job, Job::Woodcutter)).count();
        let miners = agents.iter().filter(|a| matches!(a.job, Job::Miner)).count();
        let farmers = agents.iter().filter(|a| matches!(a.job, Job::Farmer)).count();
        let builders = agents.iter().filter(|a| matches!(a.job, Job::Builder)).count();
        let unemployed = agents.iter().filter(|a| matches!(a.job, Job::Unemployed)).count();
        
        let harvesters = woodcutters + miners + farmers;
        let harvester_percentage = (harvesters as f32 / total as f32) * 100.0;
        
        // PRICE-BASED LABOR ALLOCATION: Calculate demand based on market prices & inventory
        let markets = self.markets.read();
        let all_markets = markets.get_all_markets();
        
        // Aggregate market data
        let mut total_wood_inventory = 0u32;
        let mut total_stone_inventory = 0u32;
        let mut total_iron_inventory = 0u32;
        let mut total_food_inventory = 0u32;
        let mut wood_price = 5.0;
        let mut stone_price = 3.0;
        let mut iron_price = 15.0;
        let mut food_price = 10.0;
        
        for market in all_markets {
            if let Some(wood_good) = market.inventory.get(&world_sim_core::ResourceType::Wood) {
                total_wood_inventory += wood_good.quantity;
                wood_price = wood_good.current_price; // Use last market's price
            }
            if let Some(stone_good) = market.inventory.get(&world_sim_core::ResourceType::Stone) {
                total_stone_inventory += stone_good.quantity;
                stone_price = stone_good.current_price;
            }
            if let Some(iron_good) = market.inventory.get(&world_sim_core::ResourceType::Iron) {
                total_iron_inventory += iron_good.quantity;
                iron_price = iron_good.current_price;
            }
            if let Some(food_good) = market.inventory.get(&world_sim_core::ResourceType::Food) {
                total_food_inventory += food_good.quantity;
                food_price = food_good.current_price;
            }
        }
        
        drop(markets);
        
        // Calculate "demand scores" (higher = more valuable = more workers needed)
        // Demand score = price / (inventory + 10)  [scarce + expensive = high score]
        let wood_demand = wood_price / (total_wood_inventory as f64 + 10.0);
        let stone_demand = stone_price / (total_stone_inventory as f64 + 10.0);
        let iron_demand = iron_price / (total_iron_inventory as f64 + 10.0);
        let food_demand = food_price / (total_food_inventory as f64 + 10.0);
        
        let total_demand = wood_demand + stone_demand + iron_demand + food_demand;
        
        // Calculate target worker distribution based on demand scores
        let target_harvesters = (total as f32 * 0.40) as usize;
        
        // Miners harvest BOTH stone and iron, so their target is based on combined demand
        let target_woodcutters = ((wood_demand / total_demand) * target_harvesters as f64).max(1.0) as usize;
        let target_miners = (((stone_demand + iron_demand) / total_demand) * target_harvesters as f64).max(1.0) as usize;
        let target_farmers = ((food_demand / total_demand) * target_harvesters as f64).max(1.0) as usize;
        
        // Adjust if targets exceed available slots
        let target_sum = target_woodcutters + target_miners + target_farmers;
        let (final_woodcutters, final_miners, final_farmers) = if target_sum > target_harvesters {
            // Scale down proportionally
            let scale = target_harvesters as f32 / target_sum as f32;
            (
                (target_woodcutters as f32 * scale).max(1.0) as usize,
                (target_miners as f32 * scale).max(1.0) as usize,
                (target_farmers as f32 * scale).max(1.0) as usize,
            )
        } else {
            (target_woodcutters, target_miners, target_farmers)
        };
        
        // ALWAYS log for diagnostics
        info!("💹 Market demand: Wood:{:.3} Stone:{:.3} Iron:{:.3} Food:{:.3}", 
              wood_demand, stone_demand, iron_demand, food_demand);
        info!("⚖️ Target jobs: W:{} M:{} F:{} (current: W:{} M:{} F:{})", 
              final_woodcutters, final_miners, final_farmers, woodcutters, miners, farmers);
        info!("📦 Market inventory: 🌲{} 🪨{} ⚙️{} 🌾{}", 
              total_wood_inventory, total_stone_inventory, total_iron_inventory, total_food_inventory);
        
        if harvesters < target_harvesters {
            // Need more harvesters
            let needed = target_harvesters - harvesters;
            let mut converted = 0;
            let mut current_wood = woodcutters;
            let mut current_miners = miners;
            let mut current_farmers = farmers;
            
            for agent in agents.iter_mut() {
                if converted >= needed {
                    break;
                }
                
                // Only convert Builders and Unemployed (not upper classes)
                if matches!(agent.job, Job::Builder | Job::Unemployed) {
                    if matches!(agent.social_class, 
                               world_sim_agents::SocialClass::King | 
                               world_sim_agents::SocialClass::Noble | 
                               world_sim_agents::SocialClass::Knight |
                               world_sim_agents::SocialClass::Soldier) {
                        continue;
                    }
                    
                    // Assign based on market demand targets
                    let new_job = if current_wood < final_woodcutters {
                        current_wood += 1;
                        Job::Woodcutter
                    } else if current_miners < final_miners {
                        current_miners += 1;
                        Job::Miner
                    } else if current_farmers < final_farmers {
                        current_farmers += 1;
                        Job::Farmer
                    } else {
                        // If all targets met, assign to highest demand
                        if wood_demand >= stone_demand + iron_demand && wood_demand >= food_demand {
                            current_wood += 1;
                            Job::Woodcutter
                        } else if stone_demand + iron_demand >= food_demand {
                            current_miners += 1;
                            Job::Miner
                        } else {
                            current_farmers += 1;
                            Job::Farmer
                        }
                    };
                    
                    info!("🔄 Converting {} from {:?} to {:?} (market demand)", agent.name, agent.job, new_job);
                    agent.job = new_job;
                    converted += 1;
                }
            }
            
            if converted > 0 {
                info!("✅ Converted {} agents based on market demand (W:+{} M:+{} F:+{})", 
                      converted, current_wood - woodcutters, current_miners - miners, current_farmers - farmers);
            } else {
                info!("⚠️ Could not convert any agents! All eligible agents may already be harvesters or protected");
            }
        }
    }
    
    /// Very slow tick (1/minute) - ecology, demographics, metrics
    pub async fn tick_very_slow(&mut self, _delta_seconds: f64) -> Result<()> {
        info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        info!("🕐 tick_very_slow STARTING (runs every 60s)");
        info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        
        // LABOR REBALANCING: Ensure minimum harvesting workforce
        info!("🕐 About to call rebalance_labor...");
        self.rebalance_labor();
        info!("🕐 rebalance_labor completed");
        
        // TAX COLLECTION: Nobles and Kings collect taxes to fund public works
        self.collect_taxes();
        
        // FUND REPLENISHMENT: Top up construction funds if prices rose
        self.replenish_construction_funds();
        
        // Update ecology
        self.ecology.tick(&self.event_bus, &self.grid).await;
        
        // Update demographics
        self.lifecycle.tick().await;
        
        let agent_count = self.lifecycle.count_living();
        
        // Check for resource scarcity and trigger wars organically
        self.check_resource_scarcity_and_trigger_wars().await;
        
        // HIERARCHICAL AI: King decision-making
        self.process_king_decisions().await;
        
        // HIERARCHICAL AI: Noble order execution
        self.process_noble_orders().await;
        
        // HIERARCHICAL AI: Peasant self-building
        self.process_peasant_building().await;
        
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
                    AgentState::Talking { .. } => "Talking",
                    AgentState::Patrolling { .. } => "Patrolling",
                    AgentState::Following { .. } => "Following",
                    AgentState::Building { .. } => "Building",
                    AgentState::Trading { .. } => "Trading",
                };
                
                let social_class_str = match a.social_class {
                    world_sim_agents::SocialClass::King => "King",
                    world_sim_agents::SocialClass::Noble => "Noble",
                    world_sim_agents::SocialClass::Knight => "Knight",
                    world_sim_agents::SocialClass::Soldier => "Soldier",
                    world_sim_agents::SocialClass::Merchant => "Merchant",
                    world_sim_agents::SocialClass::Burgher => "Burgher",
                    world_sim_agents::SocialClass::Cleric => "Cleric",
                    world_sim_agents::SocialClass::Peasant => "Peasant",
                };
                
                // Filter just_harvested if it's older than 2 seconds
                let just_harvested = a.just_harvested.and_then(|(resource_type, amount, timestamp)| {
                    if self.sim_time.seconds - timestamp < 2.0 {
                        let resource_str = match resource_type {
                            world_sim_core::ResourceType::Wood => "wood",
                            world_sim_core::ResourceType::Stone => "stone",
                            world_sim_core::ResourceType::Food => "food",
                            world_sim_core::ResourceType::Iron => "iron",
                            world_sim_core::ResourceType::Gold => "gold",
                            world_sim_core::ResourceType::Water => "water",
                            world_sim_core::ResourceType::Cloth => "cloth",
                            world_sim_core::ResourceType::Tool => "tool",
                            world_sim_core::ResourceType::Weapon => "weapon",
                            world_sim_core::ResourceType::Coin => "coin",
                        };
                        Some((resource_str.to_string(), amount))
                    } else {
                        None
                    }
                });
                
                // Filter just_transacted if it's older than 2 seconds
                let just_transacted = a.just_transacted.and_then(|(transaction_type, amount, timestamp)| {
                    if self.sim_time.seconds - timestamp < 2.0 {
                        let transaction_str = match transaction_type {
                            world_sim_agents::TransactionType::Sold => "Sold",
                            world_sim_agents::TransactionType::Bought => "Bought",
                        };
                        Some((transaction_str.to_string(), amount))
                    } else {
                        None
                    }
                });
                
                ApiAgentState {
                    id: format!("{:?}", a.id),
                    x: a.position.x,
                    y: a.position.y,
                    z: a.position.z,
                    name: a.name.clone(),
                    state: state_str.to_string(),
                    faction: a.personality.beliefs.faction_loyalty.map(|f| format!("{:?}", f)),
                    social_class: social_class_str.to_string(),
                    leader_id: a.leader_id.map(|l| format!("{:?}", l)),
                    wallet: a.wallet,
                    inventory_wood: *a.inventory.get(&world_sim_core::ResourceType::Wood).unwrap_or(&0),
                    inventory_stone: *a.inventory.get(&world_sim_core::ResourceType::Stone).unwrap_or(&0),
                    inventory_food: *a.inventory.get(&world_sim_core::ResourceType::Food).unwrap_or(&0),
                    inventory_iron: *a.inventory.get(&world_sim_core::ResourceType::Iron).unwrap_or(&0),
                    carrying_wood: a.carrying_resources.as_ref().map(|c| c.wood).unwrap_or(0),
                    carrying_stone: a.carrying_resources.as_ref().map(|c| c.stone).unwrap_or(0),
                    carrying_iron: a.carrying_resources.as_ref().map(|c| c.iron).unwrap_or(0),
                    target_building_id: a.carrying_resources.as_ref()
                        .map(|c| c.target_building_id.to_string()),
                    just_harvested,
                    just_transacted,
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
        
        // Update markets
        let markets = self.markets.read();
        world_state.markets = markets.get_all_markets()
            .iter()
            .map(|m| {
                let market_type_str = match m.market_type {
                    MarketType::General => "general",
                    MarketType::Food => "food",
                    MarketType::Materials => "materials",
                    MarketType::Luxury => "luxury",
                    MarketType::Weapons => "weapons",
                };
                
                world_sim_admin_api::MarketState {
                    id: format!("{}", m.id),
                    name: m.name.clone(),
                    market_type: market_type_str.to_string(),
                    x: m.position.x,
                    y: m.position.y,
                    z: m.position.z,
                    transaction_count: m.transaction_count,
                    reputation: m.reputation,
                    inventory_wood: m.inventory.get(&world_sim_core::ResourceType::Wood).map(|g| g.quantity).unwrap_or(0),
                    inventory_stone: m.inventory.get(&world_sim_core::ResourceType::Stone).map(|g| g.quantity).unwrap_or(0),
                    inventory_food: m.inventory.get(&world_sim_core::ResourceType::Food).map(|g| g.quantity).unwrap_or(0),
                    inventory_iron: m.inventory.get(&world_sim_core::ResourceType::Iron).map(|g| g.quantity).unwrap_or(0),
                }
            })
            .collect();
        
        // Update currency info
        let currency = self.currency.read();
        world_state.currency_info = world_sim_admin_api::CurrencyInfo {
            total_supply: currency.total_supply,
            inflation_rate: currency.inflation_rate,
            purchasing_power: currency.get_purchasing_power(),
            transaction_count: currency.transaction_count,
        };
        
        // Update buildings
        let buildings = self.buildings.read();
        let all_buildings = buildings.get_all_buildings();
        world_state.buildings = all_buildings
            .iter()
            .map(|b| {
                let building_type_str = format!("{:?}", b.building_type);
                let owner_str = match &b.owner {
                    world_sim_world::BuildingOwner::Public => "Public".to_string(),
                    world_sim_world::BuildingOwner::Faction(f) => format!("Faction({:?})", f),
                    world_sim_world::BuildingOwner::Agent(a) => format!("Agent({:?})", a),
                };
                
                world_sim_admin_api::BuildingState {
                    id: format!("{}", b.id),
                    building_type: building_type_str,
                    name: b.name.clone(),
                    x: b.position.x,
                    y: b.position.y,
                    z: b.position.z,
                    construction_progress: b.construction_progress,
                    health: b.health,
                    owner: owner_str,
                    storage_wood: b.storage.get_quantity(world_sim_core::ResourceType::Wood),
                    storage_stone: b.storage.get_quantity(world_sim_core::ResourceType::Stone),
                    storage_food: b.storage.get_quantity(world_sim_core::ResourceType::Food),
                    storage_iron: b.storage.get_quantity(world_sim_core::ResourceType::Iron),
                    required_wood: *b.required_resources.get(&world_sim_core::ResourceType::Wood).unwrap_or(&0),
                    required_stone: *b.required_resources.get(&world_sim_core::ResourceType::Stone).unwrap_or(&0),
                    required_iron: *b.required_resources.get(&world_sim_core::ResourceType::Iron).unwrap_or(&0),
                    current_wood: *b.current_resources.get(&world_sim_core::ResourceType::Wood).unwrap_or(&0),
                    current_stone: *b.current_resources.get(&world_sim_core::ResourceType::Stone).unwrap_or(&0),
                    current_iron: *b.current_resources.get(&world_sim_core::ResourceType::Iron).unwrap_or(&0),
                    construction_fund: b.construction_fund,
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
    
    /// Check for resource scarcity and trigger wars organically
    async fn check_resource_scarcity_and_trigger_wars(&self) {
        let resource_nodes = self.resources.get_nodes();
        
        // Calculate total resources available
        let total_food: u32 = resource_nodes
            .iter()
            .filter(|r| matches!(r.resource_type, ResourceNodeType::Farm))
            .map(|r| r.quantity)
            .sum();
        
        let total_materials: u32 = resource_nodes
            .iter()
            .filter(|r| matches!(
                r.resource_type,
                ResourceNodeType::Tree | ResourceNodeType::Rock | ResourceNodeType::IronDeposit
            ))
            .map(|r| r.quantity)
            .sum();
        
        let agent_count = self.lifecycle.count_living();
        
        // Check if resources are scarce (per capita)
        let food_per_capita = if agent_count > 0 {
            total_food as f32 / agent_count as f32
        } else {
            100.0
        };
        
        let materials_per_capita = if agent_count > 0 {
            total_materials as f32 / agent_count as f32
        } else {
            100.0
        };
        
        // Trigger war if resources are scarce (< 15 per person) and no active war
        if food_per_capita < 15.0 || materials_per_capita < 20.0 {
            // Check if already at war
            let factions = self.politics.get_all_factions();
            
            if factions.len() >= 2 {
                let faction_a = factions[0].id;
                let faction_b = factions[1].id;
                
                // Check if already at war (simplified - check if agents are hostile)
                let agents = self.lifecycle.get_agents();
                let has_combat = agents.iter().any(|a| matches!(a.state, AgentState::Fighting { .. }));
                
                // Only declare war if not already fighting and resources are critically low
                if !has_combat && (food_per_capita < 10.0 || materials_per_capita < 15.0) {
                    let reason = if food_per_capita < materials_per_capita {
                        format!("Food scarcity crisis! ({:.1} food per person)", food_per_capita)
                    } else {
                        format!("Material shortage! ({:.1} materials per person)", materials_per_capita)
                    };
                    
                    self.politics.declare_war(faction_a, faction_b, reason).await;
                    
                    info!("⚔️ WAR DECLARED due to resource scarcity!");
                    info!("  Food per capita: {:.1}", food_per_capita);
                    info!("  Materials per capita: {:.1}", materials_per_capita);
                }
            }
        }
    }
    
    /// HIERARCHICAL AI: King decision-making (sets kingdom goals)
    async fn process_king_decisions(&self) {
        let agents = self.lifecycle.get_agents();
        let resource_nodes = self.resources.get_nodes();
        let agent_count = agents.len();
        
        // Calculate economic metrics for decision-making
        let total_food: u32 = resource_nodes.iter()
            .filter(|r| matches!(r.resource_type, ResourceNodeType::Farm))
            .map(|r| r.quantity).sum();
        
        let total_materials: u32 = resource_nodes.iter()
            .filter(|r| matches!(r.resource_type, ResourceNodeType::Tree | ResourceNodeType::Rock | ResourceNodeType::IronDeposit))
            .map(|r| r.quantity).sum();
        
        let food_per_capita = if agent_count > 0 { total_food as f32 / agent_count as f32 } else { 100.0 };
        let materials_per_capita = if agent_count > 0 { total_materials as f32 / agent_count as f32 } else { 100.0 };
        
        // Check for threats
        let factions = self.politics.get_all_factions();
        let at_war = agents.iter().any(|a| matches!(a.state, AgentState::Fighting { .. }));
        let has_enemies = factions.len() >= 2;
        
        // Find all kings and make decisions
        let mut kingdoms_lock = self.kingdoms.write();
        
        for agent in agents.iter() {
            if matches!(agent.social_class, world_sim_agents::SocialClass::King) {
                // Ensure king has a kingdom
                if kingdoms_lock.get_kingdom_by_king(agent.id).is_none() {
                    kingdoms_lock.create_kingdom(agent.id, agent.position);
                    info!("👑 Kingdom established by {}", agent.name);
                }
                
                if let Some(kingdom) = kingdoms_lock.get_kingdom_by_king_mut(agent.id) {
                    // King AI: Analyze situation and set goal
                    let new_goal = if at_war || has_enemies {
                        use world_sim_societal::KingdomGoal;
                        (KingdomGoal::DefendTerritory, 1.0)
                    } else if food_per_capita < 15.0 {
                        use world_sim_societal::KingdomGoal;
                        (KingdomGoal::GrowPopulation, 0.9)
                    } else if materials_per_capita < 25.0 {
                        use world_sim_societal::KingdomGoal;
                        (KingdomGoal::ExpandResources, 0.8)
                    } else if agent_count > 50 {
                        use world_sim_societal::KingdomGoal;
                        (KingdomGoal::ImproveInfrastructure, 0.6)
                    } else {
                        use world_sim_societal::KingdomGoal;
                        (KingdomGoal::Consolidate, 0.3)
                    };
                    
                    if kingdom.current_goal != new_goal.0 {
                        kingdom.set_goal(new_goal.0, new_goal.1, self.sim_time.seconds);
                        info!("👑 King {} sets new goal: {:?} (priority: {:.1})", 
                              agent.name, new_goal.0, new_goal.1);
                    }
                }
            }
        }
        drop(kingdoms_lock); // Explicitly drop kingdoms write lock
    }
    
    /// HIERARCHICAL AI: Noble order execution (creates building orders)
    async fn process_noble_orders(&self) {
        // CONSTRUCTION LIMIT: Check how many buildings are currently under construction
        let buildings = self.buildings.read();
        let under_construction = buildings.get_all_buildings().iter()
            .filter(|b| b.construction_progress < 1.0)
            .count();
        
        // Limit: Maximum 8 buildings under construction at once
        const MAX_CONCURRENT_CONSTRUCTION: usize = 8;
        
        if under_construction >= MAX_CONCURRENT_CONSTRUCTION {
            info!("🚧 Construction limit reached: {}/{} buildings under construction - no new orders", 
                  under_construction, MAX_CONCURRENT_CONSTRUCTION);
            return;
        }
        
        let available_slots = MAX_CONCURRENT_CONSTRUCTION - under_construction;
        info!("🚧 Construction capacity: {}/{} buildings under construction, {} slots available", 
              under_construction, MAX_CONCURRENT_CONSTRUCTION, available_slots);
        drop(buildings);
        
        let agents = self.lifecycle.get_agents();
        let mut kingdoms_write = self.kingdoms.write();
        let mut buildings_created = 0;
        
        for agent in agents.iter() {
            if buildings_created >= available_slots {
                break; // Stop if we've used all available slots
            }
            
            if matches!(agent.social_class, world_sim_agents::SocialClass::Noble) {
                // Find king's kingdom to get current goal
                let king_goal = agents.iter()
                    .filter(|a| matches!(a.social_class, world_sim_agents::SocialClass::King))
                    .filter_map(|king| kingdoms_write.get_kingdom_by_king(king.id))
                    .next()
                    .map(|k| k.current_goal);
                
                if let Some(goal) = king_goal {
                    // Noble AI: Execute king's goal by creating building orders
                    use world_sim_societal::{KingdomGoal, NobleOrder};
                    use world_sim_world::BuildingType;
                    use rand::Rng;
                    let mut rng = rand::thread_rng();
                    
                    // Only create new orders occasionally (5% chance per minute, reduced from 10%)
                    if rng.gen::<f32>() < 0.05 {
                        let (building_type, priority) = match goal {
                            KingdomGoal::DefendTerritory => {
                                if rng.gen::<bool>() {
                                    (BuildingType::Barracks, 0.9)
                                } else {
                                    (BuildingType::Walls, 1.0)
                                }
                            },
                            KingdomGoal::ExpandResources => {
                                if rng.gen::<bool>() {
                                    (BuildingType::Farm, 0.8)
                                } else {
                                    (BuildingType::Mine, 0.9)
                                }
                            },
                            KingdomGoal::PrepareForWar => {
                                (BuildingType::Barracks, 1.0)
                            },
                            KingdomGoal::GrowPopulation => {
                                (BuildingType::Farm, 0.9)
                            },
                            KingdomGoal::ImproveInfrastructure => {
                                let choice = rng.gen_range(0..3);
                                match choice {
                                    0 => (BuildingType::Workshop, 0.7),
                                    1 => (BuildingType::Tavern, 0.5),
                                    _ => (BuildingType::Market, 0.8),
                                }
                            },
                            KingdomGoal::Consolidate => {
                                // No new orders during consolidation
                                continue;
                            }
                        };
                        
                        // Choose location near noble's position
                        let offset_x = rng.gen_range(-20.0..20.0);
                        let offset_z = rng.gen_range(-20.0..20.0);
                        let location = Position::new(
                            agent.position.x + offset_x,
                            1.0,
                            agent.position.z + offset_z
                        );
                        
                        let order = NobleOrder::new(agent.id, building_type, location, priority);
                        kingdoms_write.add_noble_order(order.clone());
                        
                        let requirements = building_type.required_resources();
                        let req_summary = format!("{}W, {}S, {}I", 
                            requirements.get(&world_sim_core::ResourceType::Wood).unwrap_or(&0),
                            requirements.get(&world_sim_core::ResourceType::Stone).unwrap_or(&0),
                            requirements.get(&world_sim_core::ResourceType::Iron).unwrap_or(&0));
                        info!("🏛️ Noble {} orders {:?} at ({:.1}, {:.1}) [Needs: {}]", 
                              agent.name, building_type, location.x, location.z, req_summary);
                        
                        // Create the actual building
                        let mut buildings = self.buildings.write();
                        let mut new_building = world_sim_world::Building::new(
                            building_type,
                            location,
                            format!("{:?} (Noble Order)", building_type),
                            world_sim_world::BuildingOwner::Public,
                        );
                        
                        // FUNDING: Noble allocates construction funds from their own wallet
                        // Calculate total cost using REAL CURRENT MARKET PRICES
                        let total_cost = new_building.required_resources.iter()
                            .map(|(resource_type, qty)| {
                                let market_price = self.get_market_price(*resource_type);
                                market_price * (*qty as f64)
                            })
                            .sum::<f64>();
                        
                        // Noble funds the building (allocates 300% for price volatility + market inefficiency)
                        let allocated_funds = total_cost * 3.0;
                        new_building.construction_fund = allocated_funds;
                        
                        info!("💰 Noble {} allocated {:.1} gold for {:?} construction (estimated cost: {:.1})", 
                              agent.name, allocated_funds, building_type, total_cost);
                        
                        let building_id = new_building.id;
                        buildings.add_building(new_building);
                        
                        // Update order with building ID
                        if let Some(order_mut) = kingdoms_write.get_order_mut(order.id) {
                            order_mut.building_id = Some(building_id);
                            order_mut.status = world_sim_societal::OrderStatus::InProgress;
                        }
                        
                        buildings_created += 1; // Track how many buildings we've created
                        drop(buildings); // CRITICAL: Drop buildings write lock immediately
                    }
                }
            }
        }
        drop(kingdoms_write); // Explicitly drop kingdoms write lock
    }
    
    /// HIERARCHICAL AI: Peasant self-building (personal needs)
    async fn process_peasant_building(&self) {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        
        // CONSTRUCTION LIMIT: Check how many buildings are currently under construction
        let buildings = self.buildings.read();
        let under_construction = buildings.get_all_buildings().iter()
            .filter(|b| b.construction_progress < 1.0)
            .count();
        
        // Use same limit as nobles
        const MAX_CONCURRENT_CONSTRUCTION: usize = 8;
        
        if under_construction >= MAX_CONCURRENT_CONSTRUCTION {
            info!("🚧 Construction limit reached - peasants cannot start new buildings");
            return;
        }
        
        let available_slots = MAX_CONCURRENT_CONSTRUCTION - under_construction;
        drop(buildings);
        
        let agents = self.lifecycle.get_agents();
        let mut buildings_created = 0;
        
        for agent in agents.iter() {
            if buildings_created >= available_slots {
                break; // Stop if we've used all available slots
            }
            
            if matches!(agent.social_class, world_sim_agents::SocialClass::Peasant) {
                // Peasants occasionally decide to build for themselves (10% chance per minute for more construction)
                if rng.gen::<f32>() < 0.10 {
                    // Check if they have a home nearby
                    let buildings = self.buildings.read();
                    let has_nearby_house = buildings.get_all_buildings().iter()
                        .any(|b| matches!(b.building_type, world_sim_world::BuildingType::PeasantHouse)
                            && b.position.distance_to(&agent.position) < 30.0);
                    
                    drop(buildings);
                    
                    if !has_nearby_house {
                        // Calculate cost using REAL CURRENT MARKET PRICES
                        let wood_price = self.get_market_price(world_sim_core::ResourceType::Wood);
                        let stone_price = self.get_market_price(world_sim_core::ResourceType::Stone);
                        
                        let house_wood_cost = 30.0 * wood_price;  // 30 wood @ market price
                        let house_stone_cost = 10.0 * stone_price; // 10 stone @ market price
                        let total_house_cost = (house_wood_cost + house_stone_cost) * 3.0; // 300% buffer for volatility
                        
                        // Check if peasant can afford it
                        if agent.wallet >= total_house_cost {
                            // Build a house for themselves
                            use world_sim_world::BuildingType;
                            
                            let offset_x = rng.gen_range(-10.0..10.0);
                            let offset_z = rng.gen_range(-10.0..10.0);
                            let location = Position::new(
                                agent.position.x + offset_x,
                                1.0,
                                agent.position.z + offset_z
                            );
                            
                            let mut buildings = self.buildings.write();
                            let mut house = world_sim_world::Building::new(
                                BuildingType::PeasantHouse,
                                location,
                                format!("{}'s House", agent.name),
                                world_sim_world::BuildingOwner::Agent(agent.id),
                            );
                            
                            // FUNDING: Peasant allocates their own money for construction
                            house.construction_fund = total_house_cost;
                            // NOTE: We DON'T deduct from wallet yet - it's deducted when builders buy materials
                            
                            buildings.add_building(house);
                            drop(buildings); // CRITICAL: Drop buildings write lock immediately
                            
                            buildings_created += 1; // Track buildings created
                            
                            info!("🏠 Peasant {} starts building a house at ({:.1}, {:.1}) [Needs: 30 wood, 10 stone] [Fund: {:.1} gold]", 
                                  agent.name, location.x, location.z, total_house_cost);
                        } else {
                            // Removed spam log - peasants silently save money
                        }
                    } else if agent.job == Job::Farmer {
                        // Farmers build sheds
                        let has_nearby_shed = {
                            let buildings = self.buildings.read();
                            buildings.get_all_buildings().iter()
                                .any(|b| matches!(b.building_type, world_sim_world::BuildingType::FarmingShed)
                                    && b.position.distance_to(&agent.position) < 20.0)
                        };
                        
                        if !has_nearby_shed {
                            // Calculate cost using REAL CURRENT MARKET PRICES
                            let wood_price = self.get_market_price(world_sim_core::ResourceType::Wood);
                            let stone_price = self.get_market_price(world_sim_core::ResourceType::Stone);
                            
                            let shed_wood_cost = 20.0 * wood_price;  // 20 wood @ market price
                            let shed_stone_cost = 5.0 * stone_price;  // 5 stone @ market price
                            let total_shed_cost = (shed_wood_cost + shed_stone_cost) * 3.0; // 300% buffer
                            
                            // Check if farmer can afford it
                            if agent.wallet >= total_shed_cost {
                                use world_sim_world::BuildingType;
                                
                                let offset_x = rng.gen_range(-8.0..8.0);
                                let offset_z = rng.gen_range(-8.0..8.0);
                                let location = Position::new(
                                    agent.position.x + offset_x,
                                    1.0,
                                    agent.position.z + offset_z
                                );
                                
                                let mut buildings = self.buildings.write();
                                let mut shed = world_sim_world::Building::new(
                                    BuildingType::FarmingShed,
                                    location,
                                    format!("{}'s Shed", agent.name),
                                    world_sim_world::BuildingOwner::Agent(agent.id),
                                );
                                
                                // FUNDING: Farmer allocates their own money for construction
                                shed.construction_fund = total_shed_cost;
                                
                                buildings.add_building(shed);
                                drop(buildings); // CRITICAL: Drop buildings write lock immediately
                                
                                buildings_created += 1; // Track buildings created
                                
                                info!("🌾 Farmer {} starts building a shed at ({:.1}, {:.1}) [Needs: 20 wood, 5 stone] [Fund: {:.1} gold]", 
                                      agent.name, location.x, location.z, total_shed_cost);
                            } else {
                                // Removed spam log - farmers silently save money
                            }
                        }
                    }
                }
            }
        }
        
        // DIAGNOSTICS: Why no buildings?
        if buildings_created == 0 {
            let peasant_count = agents.iter().filter(|a| matches!(a.social_class, world_sim_agents::SocialClass::Peasant)).count();
            let rich_peasants = agents.iter().filter(|a| matches!(a.social_class, world_sim_agents::SocialClass::Peasant) && a.wallet >= 400.0).count();
            info!("🏠 Peasant building check: {} peasants, {} can afford houses (>400g), {} buildings created this cycle", 
                  peasant_count, rich_peasants, buildings_created);
        }
    }
    
    /// Check if an agent can order construction of a building type (building permissions)
    fn can_order_building(social_class: world_sim_agents::SocialClass, building_type: world_sim_world::BuildingType) -> bool {
        use world_sim_agents::SocialClass;
        use world_sim_world::BuildingType;
        
        match (social_class, building_type) {
            // Kings can build anything
            (SocialClass::King, _) => true,
            
            // Nobles can build military and infrastructure
            (SocialClass::Noble, BuildingType::Barracks) => true,
            (SocialClass::Noble, BuildingType::Walls) => true,
            (SocialClass::Noble, BuildingType::Market) => true,
            (SocialClass::Noble, BuildingType::Workshop) => true,
            (SocialClass::Noble, BuildingType::NobleEstate) => true, // For themselves
            (SocialClass::Noble, BuildingType::Farm) => true,
            (SocialClass::Noble, BuildingType::Mine) => true,
            
            // Merchants can build commercial buildings
            (SocialClass::Merchant, BuildingType::Workshop) => true,
            (SocialClass::Merchant, BuildingType::Market) => true,
            (SocialClass::Merchant, BuildingType::Tavern) => true,
            
            // Burghers (craftsmen) can build workshops
            (SocialClass::Burgher, BuildingType::Workshop) => true,
            (SocialClass::Burgher, BuildingType::Tavern) => true,
            
            // Clerics can build churches
            (SocialClass::Cleric, BuildingType::Church) => true,
            
            // Peasants can only build basic personal structures
            (SocialClass::Peasant, BuildingType::PeasantHouse) => true,
            (SocialClass::Peasant, BuildingType::FarmingShed) => true,
            
            // Everything else is not permitted
            _ => false,
        }
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

