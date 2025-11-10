# 🔧 Troubleshooting: Zero Agents Issue

## ✅ **Build Status: SUCCESSFUL**

The code compiles with only minor warnings (no errors).

---

## 🔍 **Issue: Agent Count = 0**

This happens when:
1. ❌ Server isn't running
2. ❌ Running old server binary (before economic changes)
3. ❌ API connection failed
4. ❌ Agents failed to spawn

---

## 🎯 **Solution Steps:**

### **Step 1: Stop Any Running Server**
```powershell
# In the terminal running sim_server, press:
Ctrl + C
```

### **Step 2: Start New Server**
```powershell
Set-Location E:\Repo\ProjectWorldSimRust
.\target\release\sim_server.exe
```

### **Step 3: Watch Console Output**

**You SHOULD see:**
```
Generating initial world...
Generating resource nodes...
Spawning initial population without factions...
Initial population: 100 agents WITHOUT factions
Created 3 public markets
Created 3 public buildings (including 1 under construction)
Admin API server started on 0.0.0.0:8080
```

**If you DON'T see this, there's a runtime error!**

### **Step 4: Refresh Visualizer**
```
In browser:
Ctrl + Shift + R (hard refresh)
```

### **Step 5: Check Connection**
- Top-right should show "● Connected" (green)
- Agent count should show "100"

---

## 🐛 **If Still Zero Agents:**

### **Check 1: Server Running?**
```powershell
# In PowerShell:
Get-Process sim_server
```

**Should show:** sim_server process running

### **Check 2: API Responding?**
```
Open in browser:
http://127.0.0.1:8080/api/metrics
```

**Should show:** `{"agent_count":100, ...}`

### **Check 3: World State Endpoint?**
```
Open in browser:
http://127.0.0.1:8080/api/world/state
```

**Should show:** JSON with agents array containing 100 agents

### **Check 4: Browser Console**
```
Press F12 in browser
Check Console tab for errors
```

**Look for:**
- Network errors (CORS, 404, 503)
- JavaScript errors
- API connection failures

---

## 💡 **Most Likely Cause:**

**You need to restart the server!**

The economic system adds new fields to agents. The old server binary doesn't have these fields, which could cause serialization issues.

**Solution:**
```powershell
# 1. Stop old server (Ctrl + C)
# 2. The build already completed
# 3. Start new server:
.\target\release\sim_server.exe
```

---

## 🎮 **Complete Restart Procedure:**

```powershell
# Step 1: Navigate to project
Set-Location E:\Repo\ProjectWorldSimRust

# Step 2: Build (already done, but doesn't hurt)
cargo build --release

# Step 3: Start server
.\target\release\sim_server.exe
```

**Watch for console output showing 100 agents spawned!**

Then in browser:
```
Ctrl + Shift + R
```

---

## 📊 **Expected Visualizer Display:**

Once working, you should see:
- **Agent Count:** 100
- **Agents visible:** 100 agents in circular pattern
- **States:** Idle, Moving, Working, Trading, etc.
- **Speech bubbles** appearing
- **Markets visible** (3 markets as buildings)
- **Buildings visible** (2 complete + 1 under construction)

---

## 🔍 **Debug Checklist:**

- [ ] Server stopped (Ctrl + C)
- [ ] New build completed (no errors)
- [ ] New server started
- [ ] Console shows "100 agents"
- [ ] Browser hard refreshed (Ctrl + Shift + R)
- [ ] Connection status = "Connected"
- [ ] Agent count = 100
- [ ] Agents visible in 3D view

---

**Try restarting the server and let me know if agents appear!** 🚀

