# Development Guide

## Setting Up Development Environment

### Prerequisites
- Rust 1.70+ (`rustup update`)
- PostgreSQL 14+ (optional)
- VS Code with rust-analyzer (recommended)

### Clone and Build
```bash
git clone <repository>
cd ProjectWorldSimRust
cargo build
```

### Environment Setup
Create a `.env` file (optional):
```
DATABASE_URL=postgresql://localhost/worldsim
RUST_LOG=debug
```

## Project Structure

```
ProjectWorldSimRust/
├── crates/                    # All library crates
│   ├── core/                 # Shared types (Position, BlockType, etc.)
│   ├── event_bus/            # Event system
│   ├── persistence/          # Database layer
│   ├── admin_api/            # HTTP API
│   ├── world/                # Grid, ecology, content
│   ├── agents/               # Agent definitions
│   ├── cognitive/            # AI systems
│   ├── societal/             # Social, economy, politics
│   └── meta/                 # Dungeon Master
├── sim_server/               # Main binary
└── Cargo.toml                # Workspace root
```

## Development Workflow

### Running the Server
```bash
# Development mode (with logging)
RUST_LOG=debug cargo run --bin sim_server

# Release mode
cargo run --release --bin sim_server
```

### Running Tests
```bash
# All tests
cargo test --workspace

# Specific crate
cargo test -p world_sim_core

# With output
cargo test -- --nocapture
```

### Linting and Formatting
```bash
# Format code
cargo fmt

# Lint
cargo clippy -- -D warnings
```

## Adding New Features

### 1. Adding a New Event Type

**Step 1:** Define the event in `crates/event_bus/src/events.rs`:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MyNewEvent {
    pub data: String,
}

impl Event for MyNewEvent {
    fn event_type(&self) -> &'static str {
        "MyNew"
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
```

**Step 2:** Publish from a system:
```rust
event_bus.publish(&MyNewEvent {
    data: "Hello".to_string()
}).await;
```

**Step 3:** Subscribe in a system:
```rust
#[async_trait]
impl EventSubscriber for MySystem {
    async fn on_event(&self, event: &EventEnvelope) {
        if event.event_type == "MyNew" {
            // Handle event
        }
    }
}
```

### 2. Adding a New GOAP Action

**Step 1:** Define in `crates/world/src/content.rs`:
```rust
actions.insert(
    "my_action".to_string(),
    ActionDefinition {
        id: "my_action".to_string(),
        name: "My Action".to_string(),
        base_cost: 5.0,
        intended_use: 75,
        required_skill: Some((Skill::Crafting, 10.0)),
        preconditions: vec!["HasTool".to_string()],
        effects: vec!["TaskComplete".to_string()],
    },
);
```

**Step 2:** Agents will automatically discover this action during planning.

### 3. Adding a New Personality Trait

**Step 1:** Define in `crates/core/src/types.rs`:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Trait {
    // ... existing traits
    MyNewTrait,
}
```

**Step 2:** Add behavior modifier:
```rust
impl Trait {
    pub fn action_cost_modifier(&self, action_type: &str) -> f32 {
        match (self, action_type) {
            (Trait::MyNewTrait, "MyAction") => 0.5,
            _ => 1.0,
        }
    }
}
```

### 4. Adding a New Subsystem

**Step 1:** Create module in appropriate crate:
```rust
// crates/world/src/my_subsystem.rs
pub struct MySubsystem {
    data: Vec<Something>,
}

impl MySubsystem {
    pub fn new() -> Self {
        Self { data: Vec::new() }
    }
    
    pub fn tick(&mut self) {
        // Update logic
    }
}
```

**Step 2:** Integrate in `sim_server/src/simulation.rs`:
```rust
pub struct Simulation {
    // ... existing fields
    my_subsystem: MySubsystem,
}

impl Simulation {
    pub async fn new() -> Result<Self> {
        // ... existing code
        let my_subsystem = MySubsystem::new();
        
        Ok(Self {
            // ... existing fields
            my_subsystem,
        })
    }
    
    pub async fn tick_slow(&mut self, delta: f64) -> Result<()> {
        // ... existing code
        self.my_subsystem.tick();
        Ok(())
    }
}
```

## Testing Strategies

### Unit Tests
Test individual components:
```rust
#[test]
fn test_skill_leveling() {
    let mut db = SkillDatabase::new();
    db.add_experience(Skill::Mining, 100.0);
    assert!(db.get_level(Skill::Mining) > 0.0);
}
```

### Integration Tests
Test system interactions:
```rust
#[tokio::test]
async fn test_economy_reacts_to_blight() {
    let event_bus = Arc::new(EventBus::new());
    let economy = EconomySubsystem::new(event_bus.clone());
    
    economy.update_supply(ResourceType::Wood, 100);
    event_bus.publish(&BlightStartedEvent { /* ... */ }).await;
    
    // Verify price increased
    assert!(economy.get_price(ResourceType::Wood) > initial_price);
}
```

### End-to-End Tests
Test full simulation loop:
```rust
#[tokio::test]
async fn test_full_simulation_cycle() {
    let mut sim = Simulation::new().await.unwrap();
    
    // Run for 10 seconds
    for _ in 0..10 {
        sim.tick_slow(1.0).await.unwrap();
    }
    
    // Verify agents are still alive
    assert!(sim.lifecycle.count_living() > 0);
}
```

## Performance Profiling

### Using Criterion (benchmarks)
Create `benches/my_benchmark.rs`:
```rust
use criterion::{criterion_group, criterion_main, Criterion};

fn benchmark_pathfinding(c: &mut Criterion) {
    c.bench_function("pathfinding 100 nodes", |b| {
        b.iter(|| {
            // Your benchmark code
        });
    });
}

criterion_group!(benches, benchmark_pathfinding);
criterion_main!(benches);
```

Run: `cargo bench`

### Using Flamegraph
```bash
cargo install flamegraph
sudo cargo flamegraph --bin sim_server
```

## Debugging Tips

### Enable Detailed Logging
```bash
RUST_LOG=world_sim_cognitive=trace,world_sim_agents=debug cargo run
```

### Use the Admin API
Query agent state:
```bash
curl http://localhost:8080/api/agent/{agent_id}
```

Inject test events:
```bash
curl -X POST http://localhost:8080/api/dm/inject_event \
  -H "Content-Type: application/json" \
  -d '{"event_type": "BlightStarted", "payload": {...}}'
```

### Inspecting Database
```sql
-- View recent events
SELECT * FROM event_history ORDER BY timestamp DESC LIMIT 10;

-- Count events by type
SELECT event_type, COUNT(*) FROM event_history GROUP BY event_type;

-- View world snapshots
SELECT id, name, timestamp FROM world_snapshots ORDER BY timestamp DESC;
```

## Common Issues

### Issue: "Cannot borrow as mutable"
**Cause:** Rust ownership rules
**Solution:** Use `Arc<RwLock<T>>` for shared mutable state

### Issue: "Future cannot be sent between threads"
**Cause:** Using non-Send types in async code
**Solution:** Wrap in `Arc` or use `tokio::spawn_blocking`

### Issue: High memory usage
**Cause:** Too many loaded chunks or agents
**Solution:** Implement chunk unloading and AI LOD system

### Issue: GOAP planner times out
**Cause:** Action space too large
**Solution:** Reduce `max_iterations` or improve heuristic

## Code Style Guidelines

1. **Use descriptive names**: `calculate_path()` not `calc()`
2. **Document public APIs**: Add doc comments to public functions
3. **Keep functions small**: Aim for < 50 lines
4. **Prefer immutability**: Use `let` not `let mut` when possible
5. **Use Result<T, E>**: For fallible operations
6. **Follow Rust naming conventions**:
   - Types: `PascalCase`
   - Functions/variables: `snake_case`
   - Constants: `SCREAMING_SNAKE_CASE`

## Release Checklist

- [ ] All tests pass (`cargo test --workspace`)
- [ ] No clippy warnings (`cargo clippy -- -D warnings`)
- [ ] Code is formatted (`cargo fmt`)
- [ ] Documentation is updated
- [ ] CHANGELOG.md is updated
- [ ] Version bumped in Cargo.toml
- [ ] Release builds successfully (`cargo build --release`)
- [ ] Binary tested on target platform

## Resources

- [Rust Book](https://doc.rust-lang.org/book/)
- [Async Rust](https://rust-lang.github.io/async-book/)
- [Tokio Tutorial](https://tokio.rs/tokio/tutorial)
- [GOAP Introduction](https://gamedevelopment.tutsplus.com/goal-oriented-action-planning-for-a-smarter-ai--cms-20793t)

---

**Happy coding!** 🦀

