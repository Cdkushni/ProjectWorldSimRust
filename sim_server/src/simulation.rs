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
        });
        
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
        
        // Now update each building
        for (building_id, building_pos) in incomplete_buildings {
            // Count how many builders are working on this building
            let builders_present = agents.iter().filter(|a| {
                matches!(a.state, AgentState::Building { .. }) 
                    && a.position.distance_to(&building_pos) < 5.0
            }).count();
            
            if builders_present > 0 {
                // Progress increases faster with more builders
                let progress_per_builder = 0.02; // 2% per builder per second
                let total_progress = progress_per_builder * builders_present as f32;
                
                let mut buildings = self.buildings.write();
                if let Some(b) = buildings.get_building_mut(building_id) {
                    let was_incomplete = !b.is_complete();
                    b.add_construction_progress(total_progress);
                    
                    if was_incomplete && b.is_complete() {
                        info!("🏗️ Building completed: {}", b.name);
                    }
                }
            }
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

