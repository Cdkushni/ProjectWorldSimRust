# 🧪 Phase 1: Economic System - Testing Guide

## ✅ **What to Test:**

Phase 1 implemented a complete trading economy. Here's how to verify it's working correctly before moving to Phase 2.

---

## 📋 **Testing Checklist:**

### **Test 1: Resource Harvesting → Inventory** 🌲

**What to Look For:**
- Workers (Woodcutters, Miners, Farmers) moving to resource nodes
- State changes to "Working" when near resources
- **Console Log:** No errors about harvesting

**How to Verify:**
1. Watch agents with job badges (👨 Peasants doing work)
2. Hover over a peasant agent
3. Note their position and state
4. If "Working" near a tree/rock/farm → ✅ Harvesting working

**Expected:** Workers automatically move to resources and harvest them into inventory

---

### **Test 2: Market Trading Behavior** 💰

**What to Look For:**
- Merchants (💰) and Burghers (🏪) traveling to markets
- State changes to "Trading" when at markets
- **Console Log:** `💰 Trade executed: [quantity] [resource] for [price]`

**How to Verify:**
1. Find merchants/burghers (look for 💰 and 🏪 icons)
2. Watch them move toward market buildings
3. Hover over them when at markets
4. State should be "Trading"
5. **Check server console** for trade logs

**Expected Logs (after a few minutes):**
```
💰 Trade executed: 10 Wood for 52.50 (5.25/unit)
💰 Trade executed: 5 Food for 75.00 (15.00/unit)
💰 Trade executed: 15 Stone for 48.00 (3.20/unit)
```

**If NO trades after 2-3 minutes:**
- Agents might not have excess inventory yet
- Wait longer (they need to harvest first)
- Check if merchants are actually reaching markets

---

### **Test 3: Wage System** 💵

**What to Look For:**
- **Console Log:** `💵 Wages paid to [count] workers`
- Occurs every 60 seconds

**How to Verify:**
1. Start server
2. Watch server console
3. Wait 60 seconds
4. Should see: `💵 Wages paid to 100 workers`
5. Repeat every minute

**Expected Timeline:**
```
T=0s:   Server starts
T=60s:  💵 Wages paid to 100 workers
T=120s: 💵 Wages paid to 100 workers
T=180s: 💵 Wages paid to 100 workers
```

**If NO wage logs:**
- Check if 60 seconds have passed
- Wage timer might not be incrementing
- Report back to me

---

### **Test 4: Market Order Matching** 📊

**What to Look For:**
- Buy/sell orders being placed
- Orders being matched
- Trades executing

**How to Verify:**
1. Open Economy Dashboard (📊 button)
2. Check "Market Activity" panel
3. Look for "Total Transactions" increasing
4. Should be > 0 after a few minutes

**Expected Progression:**
```
T=0:   0 transactions
T=2m:  0 transactions (agents still harvesting)
T=5m:  5-10 transactions (first trades!)
T=10m: 20-50 transactions (active economy)
```

**If Stuck at 0:**
- Agents might not be reaching markets
- No excess inventory to sell yet
- Check if merchants are at markets

---

### **Test 5: Currency & Inflation** 📈

**What to Look For:**
- Money supply increasing
- Inflation rate rising
- Transaction count increasing

**How to Verify:**
1. Open Economy Dashboard
2. Check "Currency System" panel
3. Watch metrics change:
   - **Total Supply:** Should increase (wages minting money)
   - **Transactions:** Should increase (trades happening)
   - **Inflation Rate:** Should slowly rise (money creation)

**Expected Values:**
```
T=0:   Supply: 20,000 | Inflation: 0.00% | Transactions: 0
T=1m:  Supply: 20,500 | Inflation: 0.05% | Transactions: 0
T=5m:  Supply: 22,500 | Inflation: 0.15% | Transactions: 5-10
T=10m: Supply: 25,000 | Inflation: 0.30% | Transactions: 30-60
```

---

### **Test 6: Visual Indicators** 👀

**What to Check:**
- [ ] Agents visible (100 total)
- [ ] Resources visible (trees, rocks, farms)
- [ ] Markets visible (3 buildings)
- [ ] Buildings visible (2 complete + 1 under construction)
- [ ] Agents moving between resources and markets
- [ ] "Trading" state color visible (cyan/teal)
- [ ] Speech bubbles appearing
- [ ] No console errors in browser (F12)

---

## 🎮 **Recommended Test Sequence:**

### **5-Minute Quick Test:**

**Minute 0-1: Initial State**
```
✅ See 100 agents
✅ See resources (trees, rocks, farms)
✅ See 3 markets
✅ See 2-3 buildings
✅ Agents start moving
```

**Minute 1-2: Harvesting Phase**
```
✅ Workers moving to resources
✅ States changing to "Working"
✅ Agents harvesting (inventory filling silently)
✅ NO trade logs yet (normal)
```

**Minute 2-3: First Wages**
```
✅ Server console: "💵 Wages paid to 100 workers"
✅ Agents now have more money
✅ Some may have excess inventory
```

**Minute 3-5: Trading Begins**
```
✅ Merchants/Burghers moving to markets
✅ States changing to "Trading"
✅ Server console: "💰 Trade executed..." logs
✅ Economy Dashboard: Transactions > 0
```

**Minute 5+: Active Economy**
```
✅ Regular trade logs
✅ Wage payments every minute
✅ Transaction count climbing
✅ Inflation slowly rising
```

---

## 📊 **Economy Dashboard Checks:**

### **Open Dashboard:**
```
Click "📊 Economy Dashboard" button
```

### **What to Verify:**

**Currency System:**
- [ ] Total Supply increasing (from 20,000)
- [ ] Inflation Rate rising (from 0.00%)
- [ ] Transaction Count increasing
- [ ] Velocity > 0 (once trades start)

**Market Activity:**
- [ ] Total Markets: 3 ✅
- [ ] Total Transactions: Increasing
- [ ] Active Traders: > 0 (merchants at markets)
- [ ] Avg Reputation: ~50 (default)

**Resource Supply:**
- [ ] Wood, Stone, Food, Iron showing quantities
- [ ] Quantities decreasing as harvested
- [ ] Some resources being consumed

**Per Capita Resources:**
- [ ] Food/Person: Should be high initially (green)
- [ ] Materials/Person: Should be high initially (green)
- [ ] Will decrease over time

---

## 🎯 **Success Criteria:**

**Phase 1 is working if:**

1. ✅ **No Console Errors** - Browser console (F12) is clean
2. ✅ **Agents Visible** - 100 agents in scene
3. ✅ **Wage Logs** - "💵 Wages paid" every 60 seconds
4. ✅ **Trade Logs** - "💰 Trade executed" appearing (after ~3-5 min)
5. ✅ **Transaction Count** - Economy Dashboard shows > 0 transactions
6. ✅ **Money Supply** - Increasing from 20,000
7. ✅ **Merchants Active** - Some in "Trading" state at markets

**Minimum Success (if short on time):**
- ✅ Wage logs appearing
- ✅ At least 1 trade executed
- ✅ No crashes or errors

---

## 🐛 **Troubleshooting:**

### **Problem: No Trade Logs After 5 Minutes**

**Possible Causes:**
1. Agents haven't harvested enough yet (need excess to sell)
2. Merchants not reaching markets (pathing issue)
3. Orders not matching (pricing mismatch)

**Debug:**
- Check if merchants are in "Trading" state
- Verify agent inventories have resources
- Check if buy/sell order logic is executing

### **Problem: Wage Logs Stop**

**Cause:** Timer might have reset improperly

**Debug:**
- Wait another 60 seconds
- Check if agent wallets are increasing

### **Problem: Agents Not Working**

**Cause:** State machine not transitioning to Working

**Debug:**
- Hover over agents
- Check their states
- Look for majority in Idle/Moving/Working

---

## 📈 **Expected Economy Timeline:**

**T=0-60s: Bootstrap Phase**
- Agents spawned with initial wallets
- Begin harvesting resources
- No trading yet (building inventory)
- First wage payment at T=60s

**T=60-180s: Early Trading**
- Some agents have excess resources
- First trades occur
- Markets become active
- Transaction count: 1-10

**T=180-600s: Active Economy**
- Regular trades every few seconds
- Wage payments every minute
- Money circulating
- Transaction count: 50-200

**T=600s+: Mature Economy**
- Established trade patterns
- Price fluctuations
- Wealth inequality emerging
- Active market dynamics

---

## 🎯 **Quick Test (30 seconds):**

If you just want to verify it's working:

1. **Start server**
2. **Open visualizer**
3. **Wait 60 seconds** (one wage cycle)
4. **Check server console** for: `💵 Wages paid to 100 workers`
5. **If you see that → Phase 1 is working!** ✅

The trading will happen naturally over the next few minutes as agents accumulate inventory.

---

## ✅ **When You're Ready:**

Once you verify Phase 1 is working (even just wage logs), let me know and I'll proceed with:

**Phase 2: Resource-Based Building System (4 tasks)**
- Buildings require materials
- Builders retrieve resources
- Construction consumes resources
- Carrying capacity limits

This will make construction realistic and connected to the economy!

---

**Test for a few minutes and report back what you observe!** 🧪

