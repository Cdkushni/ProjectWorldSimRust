# 🔄 Complete Restart Procedure

## ✅ **Build Status: SUCCESS**

All locks have been explicitly dropped. The code is correct.

---

## 🎯 **COMPLETE RESTART:**

### **Step 1: Stop ALL Processes**
```powershell
# Press Ctrl + C in server terminal
# If that doesn't work, close the terminal window completely
```

### **Step 2: Kill Any Zombie Processes**
```powershell
Stop-Process -Name "sim_server" -Force -ErrorAction SilentlyContinue
Stop-Process -Name "cargo" -Force -ErrorAction SilentlyContinue
```

### **Step 3: Start Fresh Server**
```powershell
Set-Location E:\Repo\ProjectWorldSimRust
.\target\release\sim_server.exe
```

### **Step 4: Verify Startup Logs**
You should see:
```
🌍 Starting World Simulation Server
Generating initial world...
Initial population: 100 agents WITHOUT factions
Created 3 public markets
Created 3 public buildings
✅ Simulation initialized
🌐 Admin API listening on http://127.0.0.1:8080
```

### **Step 5: Test API Immediately**
In browser, open:
```
http://127.0.0.1:8080/api/metrics
```

**Should show:**
```json
{"agent_count":100,"uptime_seconds":...}
```

**If it HANGS or shows error:**
- API is deadlocked
- Send me screenshot

### **Step 6: Refresh Visualizer**
```
Open visualizer.html
Ctrl + Shift + R
```

---

## 🔍 **If Still No Agents/Buildings:**

### **Check 1: Browser Console**
```
Press F12
Console tab
Look for errors
```

### **Check 2: Network Tab**
```
F12 → Network
Refresh page
Look for /api/world/state
- Does it respond?
- What status code?
- How long does it take?
```

### **Check 3: API Direct Test**
```
http://127.0.0.1:8080/api/world/state
```

**Should return immediately** with JSON containing agents, resources, buildings

**If it HANGS:**
- sync_world_state_to_api() is still blocked
- There's a lock I haven't found

---

## 💡 **Key Diagnostic:**

**Does `/api/metrics` respond quickly?**
- **YES** → API working, issue is in `/api/world/state` sync
- **NO** → Entire API deadlocked

**Does server show new logs every minute?**
- **YES** → Simulation running, API blocked
- **NO** → Server hung completely

---

**Try the complete restart and let me know if the API responds!** 🔧

