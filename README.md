# 🌍 Dynamic World Simulation Server

A comprehensive, headless world simulation engine built in Rust, designed to power games built with Unreal Engine, Three.js, or other visualization platforms.

## 🎯 Project Philosophy

This is a **decoupled, headless simulation** where:
- The simulation is the **single source of truth** for all world state
- Visualizers are "dumb clients" that connect via API
- Agent behavior is **emergent** from simple, data-driven systems
- Everything is **event-driven** and **interconnected**

## 🏗️ Architecture Overview

### Core Infrastructure Layer
- **Event Bus**: Pub/sub system for macro-level communication
- **Persistence**: PostgreSQL-based save/load system with event history
- **Admin API**: HTTP/WebSocket server for external control and monitoring

### World Layer
- **Grid System**: Voxel-based 3D world with chunk-based optimization
- **Ecology**: Seasons, weather, resource lifecycle, and fauna
- **Content Definition**: Central database of actions, items, recipes, and traits
- **Pathfinding**: A* with hierarchical optimization (HPA*)

### Agent Layer
- **Lifecycle**: Birth, death, and population management
- **Skills & Knowledge**: Progression system with gated information
- **Personality**: Traits, beliefs, and attributes that shape behavior
- **Ownership**: Global registry and personal domains

### Cognitive Layer
- **Perception**: Sight and hearing simulation (no cheating!)
- **Utility AI**: Emotional engine that decides what agents want
- **GOAP**: Goal-Oriented Action Planning with regressive A* search

### Societal Layer
- **Social**: Relationship management and memory system
- **Economy**: Dynamic supply/demand market with event-driven prices
- **Politics**: Factions, territory, and diplomatic relations

### Meta Layer
- **Dungeon Master**: AI director that injects drama when the world gets boring

## 🚀 Getting Started

### Prerequisites

- **Rust** (1.70+): Install from [rustup.rs](https://rustup.rs/)
- **PostgreSQL** (Optional): For persistence

### Quick Start (Windows)

```powershell
# Clone the repository
git clone <repository-url>
cd ProjectWorldSimRust

# Build the project
.\build.ps1

# Run the server
.\target\release\sim_server.exe
```

### Quick Start (Linux/Mac)

```bash
# Clone the repository
git clone <repository-url>
cd ProjectWorldSimRust

# Make scripts executable
chmod +x build.sh test.sh

# Build the project
./build.sh

# Run the server
./target/release/sim_server
```

### Configuration

Copy `.env.example` to `.env` and configure:

```bash
# Optional: Enable database persistence
DATABASE_URL=postgresql://user:password@localhost/worldsim

# Log level
RUST_LOG=info
```

## 🧪 Running Tests

```bash
# Windows
.\test.ps1

# Linux/Mac
./test.sh
```

## 🌐 Admin API

Once running, the Admin API is available at `http://127.0.0.1:8080`

### Key Endpoints

- `GET /health` - Health check
- `GET /api/history` - Query event history
- `POST /api/dm/inject_event` - Inject custom events (Dungeon Master)
- `POST /api/agent/:id/add_memory` - Add false memories to agents
- `GET /api/world/snapshot` - Create world snapshot
- `GET /api/metrics` - Get simulation metrics

### Example: Inject a Drought Event

```bash
curl -X POST http://127.0.0.1:8080/api/dm/inject_event \
  -H "Content-Type: application/json" \
  -d '{
    "event_type": "DroughtStarted",
    "payload": {
      "region": "global",
      "severity": 0.8,
      "expected_duration_days": 30
    }
  }'
```

## 📦 Project Structure

```
ProjectWorldSimRust/
├── crates/
│   ├── core/              # Shared types and utilities
│   ├── event_bus/         # Event system
│   ├── persistence/       # Database layer
│   ├── admin_api/         # HTTP API server
│   ├── world/             # Grid, ecology, content
│   ├── agents/            # Agent definitions
│   ├── cognitive/         # AI systems
│   ├── societal/          # Social, economy, politics
│   └── meta/              # Dungeon Master
├── sim_server/            # Main server binary
├── Cargo.toml             # Workspace configuration
└── README.md
```

## 🎮 Connecting a Visualizer

### For Unreal Engine

1. Use HTTP REST client to poll `/api/world/state` (implement this endpoint)
2. Subscribe to WebSocket for real-time events
3. Send player input via `/api/player/:id/action`

### For Three.js

1. Use `fetch()` or WebSocket to connect to the API
2. Render the world based on grid data
3. Display agents at their positions

## 🔧 Optimization Features

### AI Level of Detail (LOD)
- **Tier 1 (Bubble)**: Statistical simulation only
- **Tier 2 (Gist)**: Utility AI only
- **Tier 3 (Full)**: Full GOAP planning

### Hierarchical Pathfinding (HPA*)
- Chunk-based abstraction
- Path caching
- Time-sliced computation

### Staggered Ticks
- **10Hz**: Perception, GOAP
- **1Hz**: Economy, Utility AI
- **1/min**: Ecology, Demographics

## 🎯 Key Design Decisions

### Why Rust over C++?
- **Memory safety** without garbage collection
- **Fearless concurrency** with ownership system
- **Modern tooling** (Cargo, clippy, rustfmt)
- **Performance** on par with C++

### Why Regressive GOAP?
- More efficient than forward search
- Natural fit for goal-driven behavior
- Better heuristics through "intended use" scores

### Why Event Bus?
- Decouples all systems
- Easy to add new systems
- Natural fit for save/replay
- Enables AI "storytelling"

## 📚 Further Reading

### Core Concepts
- [GOAP (Goal-Oriented Action Planning)](https://en.wikipedia.org/wiki/Goal-oriented_action_planning)
- [Utility AI](https://www.gdcvault.com/play/1012410/Improving-AI-Decision-Modeling-Through)
- [Event-Driven Architecture](https://en.wikipedia.org/wiki/Event-driven_architecture)

### Optimization Techniques
- [HPA* (Hierarchical Pathfinding)](https://en.wikipedia.org/wiki/HPA*)
- [Level of Detail (AI)](https://en.wikipedia.org/wiki/Level_of_detail_(computer_graphics))

## 🤝 Contributing

This is a blueprint implementation. Key areas for expansion:

1. **Expression System**: Implement the procedural precondition/effect system
2. **AI LOD**: Complete the tier system for agent simulation
3. **Visualizer Protocol**: Design the client-server communication protocol
4. **Content Editor**: Build the web-based GOAP expression editor
5. **More Events**: Expand the Dungeon Master's story event library

## 📝 License

MIT License - See LICENSE file for details

## 🙏 Acknowledgments

Built following the principles outlined in the project blueprint, inspired by:
- The Sims' autonomous behavior systems
- Dwarf Fortress's emergent storytelling
- F.E.A.R.'s GOAP implementation
- Modern ECS architectures

---

**Status**: ✅ Core implementation complete and buildable

For questions or issues, please open a GitHub issue.

