# 🎯 Final Test Instructions - Building Visibility

## ✅ **Changes Made:**

1. **Re-enabled Phase 3** - All hierarchical AI active
2. **Increased building opacity** - 0.3 → 0.7 (70% visible even at 0% progress)
3. **Added debug logging** - Will show building positions and counts

---

## 🔧 **Complete Restart:**

### **1. Stop Server**
```powershell
Ctrl + C in server terminal
```

### **2. Start Server**
```powershell
.\target\release\sim_server.exe
```

### **3. HARD Refresh Visualizer**
```
Ctrl + Shift + R in browser
```

### **4. Open Browser Console**
```
F12 → Console tab
```

---

## 🔍 **What to Check:**

### **In Browser Console, Look For:**

**Building Creation Logs:**
```javascript
🏗️ Building created: Farm (Noble Order) (0% complete) at (54.2, 2.4)
🏗️ Building created: Peasant_71's House (0% complete) at (-18.3, -87.2)
📊 Buildings in scene: 15 (received 15 from API)
```

**Count Updates:**
Every 0.2 seconds you should see:
```javascript
📊 Buildings in scene: X (received Y from API)
```

---

## 📊 **Expected Results:**

### **If Console Shows:**

**A) "Buildings in scene: 15 (received 15 from API)"**
- ✅ Buildings ARE in the scene
- Issue: They might be:
  - Too transparent (fixed with opacity increase)
  - Outside camera view (need to pan around)
  - Too small (need to zoom in)

**B) "Buildings in scene: 3 (received 30 from API)"**
- ❌ Buildings received but not being added to scene
- JavaScript error in updateBuildings function
- Send me any red errors from console

**C) No building logs at all**
- ❌ updateBuildings not being called
- Check if `data.buildings` exists in fetchWorldState

---

## 🎥 **Finding Buildings:**

If buildings are in scene but you don't see them, try:

### **Option 1: Zoom Out**
```
Scroll wheel out
Get bird's eye view
```

### **Option 2: Pan to Building Location**
**Browser console:**
```javascript
// From your server logs, Farm at (54.2, 2.4):
camera.position.set(54, 30, 30);
camera.lookAt(54, 0, 2);
controls.update();
```

### **Option 3: Look for Scaffolding**
- New buildings have **yellow wireframe** scaffolding
- Might be easier to spot than walls
- Look for golden grid patterns

---

## 🎯 **Quick Diagnostic:**

**After restart, in browser console type:**
```javascript
buildings.size
```

**Expected:**
- Initial: 3 buildings
- After 5 min: 10-20 buildings
- After 30 min: 50+ buildings

**If buildings.size is correct but you don't see them:**
- They ARE there, just hard to spot
- Try zooming way out
- Look for scaffolding

---

**Restart server, hard refresh visualizer, and check browser console!** 🔍

