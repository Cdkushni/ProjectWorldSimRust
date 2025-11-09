# Admin API Documentation

Base URL: `http://127.0.0.1:8080`

## Endpoints

### Health Check

```http
GET /health
```

Returns `200 OK` with body `"OK"` if server is running.

**Example:**
```bash
curl http://127.0.0.1:8080/health
```

---

### Get Event History

```http
GET /api/history?event_type={type}&limit={n}
```

Query the event history database.

**Query Parameters:**
- `event_type` (optional): Filter by event type (e.g., "PriceChange", "WarDeclared")
- `limit` (optional): Max number of events to return (default: 100)

**Response:**
```json
{
  "events": [
    {
      "id": "uuid",
      "timestamp": "2023-11-09T12:00:00Z",
      "event_type": "PriceChange",
      "source": "system",
      "payload": {
        "resource": "Wood",
        "old_price": 5.0,
        "new_price": 7.5,
        "total_supply": 100,
        "total_demand": 200
      }
    }
  ]
}
```

**Example:**
```bash
curl "http://127.0.0.1:8080/api/history?event_type=PriceChange&limit=10"
```

---

### Inject Custom Event (Dungeon Master)

```http
POST /api/dm/inject_event
Content-Type: application/json
```

Manually inject an event into the simulation.

**Request Body:**
```json
{
  "event_type": "string",
  "payload": {}
}
```

**Response:**
```json
{
  "success": true,
  "event_id": "uuid"
}
```

**Examples:**

Inject a Blight:
```bash
curl -X POST http://127.0.0.1:8080/api/dm/inject_event \
  -H "Content-Type: application/json" \
  -d '{
    "event_type": "BlightStarted",
    "payload": {
      "center": {"x": 0.0, "y": 0.0, "z": 0.0},
      "radius": 100.0,
      "affected_resource": "Wood"
    }
  }'
```

Inject a War:
```bash
curl -X POST http://127.0.0.1:8080/api/dm/inject_event \
  -H "Content-Type: application/json" \
  -d '{
    "event_type": "WarDeclared",
    "payload": {
      "aggressor": "faction-uuid",
      "defender": "faction-uuid",
      "reason": "Border dispute"
    }
  }'
```

---

### Add Memory to Agent

```http
POST /api/agent/:id/add_memory
Content-Type: application/json
```

Add a (possibly false) memory to an agent.

**Path Parameters:**
- `id`: Agent UUID

**Request Body:**
```json
{
  "fact": "string",
  "source": "string (optional)"
}
```

**Response:**
```json
{
  "success": true,
  "agent_id": "uuid",
  "memory": "string"
}
```

**Example:**
```bash
curl -X POST http://127.0.0.1:8080/api/agent/550e8400-e29b-41d4-a716-446655440000/add_memory \
  -H "Content-Type: application/json" \
  -d '{
    "fact": "The king is planning to betray us",
    "source": "rumor"
  }'
```

---

### Get Agent Information

```http
GET /api/agent/:id
```

Get information about a specific agent.

**Path Parameters:**
- `id`: Agent UUID

**Response:**
```json
{
  "agent_id": "uuid",
  "status": "placeholder"
}
```

**Example:**
```bash
curl http://127.0.0.1:8080/api/agent/550e8400-e29b-41d4-a716-446655440000
```

---

### Create World Snapshot

```http
GET /api/world/snapshot
```

Create a snapshot of the current world state.

**Response:**
```json
{
  "success": true,
  "message": "Snapshot created (placeholder)"
}
```

**Example:**
```bash
curl http://127.0.0.1:8080/api/world/snapshot
```

---

### List Snapshots

```http
GET /api/world/snapshots
```

List all saved world snapshots.

**Response:**
```json
{
  "snapshots": [
    ["uuid", "snapshot_name", "2023-11-09T12:00:00Z"]
  ]
}
```

**Example:**
```bash
curl http://127.0.0.1:8080/api/world/snapshots
```

---

### Get Metrics

```http
GET /api/metrics
```

Get simulation metrics and statistics.

**Response:**
```json
{
  "uptime": 0,
  "agent_count": 0,
  "events_processed": 0
}
```

**Example:**
```bash
curl http://127.0.0.1:8080/api/metrics
```

---

## Event Types Reference

### Economic Events

#### PriceChange
```json
{
  "resource": "Wood|Stone|Iron|Gold|Food|Water|Cloth|Tool|Weapon|Coin",
  "old_price": 0.0,
  "new_price": 0.0,
  "total_supply": 0,
  "total_demand": 0
}
```

#### TradeExecuted
```json
{
  "seller_id": "uuid",
  "buyer_id": "uuid",
  "resource": "Wood",
  "quantity": 10,
  "price": 5.0,
  "location": {"x": 0.0, "y": 0.0, "z": 0.0}
}
```

### Political Events

#### WarDeclared
```json
{
  "aggressor": "faction-uuid",
  "defender": "faction-uuid",
  "reason": "string"
}
```

#### PeaceTreaty
```json
{
  "faction_a": "uuid",
  "faction_b": "uuid",
  "terms": "string"
}
```

### Environmental Events

#### BlightStarted
```json
{
  "center": {"x": 0.0, "y": 0.0, "z": 0.0},
  "radius": 100.0,
  "affected_resource": "Wood"
}
```

#### DroughtStarted
```json
{
  "region": "string",
  "severity": 0.8,
  "expected_duration_days": 30
}
```

#### SeasonChange
```json
{
  "old_season": "Spring|Summer|Autumn|Winter",
  "new_season": "Spring|Summer|Autumn|Winter"
}
```

### Agent Events

#### AgentDied
```json
{
  "agent_id": "uuid",
  "cause": "string",
  "location": {"x": 0.0, "y": 0.0, "z": 0.0}
}
```

#### AgentBorn
```json
{
  "agent_id": "uuid",
  "parent_ids": ["uuid"],
  "location": {"x": 0.0, "y": 0.0, "z": 0.0}
}
```

### Dungeon Master Events

#### DungeonMasterEvent
```json
{
  "event_name": "string",
  "description": "string",
  "impact": "string"
}
```

---

## WebSocket Support (Planned)

Future versions will support WebSocket connections for real-time event streaming:

```javascript
const ws = new WebSocket('ws://127.0.0.1:8080/ws');

ws.onmessage = (event) => {
  const eventData = JSON.parse(event.data);
  console.log('Received event:', eventData);
};
```

---

## Error Responses

All endpoints may return error responses:

**400 Bad Request:**
```json
{
  "error": "Invalid request format"
}
```

**404 Not Found:**
```json
{
  "error": "Resource not found"
}
```

**500 Internal Server Error:**
```json
{
  "error": "Internal server error"
}
```

**503 Service Unavailable:**
```json
{
  "error": "Database not available"
}
```

---

## Rate Limiting

Currently no rate limiting is implemented. For production use, consider:
- Implementing token-based authentication
- Rate limiting per IP/token
- Request throttling for expensive operations

---

## CORS

CORS is enabled with permissive settings for development. For production:
1. Restrict `allowed_origins` to your visualizer domain
2. Enable authentication
3. Use HTTPS

---

## Integration Examples

### JavaScript (Three.js)

```javascript
class WorldSimClient {
  constructor(baseUrl = 'http://127.0.0.1:8080') {
    this.baseUrl = baseUrl;
  }

  async getEventHistory(eventType, limit = 100) {
    const params = new URLSearchParams({ limit });
    if (eventType) params.append('event_type', eventType);
    
    const response = await fetch(`${this.baseUrl}/api/history?${params}`);
    return await response.json();
  }

  async injectEvent(eventType, payload) {
    const response = await fetch(`${this.baseUrl}/api/dm/inject_event`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ event_type: eventType, payload })
    });
    return await response.json();
  }
}

// Usage
const client = new WorldSimClient();
const events = await client.getEventHistory('PriceChange', 10);
```

### Python

```python
import requests

class WorldSimClient:
    def __init__(self, base_url='http://127.0.0.1:8080'):
        self.base_url = base_url
    
    def get_event_history(self, event_type=None, limit=100):
        params = {'limit': limit}
        if event_type:
            params['event_type'] = event_type
        
        response = requests.get(f'{self.base_url}/api/history', params=params)
        return response.json()
    
    def inject_event(self, event_type, payload):
        response = requests.post(
            f'{self.base_url}/api/dm/inject_event',
            json={'event_type': event_type, 'payload': payload}
        )
        return response.json()

# Usage
client = WorldSimClient()
events = client.get_event_history('PriceChange', 10)
```

### Unreal Engine (C++)

```cpp
// Using Unreal's HTTP module
#include "Http.h"

void UWorldSimClient::GetEventHistory(const FString& EventType, int32 Limit)
{
    FHttpModule& HttpModule = FHttpModule::Get();
    TSharedRef<IHttpRequest> Request = HttpModule.CreateRequest();
    
    Request->SetURL(FString::Printf(TEXT("http://127.0.0.1:8080/api/history?limit=%d"), Limit));
    Request->SetVerb("GET");
    Request->OnProcessRequestComplete().BindUObject(this, &UWorldSimClient::OnResponseReceived);
    
    Request->ProcessRequest();
}

void UWorldSimClient::OnResponseReceived(FHttpRequestPtr Request, FHttpResponsePtr Response, bool bWasSuccessful)
{
    if (bWasSuccessful && Response.IsValid())
    {
        FString ResponseString = Response->GetContentAsString();
        // Parse JSON and update world state
    }
}
```

---

**Note:** This API is designed for development and admin control. For production game servers, add authentication, encryption, and rate limiting.

