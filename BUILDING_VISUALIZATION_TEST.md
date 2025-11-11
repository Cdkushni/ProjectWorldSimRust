# 🏗️ Building Visualization Testing Guide

## ✅ **What's Working:**
- Server running perfectly ✅
- Agents visible ✅  
- Resources visible ✅
- Buildings being CREATED ✅ (console logs confirm!)

## ❌ **What's NOT Working:**
- New buildings (Farms, Houses, Sheds) not appearing in 3D scene

---

## 🎯 **Complete Test Procedure:**

### **Step 1: Stop Server**
```powershell
Ctrl + C in server terminal
```

### **Step 2: Start Server**
```powershell
Set-Location E:\Repo\ProjectWorldSimRust
.\target\release\sim_server.exe
```

### **Step 3: HARD Refresh Visualizer**
```
In browser:
Ctrl + Shift + R (hard refresh to reload updateBuildings code)
```

### **Step 4: Open Browser Console**
```
Press F12
Click "Console" tab
```

### **Step 5: Look for Building Logs**

**You should see:**
```javascript
🏗️ Building created: [name] (X% complete)
```

**If you see these logs:**
- Buildings ARE being received by visualizer
- Issue is in rendering (transparent buildings, wrong size, etc.)

**If you DON'T see these logs:**
- Buildings not being sent to visualizer
- Check API response

---

## 🔍 **Diagnostic Tests:**

### **Test A: Check API Response**
```
Open in browser:
http://127.0.0.1:8080/api/world/state
```

**Look for `"buildings"` array:**
```json
{
  "agents": [...],
  "resources": [...],
  "markets": [...],
  "buildings": [
    {
      "id": "...",
      "building_type": "Farm",
      "name": "Farm (Noble Order)",
      "x": 54.2,
      "y": 1.0,
      "z": 2.4,
      "construction_progress": 0.0,
      "health": 100.0,
      "owner": "Public"
    },
    ...
  ]
}
```

**Count the buildings array length:**
- Should be 3+ initial buildings
- Plus all the Farms/Houses/Sheds from logs (~20-30 buildings)

**If building count looks right but not visible:**
- Rendering issue in visualizer
- Buildings might be transparent or outside camera view

---

### **Test B: Check Building Positions**

**From your console logs:**
- Line 878: Farm at (54.2, 2.4) - Should be visible
- Line 860: House at (-18.3, -87.2) - Close to edge but visible
- Line 865: Shed at (-41.1, 2.2) - Should be visible

**Try flying camera to these coordinates:**
- Click "🎥 Reset Camera" button
- Or use browser console:
```javascript
camera.position.set(54, 30, 2);
camera.lookAt(54, 0, 2);
controls.update();
```

**Do you see a building there?**

---

### **Test C: Check Building Count in Scene**

**In browser console:**
```javascript
console.log('Buildings in scene:', buildings.size);
console.log('Buildings data:', Array.from(buildings.keys()));
```

**Should show:**
- Map of building IDs
- Should match API building count

---

## 💡 **Possible Issues:**

### **Issue 1: Transparent Buildings**
New buildings start at 0% progress → might be invisible (opacity: 0.3)

**Fix in visualizer.html, line 1877:**
Change minimum opacity from 0.3 to 0.6 so buildings are more visible

### **Issue 2: Buildings Outside Camera View**
Some positions like (-97, -37) or (111, 22) are far from center

**Solution:**
Pan camera around or zoom out to see full world

### **Issue 3: Building Type Names Mismatch**
API sends `"PeasantHouse"` but visualizer expects specific format

**Check:**
Browser console for errors like "Undefined property"

### **Issue 4: updateBuildings() Not Being Called**
Visualizer might not be calling updateBuildings

**Check:**
Line 1192 in visualizer should call `updateBuildings(data.buildings || [])`

---

## 🎯 **What I Need From You:**

**Please check and send me:**

1. **Browser Console Output** (F12):
   - Any errors?
   - Any "🏗️ Building created" logs?
   - What does `buildings.size` show?

2. **API Response** (`http://127.0.0.1:8080/api/world/state`):
   - How many buildings in the array?
   - Do you see Farm, PeasantHouse, FarmingShed types?

3. **Camera Position:**
   - Are you zoomed in too close to see scattered buildings?
   - Try zooming out or panning around

---

**Once I know which of these is the issue, I can fix it immediately!** 🔍

