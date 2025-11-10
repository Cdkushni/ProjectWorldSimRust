# 🏰 Integrated Economy & Hierarchical Building System Design

## 📋 **System Overview:**

This document outlines a comprehensive integration of:
1. **Realistic Trading Economy** - Agents with wallets, inventory, buy/sell at markets
2. **Resource-Based Construction** - Buildings require materials delivered by builders
3. **Hierarchical Decision-Making** - Kings set goals, Nobles execute, Peasants build basics

---

## 💰 **Economic System Components:**

### **1. Agent Economics (IMPLEMENTED)**
```rust
pub struct SimAgent {
    pub wallet: f64,                           // Money owned
    pub inventory: HashMap<ResourceType, u32>, // Resources owned
    pub needs: HashMap<ResourceType, u32>,     // What they want to buy
    pub carrying_resources: Option<BuildingResources>, // Carrying to build site
}

pub struct BuildingResources {
    pub wood: u32,
    pub stone: u32,
    pub iron: u32,
    pub target_building_id: Uuid,
}
```

**Initial Wallets:**
- King: 1,000
- Noble: 500
- Knight: 200
- Merchant/Burgher: 150
- Soldier: 100
- Cleric: 80
- Peasant: 50

**Basic Needs (All Agents):**
- Food: 5 units

**Job-Specific Needs:**
- Builder: Wood 10, Stone 10
- Farmer: Wood 2 (tools)
- Woodcutter: Iron 1 (axe)
- Miner: Iron 1 (pickaxe)

### **2. Resource Harvesting Flow**
```
Worker → Harvest Resource → Store in Inventory → Go to Market → Sell
```

**Implementation:**
```rust
// When harvesting:
fn harvest_resource(agent, node) {
    let quantity = node.harvest(10);
    agent.inventory.insert(node.type, quantity);
    // Later at market, will sell excess
}
```

### **3. Market Trading Behavior**
```rust
// At market:
fn tick_trading(agent, market) {
    // BUY what needed
    for (resource, need) in &agent.needs {
        if agent.inventory.get(resource) < need {
            let max_price = calculate_buy_price(agent, resource);
            market.place_buy_order(resource, need, max_price);
        }
    }
    
    // SELL excess
    for (resource, quantity) in &agent.inventory {
        if quantity > need * 2 {
            let ask_price = calculate_sell_price(agent, resource);
            market.place_sell_order(resource, excess, ask_price);
        }
    }
}
```

### **4. Trade Execution**
```rust
// Every tick:
let trades = market.match_orders();
for trade in trades {
    buyer.wallet -= trade.total;
    seller.wallet += trade.total;
    seller.inventory.remove(trade.resource, trade.quantity);
    buyer.inventory.add(trade.resource, trade.quantity);
    currency.record_transaction(trade.total);
}
market.update_prices();
```

### **5. Wage System**
```rust
// Every hour (simulated):
for agent in workers {
    let wage = match agent.job {
        Job::Farmer => 5.0,
        Job::Woodcutter => 6.0,
        Job::Miner => 7.0,
        Job::Builder => 8.0,
        _ => 0.0,
    };
    
    agent.wallet += wage;
    currency.mint_currency(wage); // Creates inflation
}
```

---

## 🏗️ **Resource-Based Building System:**

### **1. Building Resource Requirements**
```rust
pub struct BuildingType {
    Warehouse => (wood: 100, stone: 50, iron: 20),
    Barracks => (wood: 80, stone: 60, iron: 30),
    Workshop => (wood: 60, stone: 40, iron: 15),
    Farm => (wood: 40, stone: 20, iron: 5),
    PeasantHouse => (wood: 30, stone: 10, iron: 0),
    NobleEstate => (wood: 150, stone: 100, iron: 40),
    Walls => (wood: 20, stone: 200, iron: 50),
}
```

### **2. Builder Resource Retrieval**
```rust
// Builder behavior:
if agent.job == Builder && building.incomplete {
    if agent.carrying_resources.is_none() {
        // Go to warehouse/market to get resources
        let resources_needed = building.remaining_resources();
        
        // Check inventory first
        if agent.has_resources(resources_needed) {
            agent.carrying_resources = Some(BuildingResources {
                wood: take_from_inventory(Wood),
                stone: take_from_inventory(Stone),
                iron: take_from_inventory(Iron),
                target_building_id: building.id,
            });
        } else {
            // Go to warehouse
            move_to_nearest_warehouse();
            // At warehouse, withdraw resources
            warehouse.withdraw(resources_needed, agent);
        }
    } else {
        // Carrying resources - go to build site
        move_to_building();
        
        if at_building() {
            // Deliver resources
            building.add_resources(agent.carrying_resources);
            agent.carrying_resources = None;
            
            // Work on building (consumes resources)
            building.construct_with_resources(0.02);
        }
    }
}
```

### **3. Resource Consumption**
```rust
pub struct Building {
    pub required_resources: HashMap<ResourceType, u32>,
    pub current_resources: HashMap<ResourceType, u32>,
    pub construction_progress: f32,
}

fn construct_with_resources(&mut self, rate: f32) -> bool {
    // Check if we have enough resources for this tick
    let needed_wood = (self.required.wood * rate) as u32;
    let needed_stone = (self.required.stone * rate) as u32;
    
    if self.current.wood >= needed_wood 
        && self.current.stone >= needed_stone {
        // Consume resources
        self.current.wood -= needed_wood;
        self.current.stone -= needed_stone;
        
        // Increase progress
        self.construction_progress += rate;
        return true;
    }
    
    false // Not enough resources
}
```

---

## 👑 **Hierarchical Decision System:**

### **1. Kingdom Goals (King Level)**
```rust
pub enum KingdomGoal {
    DefendTerritory,      // Build walls, barracks
    ExpandResources,      // Build mines, farms
    PrepareForWar,        // Build barracks, weapons
    GrowPopulation,       // Build houses, farms
    ImproveInfrastructure, // Build markets, workshops
}

pub struct Kingdom {
    pub king_id: AgentId,
    pub nobles: Vec<AgentId>,
    pub current_goal: KingdomGoal,
    pub goal_priority: f32,
}

// King AI (runs every minute):
fn king_decide_goal(king, kingdom, world_state) -> KingdomGoal {
    let food_per_capita = calculate_food_per_capita();
    let enemies_nearby = count_enemy_factions_nearby();
    let resource_stress = calculate_resource_stress();
    
    if enemies_nearby > 0 || at_war() {
        KingdomGoal::DefendTerritory
    } else if food_per_capita < 10.0 {
        KingdomGoal::GrowPopulation
    } else if resource_stress > 0.7 {
        KingdomGoal::ExpandResources
    } else {
        KingdomGoal::ImproveInfrastructure
    }
}
```

### **2. Noble Orders (Noble Level)**
```rust
pub struct NobleOrder {
    pub building_type: BuildingType,
    pub location: Position,
    pub priority: f32,
    pub assigned_builders: Vec<AgentId>,
}

// Noble AI (executes King's goals):
fn noble_execute_goal(noble, kingdom_goal) -> Vec<NobleOrder> {
    match kingdom_goal {
        KingdomGoal::DefendTerritory => {
            vec![
                NobleOrder {
                    building_type: BuildingType::Walls,
                    location: find_border_position(),
                    priority: 1.0,
                },
                NobleOrder {
                    building_type: BuildingType::Barracks,
                    location: find_central_position(),
                    priority: 0.8,
                },
            ]
        },
        KingdomGoal::ExpandResources => {
            vec![
                NobleOrder {
                    building_type: BuildingType::Farm,
                    location: find_fertile_land(),
                    priority: 1.0,
                },
                NobleOrder {
                    building_type: BuildingType::Mine,
                    location: find_resource_rich_area(),
                    priority: 0.9,
                },
            ]
        },
        // ... other goals
    }
}
```

### **3. Peasant Self-Building**
```rust
// Peasants build for personal needs:
fn peasant_decide_building(peasant) -> Option<BuildingType> {
    if !peasant.has_home() {
        Some(BuildingType::PeasantHouse)
    } else if peasant.job == Farmer && !has_nearby_shed() {
        Some(BuildingType::FarmingShed)
    } else {
        None
    }
}

// Peasants can build:
- PeasantHouse (30 wood, 10 stone)
- FarmingShed (20 wood, 5 stone)
- SmallWorkshop (40 wood, 15 stone)
```

### **4. Building Permissions**
```rust
fn can_order_building(agent, building_type) -> bool {
    match (agent.social_class, building_type) {
        (King, _) => true,  // Can order anything
        (Noble, Barracks | Walls | Market | Workshop) => true,
        (Noble, NobleEstate) => true,  // For themselves
        (Peasant, PeasantHouse | FarmingShed) => true,
        (Merchant, Workshop | Market) => true,
        (Burgher, Workshop) => true,
        _ => false,
    }
}
```

---

## 🔄 **Complete Flow Examples:**

### **Example 1: King Orders Defense**
```
1. War threat detected
2. King sets goal: DefendTerritory
3. Noble receives goal, creates orders:
   - Build Walls at (50, 0, 50)
   - Build Barracks at (0, 0, 0)
4. Nobles assign builders to orders
5. Builders check requirements:
   - Walls need: 200 stone, 20 wood, 50 iron
6. Builders go to warehouse
7. Warehouse has insufficient stone
8. Miners prioritized to gather stone
9. Stone arrives at warehouse
10. Builders retrieve 50 stone each (4 trips)
11. Builders deliver to wall site
12. Construction begins, consuming resources
13. Walls complete after 500 builder-seconds
```

### **Example 2: Peasant Builds House**
```
1. Peasant has no home
2. Decides to build PeasantHouse
3. Needs: 30 wood, 10 stone
4. Checks inventory: 15 wood, 5 stone (not enough)
5. Goes to market
6. Places buy order: 15 wood @ 6.0 each
7. Places buy order: 5 stone @ 4.0 each
8. Market matches with sellers
9. Trade executes: pays 120 money
10. Receives resources in inventory
11. Goes to chosen location
12. Creates building (incomplete)
13. Delivers own resources to site
14. Works on building (as builder)
15. House complete after 100 seconds
```

### **Example 3: Resource Trade Cycle**
```
1. Woodcutter harvests 50 wood
2. Stores in inventory
3. Travels to market
4. Places sell order: 50 wood @ 5.5 each
5. Builder needs 20 wood
6. Places buy order: 20 wood @ 6.0 each
7. Market matches orders
8. Trade executes @ 5.75 average:
   - Woodcutter receives 115 money
   - Builder pays 115 money
   - Builder receives 20 wood
9. Woodcutter inventory: 30 wood
10. Builder inventory: 20 wood
11. Market price updates: 5.0 → 5.3 (demand increased)
12. Currency system records 115 transaction
```

---

## 📊 **Implementation Priority:**

### **Phase 1: Economic Foundation (DONE)**
- ✅ Add wallet/inventory/needs to Agent
- ✅ Initialize based on social class
- ✅ Export new types

### **Phase 2: Resource Flow (NEXT)**
- [ ] Make harvesting store in inventory
- [ ] Implement trading behavior at markets
- [ ] Add market order matching to simulation tick
- [ ] Implement trade execution
- [ ] Add wage system

### **Phase 3: Resource-Based Building**
- [ ] Add resource requirements to buildings
- [ ] Implement builder resource retrieval
- [ ] Add resource delivery to build sites
- [ ] Make construction consume resources
- [ ] Add carrying capacity limits

### **Phase 4: Hierarchical Decisions**
- [ ] Create Kingdom struct with goals
- [ ] Implement King AI
- [ ] Create Noble order system
- [ ] Implement Noble AI
- [ ] Add peasant self-building logic
- [ ] Implement building permissions

---

## 🎯 **Key Design Principles:**

1. **Emergent Complexity** - Simple rules → Complex behavior
2. **Resource Scarcity** - Limited resources drive economy
3. **Class-Based Behavior** - Different roles, different actions
4. **Hierarchical Command** - Top-down but with autonomy
5. **Economic Realism** - Supply/demand, prices, scarcity
6. **Visible Progress** - Players see every step

---

## 🚀 **Expected Outcomes:**

**Economic Dynamics:**
- Real supply/demand affecting prices
- Resource shortages causing conflicts
- Wealth accumulation by merchants
- Class disparities emerging

**Construction Realism:**
- Visible resource delivery
- Builders carrying materials
- Construction halts without resources
- Multiple builders speeding work

**Strategic Depth:**
- Kings making macro decisions
- Nobles executing tactical plans
- Peasants meeting personal needs
- Emergent civilization patterns

---

*This system will create a living, breathing medieval economy with realistic resource flows, market dynamics, and hierarchical decision-making!*

