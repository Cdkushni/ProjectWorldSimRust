# Architecture Deep Dive

## System Layers

### 1. Core Infrastructure

#### Event Bus
The Event Bus is the central nervous system. It uses a publish-subscribe pattern where:
- **Publishers**: Any system can publish events (Economy, Ecology, Dungeon Master)
- **Subscribers**: Only systems subscribe (not individual agents)
- **Event History**: All events are stored in PostgreSQL for replay and analysis

**Key Events:**
- `PriceChangeEvent` - Economy announces price changes
- `WarDeclaredEvent` - Politics announces conflicts
- `BlightStartedEvent` - Ecology/DM announces disasters
- `AgentDiedEvent` - Lifecycle announces deaths

#### Persistence Layer
Two-table approach:
1. **event_history**: Immutable "True History" (append-only)
2. **world_snapshots**: Full world state serialization

This enables:
- Time travel debugging
- AI training on historical data
- Conflict detection (subjective vs objective truth)

### 2. World Layer

#### Grid System
- **Chunk-based**: 32³ blocks per chunk
- **Dynamic loading**: Only active chunks in memory
- **Block types**: Air, Stone, Wood, Water, etc.
- **Dynamic objects**: Ships, catapults (separate from voxels)

#### Ecology
Four subsystems:
1. **Seasons**: 90-day cycles affecting growth
2. **Weather**: State machine (Clear → Rain → Drought → Storm)
3. **Resource Lifecycle**: Natural growth/decay (Grass → Tree)
4. **Fauna**: Simple non-GOAP animal agents

### 3. Agent Layer

#### Agent Definition
Each `SimAgent` has:
- **Attributes**: Strength, Intelligence, Charisma, etc.
- **Personality**: Traits + Beliefs
- **Skills**: Leveling system (XP-based)
- **Knowledge**: Gated information (recipes, secrets)
- **Domain**: Personal space and social network

#### Ownership System
Two-level system:
1. **Global Registry**: TMap<ItemId, AgentId> (who owns what)
2. **Agent Domain**: Personal reference (my home, my friends)

### 4. Cognitive Layer

#### Perception
**Anti-Cheating Filter:**
- Agents have `sight_radius` and `hearing_radius`
- They maintain a "Known World" cache
- GOAP/Utility can ONLY read from Known World, not True World
- Creates information-gathering gameplay

#### Utility AI
**Emotional Engine:**
```
Urge Score = sigmoid(current_value - 5.0) * weight * trait_modifier
```
- Updates continuously
- Personality traits modify weights
- Outputs a single Goal for GOAP

#### GOAP (Goal-Oriented Action Planning)
**Regressive A* Search:**
1. Start from goal state
2. Work backwards using action effects
3. Stop when current state is reached
4. Return action sequence

**Optimizations:**
- Intended Use heuristic (designer-defined relevance)
- Plan caching (execute until invalidated)
- Expression System (contextual preconditions)

### 5. Societal Layer

#### Social System
Two managers:
1. **RelationshipManager**: Affinity (-100 to +100) + Trust (0-100)
2. **MemoryManager**: Facts with source tracking (Witnessed, Told, Fabricated)

#### Economy
**Supply/Demand Model:**
```
new_price = old_price * (0.9 + (demand / supply) * 0.2)
```
- Recalculated every second
- Reacts to events (Blight → Supply drops → Price rises)
- Publishes PriceChangeEvent when significant

#### Politics
**Faction System:**
- Leader + Members + Policies
- Relations: Allied, Friendly, Neutral, Hostile, War
- Territory: Chunk-based ownership
- Tax rates modify economic actions

### 6. Meta Layer

#### Dungeon Master
**Boredom Detection:**
```
boredom = 0.0
if price_volatility < 0.1: boredom += 0.3
if active_conflicts == 0: boredom += 0.2
if time_since_event > 300s: boredom += 0.3
if agent_activity < 0.3: boredom += 0.2
```

**Story Events:**
- Blight (resource scarcity)
- Drought (environmental)
- Plague (population control)
- Uprising (political drama)
- Earthquake (destruction)
- Discovery (opportunity)

## Data Flow Examples

### Example 1: Hunger → Eating
1. **Utility AI**: `Urge_Hunger` increases over time
2. **Utility AI**: Outputs `Goal: NotHungry`
3. **GOAP**: Plans backwards from "NotHungry"
   - Effect "NotHungry" ← Action "Eat"
   - Precondition "HasFood" ← Action "GetFood"
4. **GOAP**: Returns plan: [`GetFood`, `Eat`]
5. **Agent**: Executes actions

### Example 2: Blight Event Cascade
1. **Dungeon Master**: Detects boredom → Injects `BlightStartedEvent`
2. **Event Bus**: Publishes to all subscribers
3. **Economy**: Receives event → Reduces Wood supply → Recalculates prices
4. **Economy**: Publishes `PriceChangeEvent` (Wood price up)
5. **Agents**: GOAP sees high Wood price → Changes job to Woodcutting
6. **Social**: Agents compete for scarce resources → Conflicts arise

### Example 3: Perception & Information
1. **Agent A**: Steals from Agent B
2. **StimulusSubsystem**: Broadcasts `Visual: StealAction`
3. **Agent C**: In sight radius → Perceives event
4. **Agent C**: Adds to Known World: "A stole from B"
5. **Social Layer**: C's affinity towards A decreases
6. **GOAP**: C may plan to report or retaliate

## Performance Optimization

### AI Level of Detail (LOD)
| Tier | Distance | Systems | Agent Count |
|------|----------|---------|-------------|
| Bubble | Far | Statistical only | 90% |
| Gist | Medium | Utility AI | 9% |
| Full | Near | GOAP + Utility + Perception | 1% |

### Staggered Update Rates
| System | Update Rate | Reason |
|--------|-------------|--------|
| Perception | 10 Hz | Real-time feedback |
| GOAP | 10 Hz | Responsive AI |
| Utility AI | 1 Hz | Emotional changes are slow |
| Economy | 1 Hz | Market fluctuations |
| Ecology | 1/min | Natural processes |
| Demographics | 1/min | Population changes |

### Memory Layout
- **Hot path**: Agent position, state → Cache-friendly
- **Cold path**: Personality, memories → Pointer-indirect
- **Chunk-based spatial partitioning** for collision/perception

## Extension Points

### 1. Expression System (Not Yet Implemented)
Replace hard-coded preconditions with:
```rust
ActionPrecondition::Expression {
    check: Box<dyn Fn(&WorldState) -> bool>
}
```

### 2. AI Training
Event history → RL training data:
- State: World + Agent perception
- Action: GOAP plan chosen
- Reward: Goal satisfaction + survival

### 3. Procedural Content
- Template-based action generation
- Emergent recipes (combine A+B, see what happens)
- Dynamic trait effects

### 4. Multiplayer
- Players are special agents
- Input → Admin API → Agent actions
- Broadcast state changes via WebSocket

## Design Principles Applied

### 1. Single Source of Truth
The simulation owns all state. Visualizers request data, never modify.

### 2. Data-Oriented
Systems operate on data, not objects. Traits for shared behavior.

### 3. Event-Driven
No polling. Systems react to events.

### 4. Emergent Behavior
No scripting. Complexity from simple rules.

### 5. Optimizable
Hot/cold split, LOD, staggered ticks.

---

**Key Insight:** Every design choice optimizes for either:
1. **Emergent Storytelling** (event-driven, GOAP, social)
2. **Performance at Scale** (LOD, chunks, staggered)
3. **Designer Control** (Dungeon Master, Admin API, content)

