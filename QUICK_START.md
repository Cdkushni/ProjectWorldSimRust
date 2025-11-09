# Quick Start Guide

Get the simulation running in 5 minutes!

## Step 1: Install Rust (2 minutes)

### Windows
Download and run: https://win.rustup.rs/x86_64

### Linux/Mac
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

Verify:
```bash
cargo --version
```

## Step 2: Build the Project (1-3 minutes)

### Windows
```powershell
cd ProjectWorldSimRust
.\build.ps1
```

### Linux/Mac
```bash
cd ProjectWorldSimRust
chmod +x build.sh
./build.sh
```

## Step 3: Run the Server (30 seconds)

### Windows
```powershell
.\target\release\sim_server.exe
```

### Linux/Mac
```bash
./target/release/sim_server
```

You should see:
```
🌍 Starting World Simulation Server
✅ Simulation initialized
Generating initial world...
Spawning initial population...
Initial population: 100 agents
🌐 Admin API listening on http://127.0.0.1:8080
🚀 Simulation running
```

## Step 4: Test the API (30 seconds)

Open a new terminal:

```bash
# Check health
curl http://127.0.0.1:8080/health

# Get metrics
curl http://127.0.0.1:8080/api/metrics

# Inject a drought event
curl -X POST http://127.0.0.1:8080/api/dm/inject_event \
  -H "Content-Type: application/json" \
  -d '{"event_type":"DroughtStarted","payload":{"region":"global","severity":0.8,"expected_duration_days":30}}'
```

## Step 5: Watch It Run

The simulation will log activity:
```
Sim Time: 60s | Living Agents: 100
Sim Time: 120s | Living Agents: 101
```

To stop: Press `Ctrl+C`

## What's Happening?

1. **100 agents** spawn in a 100x100 world
2. **Seasons change** every 90 simulated days
3. **Weather** shifts randomly
4. **Trees grow** naturally
5. **Agents** are born and die
6. **Economy** adjusts prices based on supply/demand
7. **Dungeon Master** injects drama when things get boring

## Next Steps

### 1. Explore the API
- Read [API.md](API.md) for all endpoints
- Use `curl` or Postman to interact
- Inject custom events to see reactions

### 2. Connect a Visualizer
- Poll `/api/metrics` for agent count
- Implement `/api/world/state` endpoint (see DEVELOPMENT.md)
- Render agents in Unreal/Three.js

### 3. Customize the Simulation
- Edit `sim_server/src/simulation.rs` to change:
  - Initial agent count
  - World size
  - Tick rates
- Edit `crates/world/src/content.rs` to add:
  - New items
  - New actions
  - New recipes

### 4. Dive Deeper
- [ARCHITECTURE.md](ARCHITECTURE.md) - How it works
- [DEVELOPMENT.md](DEVELOPMENT.md) - How to extend it
- [PROJECT_SUMMARY.md](PROJECT_SUMMARY.md) - What's implemented

## Common Issues

### "cargo: command not found"
Restart your terminal after installing Rust.

### "Port 8080 already in use"
Edit `sim_server/src/main.rs` and change the port:
```rust
admin_api.serve("127.0.0.1:8081").await?;
```

### "Build takes too long"
First build takes 5-10 minutes to compile dependencies. Subsequent builds are much faster.

### "High CPU usage"
This is a simulation server - it's supposed to use CPU! To reduce:
1. Lower tick rates in `main.rs`
2. Reduce agent count in `simulation.rs`

## Visualizing the Simulation

### Three.js Example
```javascript
async function pollSimulation() {
  const response = await fetch('http://127.0.0.1:8080/api/metrics');
  const data = await response.json();
  
  // Update your visualization
  updateAgentCount(data.agent_count);
}

setInterval(pollSimulation, 1000);
```

### Unreal Engine Example
Use Unreal's HTTP module to poll the API and update your scene.

## Database (Optional)

To enable persistence:

1. Install PostgreSQL
2. Create database:
   ```sql
   CREATE DATABASE worldsim;
   ```
3. Set environment:
   ```bash
   export DATABASE_URL="postgresql://user:pass@localhost/worldsim"
   ```
4. Restart server

Event history and snapshots will now persist!

---

## 🎉 You're Ready!

The simulation is running. Now you can:
- ✅ Watch the logs for emergent behavior
- ✅ Inject events via the API
- ✅ Connect your game engine
- ✅ Extend the simulation with new features

**Welcome to your living, breathing world!** 🌍

