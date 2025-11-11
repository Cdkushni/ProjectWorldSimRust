# 🔍 Debug Building Rendering

## ✅ **Changes Made:**

1. Added detailed logging to updateBuildings()
2. Added error handling with stack traces
3. Increased building opacity for visibility
4. Fresh build with all fixes

---

## 🎯 **TEST NOW:**

### **1. Start Fresh Server** (if not already running)
```powershell
.\target\release\sim_server.exe
```

### **2. Refresh Visualizer with New Logging**
```
Ctrl + Shift + R (hard refresh to load new JavaScript)
```

### **3. Open Browser Console**
```
F12 → Console tab
```

---

## 📊 **What to Look For in Browser Console:**

### **Every 0.2 seconds, you should see:**
```javascript
🏗️ updateBuildings called with X buildings
📊 Buildings in scene: X (received X from API)
```

**If you see:**

**A) "updateBuildings called with 0 buildings"**
- API isn't sending buildings
- Check http://127.0.0.1:8080/api/world/state in browser
- Look for "buildings" array

**B) "updateBuildings called with 15 buildings" but "Buildings in scene: 0"**
- Buildings received but not being added
- Look for red error: "❌ Error in updateBuildings:"
- Send me the error message

**C) "updateBuildings called with 15 buildings" and "Buildings in scene: 15"**
- ✅ Buildings ARE in scene!
- They might be:
  - Outside camera view (need to pan/zoom)
  - Too transparent (though I increased opacity)
  - Behind other objects

---

## 🎥 **If Buildings ARE in Scene (count matches):**

### **Try Finding Them:**

**Option 1: Zoom Way Out**
```
Scroll wheel backwards a lot
Get full world view
Look for yellow wireframe scaffolding
```

**Option 2: Fly to Known Building Location**

**In browser console, paste:**
```javascript
// Fly to Peasant_88's farming shed at (38.0, -84.9)
camera.position.set(38, 30, -84.9);
camera.lookAt(38, 0, -84.9);
controls.target.set(38, 0, -84.9);
controls.update();
```

**Look down** - you should see a building with:
- Dark gray foundation
- Brown/tan walls
- Yellow wireframe scaffolding
- Green progress bar
- Label: "Peasant_88's Shed"

**Option 3: List All Buildings**

**In browser console:**
```javascript
console.log('All buildings:', Array.from(buildings.values()).map(b => ({
    name: b.data.name,
    x: b.data.x,
    z: b.data.z,
    progress: b.data.construction_progress
})));
```

This will show you every building and its position.

---

## 🐛 **Most Likely Issues:**

### **Issue 1: Buildings Not Being Sent**
**Check:** `http://127.0.0.1:8080/api/world/state`
**Look for:** `"buildings": [...]` array
**Should have:** 20-50 buildings based on your server logs

### **Issue 2: JavaScript Error**
**Check:** Browser console for red error messages
**If error:** Send me the exact error text

### **Issue 3: Buildings Far From Center**
Position (-84.9) is very far south. You might be looking at center while buildings are scattered at edges.

**Solution:** Zoom out significantly or fly to building coordinates

---

## 🎯 **ACTION ITEMS:**

**Send me these 3 things:**

1. **Browser console output:**
   ```
   What does it show for:
   "🏗️ updateBuildings called with X buildings"
   "📊 Buildings in scene: X"
   ```

2. **Any red errors:**
   ```
   "❌ Error in updateBuildings: ..."
   ```

3. **API building count:**
   ```
   Open: http://127.0.0.1:8080/api/world/state
   Count buildings array length
   ```

This will tell me exactly why buildings.size = 0!

---

**Refresh visualizer (Ctrl + Shift + R) and check browser console!** 🔍

