# 🌍 Three.js Visualizer Guide

## Quick Start

### 1. Start the Simulation Server
```powershell
.\target\release\sim_server.exe
```

Wait for the message:
```
🌐 Admin API listening on http://127.0.0.1:8080
🚀 Simulation running
```

### 2. Open the Visualizer
Simply double-click `visualizer.html` or open it in your browser:
```powershell
start visualizer.html
```

Or use any modern browser:
- Chrome
- Edge
- Firefox
- Safari

## Features

### 📊 Real-Time Stats Panel (Top Left)
- **Agents**: Current population count
- **Uptime**: Simulation runtime in seconds
- **Events**: Total events processed
- **FPS**: Visualizer frame rate

### 🎮 Interactive 3D Controls
- **Left Mouse + Drag**: Rotate camera around the world
- **Right Mouse + Drag**: Pan camera
- **Mouse Wheel**: Zoom in/out
- **Reset Camera Button**: Return to default view
- **Pause/Resume Button**: Freeze visualization

### 🎨 Visual Elements
- **Colored Capsules**: Living agents (each has unique color)
- **Green Terrain**: Ground plane (100x100 world)
- **Grid Lines**: For spatial reference
- **Trees**: Environmental decoration
- **Shadows**: Realistic lighting

### 🔄 Connection Status (Top Right)
- **● Connected** (Green): Receiving data from server
- **● Disconnected** (Red): Cannot reach server

## What You're Seeing

Each **colored capsule** is an agent in your simulation:
- Position updates every 0.5 seconds
- Smooth interpolation between positions
- Subtle bobbing animation (breathing effect)
- Gentle rotation for life-like movement

The agents are spread across the terrain based on their actual positions in the simulation.

## Troubleshooting

### "● Disconnected" Status

**Problem**: Can't connect to the simulation server

**Solutions**:
1. Make sure the server is running (`sim_server.exe`)
2. Check the console for errors
3. Verify the API is accessible: http://127.0.0.1:8080/health
4. Check for firewall/port blocking

### No Agents Visible

**Problem**: Stats show agents but none appear in 3D view

**Solutions**:
1. Wait 1-2 seconds for initial data load
2. Try zooming out (mouse wheel)
3. Click "Reset Camera"
4. Check browser console (F12) for errors

### Low FPS

**Problem**: Visualization is choppy

**Solutions**:
1. Close other browser tabs
2. Reduce agent count in the simulation
3. Try a different browser (Chrome/Edge recommended)
4. Update your graphics drivers

### CORS Errors in Console

**Problem**: Browser blocks API requests

**Solution**: The server has CORS enabled. If you still see errors:
1. Make sure you're opening the HTML file (not running from `file://`)
2. Or use a simple HTTP server:
```powershell
python -m http.server 3000
# Then visit http://localhost:3000/visualizer.html
```

## Customization

### Change Update Rate
Edit `visualizer.html` line ~250:
```javascript
setInterval(fetchWorldState, 500);  // 500ms = 0.5 seconds
```

Lower = more responsive, higher = less CPU usage

### Change Camera Position
Edit `visualizer.html` line ~225:
```javascript
camera.position.set(70, 60, 70);  // X, Y, Z
```

### Change Colors
Edit the terrain color (line ~240):
```javascript
color: 0x1a4d2e,  // Hex color code
```

Or agent colors (line ~287):
```javascript
const hue = Math.random();  // Random, or use fixed value (0-1)
```

## Advanced Features (Coming Soon)

**Planned Enhancements:**
- [ ] Agent name labels on hover
- [ ] Trail rendering (show agent paths)
- [ ] Health bars above agents
- [ ] Minimap in corner
- [ ] Event notifications (wars, deaths, etc.)
- [ ] Click agent to see details
- [ ] Day/night cycle
- [ ] Weather visualization

## Performance

**Expected Performance:**
- 60 FPS with 100 agents
- 30-60 FPS with 1000 agents
- Scales well on modern GPUs

**Browser Performance:**
- Chrome/Edge: Best (V8 engine)
- Firefox: Good
- Safari: Good

## How It Works

```
┌─────────────────┐
│  Rust Server    │  ← Simulation runs here
│  (Port 8080)    │
└────────┬────────┘
         │ HTTP/JSON
         │ (Every 0.5s)
┌────────▼────────┐
│  visualizer.html│  ← Three.js renders here
│  (Your Browser) │
└─────────────────┘
```

The visualizer:
1. **Polls** `/api/world/state` every 500ms for agent positions
2. **Polls** `/api/metrics` every 1000ms for stats
3. **Interpolates** movement for smooth animation
4. **Renders** at 60 FPS in your browser

## Tips

🎥 **Camera**: Use right-click drag to pan, get different angles

🎨 **Agent Colors**: Each agent gets a random color on spawn

⚡ **Performance**: If laggy, close other browser tabs

🔍 **Zoom**: Get close to see individual agents moving

📊 **Stats**: Watch the stats panel to see population changes

---

**Enjoy watching your living, breathing world!** 🚀

*For issues or questions, check the server console logs first.*

