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
        
        // Test warehouse (starts incomplete for demonstration)
        let test_warehouse = Building::new(
            BuildingType::Warehouse,
            Position::new(0.0, 1.0, -35.0),
            "Test Construction Site".to_string(),
            BuildingOwner::Public,
        );
        // Leave at 0.0 progress - builders will construct it!
        building_manager.add_building(test_warehouse);
        
        info!("Created {} public buildings (including 1 under construction)", building_manager.get_all_buildings().len());
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
        
        Ok(Self {
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
                Job::Builder => {
                    // Builders work on incomplete buildings or help at construction sites
                    let buildings_lock = self.buildings.read();
                    
                    // Find nearest incomplete building in same faction
                    let target_building = buildings_lock.get_all_buildings()
                        .iter()
                        .filter(|b| {
                            !b.is_complete() && 
                            // Prioritize same faction buildings
                            if let (BuildingOwner::Faction(b_faction), Some(a_faction)) = 
                                (&b.owner, agent.personality.beliefs.faction_loyalty) {
                                *b_faction == a_faction
                            } else {
                                true // Public buildings
                            }
                        })
                        .min_by(|a, b| {
                            let dist_a = a.position.distance_to(&agent.position);
                            let dist_b = b.position.distance_to(&agent.position);
                            dist_a.partial_cmp(&dist_b).unwrap_or(std::cmp::Ordering::Equal)
                        })
                        .map(|b| (b.position, b.id, b.building_type));
                    
                    if let Some((build_pos, build_id, build_type)) = target_building {
                        let dist = agent.position.distance_to(&build_pos);
                        drop(buildings_lock);
                        
                        if dist > 3.0 {
                            // Move toward construction site
                            let dx = build_pos.x - agent.position.x;
                            let dz = build_pos.z - agent.position.z;
                            let dist = (dx * dx + dz * dz).sqrt();
                            if dist > 0.1 {
                                agent.position.x += (dx / dist) * 0.4;
                                agent.position.z += (dz / dist) * 0.4;
                            }
                            agent.state = AgentState::Moving { 
                                destination: GridCoord::new(build_pos.x as i32, 0, build_pos.z as i32)
                            };
                        } else {
                            // At construction site - work on building!
                            let type_str = format!("{:?}", build_type);
                            agent.state = AgentState::Building { 
                                building_type: type_str
                            };
                        }
                    } else {
                        // No incomplete buildings, social grouping
                        drop(buildings_lock);
                        if let Some(ally_pos) = nearest_ally_pos {
                            if nearest_ally_dist > 15.0 {
                                let dx = ally_pos.x - agent.position.x;
                                let dz = ally_pos.z - agent.position.z;
                                let dist = (dx * dx + dz * dz).sqrt();
                                if dist > 0.1 {
                                    agent.position.x += (dx / dist) * 0.2;
                                    agent.position.z += (dz / dist) * 0.2;
                                }
                            }
                        }
                    }
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
        }); // update_living_agents completes here, drops lock
        
        // ECONOMIC SYSTEM: Resource harvesting stores in agent inventory
        let mut agents = self.lifecycle.get_agents_mut(); // Now safe to acquire lock
        let resource_nodes = self.resources.get_nodes();
        
        for agent in agents.iter_mut() {
            // Only harvest if agent is Working near a resource node
            if matches!(agent.state, AgentState::Working { .. }) {
                let harvest_type = match agent.job {
                    Job::Woodcutter => Some(ResourceNodeType::Tree),
                    Job::Miner => Some(ResourceNodeType::Rock),
                    Job::Farmer => Some(ResourceNodeType::Farm),
                    _ => None,
                };
                
                if let Some(node_type) = harvest_type {
                    // Find nearest resource of the right type
                    if let Some(node) = resource_nodes.iter()
                        .filter(|n| n.resource_type == node_type && n.quantity > 0)
                        .min_by(|a, b| {
                            let dist_a = a.position.distance_to(&agent.position);
                            let dist_b = b.position.distance_to(&agent.position);
                            dist_a.partial_cmp(&dist_b).unwrap_or(std::cmp::Ordering::Equal)
                        })
                    {
                        let dist = node.position.distance_to(&agent.position);
                        
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
                                }
                            } else {
                                // Inventory full - go to market to sell
                                agent.state = AgentState::Idle;
                            }
                        }
                    }
                }
            }
        }
        drop(agents); // CRITICAL: Drop write lock before next section
        
        // ECONOMIC SYSTEM: Agent trading behavior at markets
        let mut markets_lock = self.markets.write();
        let agents = self.lifecycle.get_agents();
        
        for agent in agents.iter() {
            // Only trade if at a market (in Trading state)
            if matches!(agent.state, AgentState::Trading { .. }) {
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
                    } else {
                        // Not carrying - check if building needs resources
                        let remaining = building.remaining_resources();
                        
                        if !remaining.is_empty() && dist_to_building < 50.0 {
                            // Building needs resources - go get them from inventory or warehouse
                            let mut resources_to_carry = world_sim_agents::BuildingResources {
                                wood: 0,
                                stone: 0,
                                iron: 0,
                                target_building_id: building_id,
                            };
                            
                            // Take what we can from our inventory (max 20 per trip)
                            if let Some(needed_wood) = remaining.get(&world_sim_core::ResourceType::Wood) {
                                let take = (*needed_wood).min(20).min(*agent.inventory.get(&world_sim_core::ResourceType::Wood).unwrap_or(&0));
                                if take > 0 {
                                    resources_to_carry.wood = take;
                                    *agent.inventory.entry(world_sim_core::ResourceType::Wood).or_insert(0) -= take;
                                }
                            }
                            
                            if let Some(needed_stone) = remaining.get(&world_sim_core::ResourceType::Stone) {
                                let take = (*needed_stone).min(20).min(*agent.inventory.get(&world_sim_core::ResourceType::Stone).unwrap_or(&0));
                                if take > 0 {
                                    resources_to_carry.stone = take;
                                    *agent.inventory.entry(world_sim_core::ResourceType::Stone).or_insert(0) -= take;
                                }
                            }
                            
                            if let Some(needed_iron) = remaining.get(&world_sim_core::ResourceType::Iron) {
                                let take = (*needed_iron).min(10).min(*agent.inventory.get(&world_sim_core::ResourceType::Iron).unwrap_or(&0));
                                if take > 0 {
                                    resources_to_carry.iron = take;
                                    *agent.inventory.entry(world_sim_core::ResourceType::Iron).or_insert(0) -= take;
                                }
                            }
                            
                            // If we picked up any resources, start carrying them
                            if resources_to_carry.wood > 0 || resources_to_carry.stone > 0 || resources_to_carry.iron > 0 {
                                agent.carrying_resources = Some(resources_to_carry);
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
    
    /// Very slow tick (1/minute) - ecology, demographics, metrics
    pub async fn tick_very_slow(&mut self, _delta_seconds: f64) -> Result<()> {
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
        world_state.buildings = buildings.get_all_buildings()
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
    }
    
    /// HIERARCHICAL AI: Noble order execution (creates building orders)
    async fn process_noble_orders(&self) {
        let agents = self.lifecycle.get_agents();
        let mut kingdoms_write = self.kingdoms.write();
        
        for agent in agents.iter() {
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
                    
                    // Only create new orders occasionally (10% chance per minute)
                    if rng.gen::<f32>() < 0.1 {
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
                        
                        info!("🏛️ Noble {} orders construction of {:?} at ({:.1}, {:.1})", 
                              agent.name, building_type, location.x, location.z);
                        
                        // Create the actual building
                        let mut buildings = self.buildings.write();
                        let new_building = world_sim_world::Building::new(
                            building_type,
                            location,
                            format!("{:?} (Noble Order)", building_type),
                            world_sim_world::BuildingOwner::Public,
                        );
                        let building_id = new_building.id;
                        buildings.add_building(new_building);
                        
                        // Update order with building ID
                        if let Some(order_mut) = kingdoms_write.get_order_mut(order.id) {
                            order_mut.building_id = Some(building_id);
                            order_mut.status = world_sim_societal::OrderStatus::InProgress;
                        }
                    }
                }
            }
        }
    }
    
    /// HIERARCHICAL AI: Peasant self-building (personal needs)
    async fn process_peasant_building(&self) {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        
        let agents = self.lifecycle.get_agents();
        
        for agent in agents.iter() {
            if matches!(agent.social_class, world_sim_agents::SocialClass::Peasant) {
                // Peasants occasionally decide to build for themselves (5% chance per minute)
                if rng.gen::<f32>() < 0.05 {
                    // Check if they have a home nearby
                    let buildings = self.buildings.read();
                    let has_nearby_house = buildings.get_all_buildings().iter()
                        .any(|b| matches!(b.building_type, world_sim_world::BuildingType::PeasantHouse)
                            && b.position.distance_to(&agent.position) < 30.0);
                    
                    drop(buildings);
                    
                    if !has_nearby_house {
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
                        let house = world_sim_world::Building::new(
                            BuildingType::PeasantHouse,
                            location,
                            format!("{}'s House", agent.name),
                            world_sim_world::BuildingOwner::Agent(agent.id),
                        );
                        buildings.add_building(house);
                        
                        info!("🏠 Peasant {} decides to build a house at ({:.1}, {:.1})", 
                              agent.name, location.x, location.z);
                    } else if agent.job == Job::Farmer {
                        // Farmers build sheds
                        let has_nearby_shed = {
                            let buildings = self.buildings.read();
                            buildings.get_all_buildings().iter()
                                .any(|b| matches!(b.building_type, world_sim_world::BuildingType::FarmingShed)
                                    && b.position.distance_to(&agent.position) < 20.0)
                        };
                        
                        if !has_nearby_shed {
                            use world_sim_world::BuildingType;
                            
                            let offset_x = rng.gen_range(-8.0..8.0);
                            let offset_z = rng.gen_range(-8.0..8.0);
                            let location = Position::new(
                                agent.position.x + offset_x,
                                1.0,
                                agent.position.z + offset_z
                            );
                            
                            let mut buildings = self.buildings.write();
                            let shed = world_sim_world::Building::new(
                                BuildingType::FarmingShed,
                                location,
                                format!("{}'s Shed", agent.name),
                                world_sim_world::BuildingOwner::Agent(agent.id),
                            );
                            buildings.add_building(shed);
                            
                            info!("🌾 Farmer {} builds a farming shed at ({:.1}, {:.1})", 
                                  agent.name, location.x, location.z);
                        }
                    }
                }
            }
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

