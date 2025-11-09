# Setup Guide

## Prerequisites

### 1. Install Rust

**Windows:**
1. Download rustup from [rustup.rs](https://rustup.rs/)
2. Run the installer and follow prompts
3. Restart your terminal
4. Verify: `cargo --version`

**Linux/Mac:**
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
cargo --version
```

### 2. Install PostgreSQL (Optional)

**Windows:**
1. Download from [postgresql.org](https://www.postgresql.org/download/windows/)
2. Run installer
3. Create database: 
   ```sql
   CREATE DATABASE worldsim;
   ```

**Linux:**
```bash
sudo apt install postgresql postgresql-contrib
sudo -u postgres createdb worldsim
```

**Mac:**
```bash
brew install postgresql
brew services start postgresql
createdb worldsim
```

## Building the Project

### Option 1: Using Build Scripts (Recommended)

**Windows:**
```powershell
.\build.ps1
```

**Linux/Mac:**
```bash
chmod +x build.sh
./build.sh
```

### Option 2: Manual Build

```bash
# Development build (faster compilation, slower runtime)
cargo build

# Release build (slower compilation, faster runtime)
cargo build --release
```

## Configuration

### 1. Environment Variables

Create a `.env` file in the project root (optional):

```env
# Database (optional - omit to run without persistence)
DATABASE_URL=postgresql://username:password@localhost/worldsim

# Logging
RUST_LOG=info

# API Settings
API_HOST=127.0.0.1
API_PORT=8080
```

**Log Levels:**
- `error` - Only errors
- `warn` - Warnings and errors
- `info` - General information (recommended)
- `debug` - Detailed debugging info
- `trace` - Very verbose debugging

**Per-module logging:**
```env
RUST_LOG=world_sim_agents=debug,world_sim_cognitive=trace,info
```

### 2. Database Setup (If Using PostgreSQL)

```bash
# Set environment variable
export DATABASE_URL="postgresql://username:password@localhost/worldsim"

# On Windows PowerShell:
$env:DATABASE_URL="postgresql://username:password@localhost/worldsim"
```

The database schema will be created automatically on first run.

## Running the Server

### Development Mode
```bash
cargo run --bin sim_server
```

### Release Mode (Faster)
```bash
cargo run --release --bin sim_server
```

### Running the Built Binary

**Windows:**
```powershell
.\target\release\sim_server.exe
```

**Linux/Mac:**
```bash
./target/release/sim_server
```

## Verifying the Installation

### 1. Check Server is Running

You should see output like:
```
2023-11-09T12:00:00Z INFO  sim_server: 🌍 Starting World Simulation Server
2023-11-09T12:00:00Z INFO  sim_server: ✅ Simulation initialized
2023-11-09T12:00:00Z INFO  sim_server: 🌐 Admin API listening on http://127.0.0.1:8080
2023-11-09T12:00:00Z INFO  sim_server: 🚀 Simulation running
```

### 2. Test the API

```bash
curl http://127.0.0.1:8080/health
```

Should return: `OK`

### 3. View Metrics

```bash
curl http://127.0.0.1:8080/api/metrics
```

## Troubleshooting

### Issue: "cargo: command not found"
**Solution:** Restart your terminal after installing Rust, or manually source the environment:
```bash
source $HOME/.cargo/env
```

### Issue: "could not compile due to X errors"
**Solution:** Make sure you have the latest Rust version:
```bash
rustup update
```

### Issue: "connection refused" when accessing API
**Solution:** 
1. Check server is running
2. Verify port 8080 is not in use
3. Check firewall settings

### Issue: Database connection errors
**Solution:**
1. Verify PostgreSQL is running
2. Check DATABASE_URL format
3. Ensure database exists
4. Verify credentials

To run without database:
```bash
unset DATABASE_URL  # Linux/Mac
Remove-Item Env:DATABASE_URL  # Windows PowerShell
cargo run
```

### Issue: "error: linker 'cc' not found"
**Linux Solution:**
```bash
sudo apt install build-essential
```

**Mac Solution:**
```bash
xcode-select --install
```

### Issue: High CPU usage
**Solution:** This is a simulation - high CPU is expected. To reduce:
1. Lower the tick rate in `sim_server/src/main.rs`
2. Reduce initial agent count in `sim_server/src/simulation.rs`
3. Enable the AI LOD system (when implemented)

## Running Tests

### All Tests
```bash
cargo test --workspace
```

### Specific Crate
```bash
cargo test -p world_sim_core
cargo test -p world_sim_agents
```

### With Output
```bash
cargo test -- --nocapture
```

### Test Scripts

**Windows:**
```powershell
.\test.ps1
```

**Linux/Mac:**
```bash
chmod +x test.sh
./test.sh
```

## Performance Tips

### 1. Always use Release Mode for Production
```bash
cargo build --release
```
Release builds are 10-100x faster than debug builds.

### 2. Enable CPU Optimizations
Add to `Cargo.toml` (already included):
```toml
[profile.release]
opt-level = 3
lto = "thin"
codegen-units = 1
```

### 3. Use a PostgreSQL Connection Pool
The default configuration uses 5 connections. Adjust in `crates/persistence/src/database.rs`:
```rust
PgPoolOptions::new()
    .max_connections(10)  // Increase if needed
    .connect(database_url)
```

## Next Steps

1. ✅ Server is running
2. Read [API.md](API.md) to learn the API endpoints
3. Read [DEVELOPMENT.md](DEVELOPMENT.md) to start coding
4. Read [ARCHITECTURE.md](ARCHITECTURE.md) to understand the design

## Quick Start Checklist

- [ ] Rust installed (`cargo --version`)
- [ ] PostgreSQL installed (optional)
- [ ] Project built (`cargo build --release`)
- [ ] Server starts successfully
- [ ] API responds to `/health`
- [ ] Review documentation

---

**Need help?** Check the documentation or open an issue on GitHub.

