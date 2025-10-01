# Feature #2: Real-Time Performance Dashboard (WebSocket)

## Overview
Transform AI Coach from post-workout analysis to live coaching by implementing WebSocket endpoints for real-time metrics streaming, live workout tracking with alerts, and coach-athlete messaging.

## Business Value
- **Live Coaching**: Real-time feedback during workouts
- **Engagement**: Increased user engagement through live interaction
- **Differentiation**: Competitive advantage over batch-processing competitors
- **Retention**: Live features increase session duration and stickiness

## Technical Architecture

### Components
1. **WebSocket Server** (Axum WebSocket support)
2. **Redis Pub/Sub** (Message broadcasting)
3. **Real-time Metrics Processing** (Stream processing)
4. **Alert Engine** (Threshold-based notifications)
5. **Live Messaging** (Coach-athlete communication)

### Technology Stack
- `axum` WebSocket upgrade handlers
- `redis` pub/sub for message broadcasting
- `tokio` streams for async processing
- `tower-http` for WebSocket middleware
- `serde_json` for message serialization

## Implementation Tasks

### Phase 1: Foundation (Week 1-2)
**Task 1.1: WebSocket Infrastructure**
- Add WebSocket endpoint `/api/v1/live/ws`
- Implement connection manager to track active connections
- Create connection authentication using JWT tokens
- Add heartbeat/ping-pong mechanism for connection health
- Implement graceful connection cleanup on disconnect

**Task 1.2: Redis Pub/Sub Setup**
- Configure Redis connection pool for pub/sub
- Create channel structure: `live:user:{user_id}`, `live:session:{session_id}`
- Implement message publisher service
- Implement message subscriber service
- Add message routing logic

**Task 1.3: Message Protocol Design**
- Define WebSocket message types (JSON schema):
  - `metrics_update`: Real-time training metrics
  - `alert`: Zone/pace/HR alerts
  - `message`: Coach-athlete chat
  - `session_event`: Start/stop/pause events
- Create message serialization/deserialization
- Implement message validation

### Phase 2: Live Metrics Streaming (Week 3-4)
**Task 2.1: Live Session API**
- Create `POST /api/v1/live/sessions/start` endpoint
- Create `POST /api/v1/live/sessions/update` endpoint (metrics ingestion)
- Create `POST /api/v1/live/sessions/stop` endpoint
- Store live session data in `live_training_sessions` table
- Implement session state management (active/paused/completed)

**Task 2.2: Metrics Broadcasting**
- Create `LiveMetricsService` for processing incoming metrics
- Implement real-time calculations (current pace, avg HR, power zones)
- Broadcast metrics to WebSocket subscribers
- Add metric aggregation (rolling averages, lap summaries)
- Implement data buffering for disconnected clients

**Task 2.3: Database Schema**
- Create `live_training_sessions` table migration
- Create `live_metrics` table for time-series data
- Create indexes for efficient querying
- Add foreign keys to existing `training_sessions` table

### Phase 3: Alert System (Week 5)
**Task 3.1: Alert Engine**
- Create `LiveAlertService` for threshold monitoring
- Implement alert rules:
  - Heart rate zone alerts (too high/low)
  - Pace zone alerts (target pace deviation)
  - Power zone alerts (training zones)
  - Fatigue alerts (performance decline)
- Add user-configurable alert thresholds
- Implement alert cooldown to prevent spam

**Task 3.2: Alert Delivery**
- Broadcast alerts via WebSocket
- Store alert history in database
- Add alert acknowledgment mechanism
- Implement sound/vibration preferences

### Phase 4: Live Messaging (Week 6)
**Task 4.1: Coach-Athlete Messaging**
- Create `POST /api/v1/live/messages` endpoint
- Implement real-time message delivery via WebSocket
- Add message persistence in `live_messages` table
- Implement typing indicators
- Add read receipts

**Task 4.2: Presence System**
- Track online/offline status
- Broadcast presence updates
- Add "coach is watching" indicator
- Implement connection status UI feedback

### Phase 5: Client Integration & Testing (Week 7-8)
**Task 5.1: WebSocket Client Library**
- Create example JavaScript client for testing
- Document WebSocket connection flow
- Create reconnection logic with exponential backoff
- Add client-side message queue for offline scenarios

**Task 5.2: Testing**
- Unit tests for WebSocket handlers
- Integration tests for live session flow
- Load testing with multiple concurrent connections
- Test connection resilience (reconnection, network issues)
- Test Redis pub/sub reliability

**Task 5.3: Documentation**
- API documentation for WebSocket endpoints
- Message protocol documentation
- Integration guide for frontend developers
- Architecture diagrams

### Phase 6: Monitoring & Optimization (Week 9)
**Task 6.1: Observability**
- Add metrics for active WebSocket connections
- Track message latency (pub → delivery)
- Monitor Redis pub/sub performance
- Add tracing for WebSocket lifecycle

**Task 6.2: Performance Optimization**
- Implement message batching for high-frequency updates
- Add connection throttling to prevent abuse
- Optimize Redis pub/sub channel strategy
- Implement message compression for large payloads

## Database Schema

```sql
-- Live training sessions
CREATE TABLE live_training_sessions (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id),
    session_id UUID REFERENCES training_sessions(id),
    status VARCHAR(20) NOT NULL, -- active, paused, completed
    started_at TIMESTAMPTZ NOT NULL,
    last_update_at TIMESTAMPTZ NOT NULL,
    metadata JSONB,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Time-series metrics
CREATE TABLE live_metrics (
    id BIGSERIAL PRIMARY KEY,
    live_session_id UUID NOT NULL REFERENCES live_training_sessions(id),
    timestamp TIMESTAMPTZ NOT NULL,
    metric_type VARCHAR(50) NOT NULL, -- heart_rate, pace, power, cadence
    value DOUBLE PRECISION NOT NULL,
    metadata JSONB,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Live messages
CREATE TABLE live_messages (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    live_session_id UUID NOT NULL REFERENCES live_training_sessions(id),
    sender_id UUID NOT NULL REFERENCES users(id),
    recipient_id UUID NOT NULL REFERENCES users(id),
    message TEXT NOT NULL,
    read_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Indexes
CREATE INDEX idx_live_sessions_user ON live_training_sessions(user_id, status);
CREATE INDEX idx_live_metrics_session_time ON live_metrics(live_session_id, timestamp);
CREATE INDEX idx_live_messages_session ON live_messages(live_session_id, created_at);
```

## API Endpoints

### WebSocket
- `GET /api/v1/live/ws` - WebSocket upgrade endpoint (requires JWT)

### REST API
- `POST /api/v1/live/sessions/start` - Start live training session
- `POST /api/v1/live/sessions/{id}/update` - Update metrics
- `POST /api/v1/live/sessions/{id}/stop` - Stop session
- `GET /api/v1/live/sessions/{id}` - Get session details
- `POST /api/v1/live/messages` - Send message
- `GET /api/v1/live/messages/{session_id}` - Get message history

## Message Protocol Examples

```json
// Client → Server: Metrics Update
{
  "type": "metrics_update",
  "session_id": "uuid",
  "timestamp": "2025-09-30T12:00:00Z",
  "metrics": {
    "heart_rate": 150,
    "pace": 5.5,
    "distance": 3.2,
    "cadence": 180
  }
}

// Server → Client: Alert
{
  "type": "alert",
  "session_id": "uuid",
  "timestamp": "2025-09-30T12:00:00Z",
  "alert": {
    "severity": "warning",
    "message": "Heart rate above target zone",
    "metric": "heart_rate",
    "current": 175,
    "threshold": 165
  }
}

// Server → Client: Live Message
{
  "type": "message",
  "session_id": "uuid",
  "timestamp": "2025-09-30T12:00:00Z",
  "message": {
    "from": "coach_id",
    "text": "Great pace! Keep it up!",
    "id": "message_uuid"
  }
}
```

## Testing Strategy
1. **Unit Tests**: WebSocket handler logic, message parsing
2. **Integration Tests**: End-to-end WebSocket flow
3. **Load Tests**: 100+ concurrent connections, message throughput
4. **Stress Tests**: Connection churn, reconnection storms
5. **Manual Tests**: Real-world usage with mobile app

## Success Metrics
- WebSocket connection success rate >99%
- Message latency <100ms (p95)
- Support 1000+ concurrent connections
- Alert accuracy >95%
- Zero message loss for active connections

## Dependencies
- Existing authentication system (JWT)
- Redis server (for pub/sub)
- PostgreSQL (for persistence)
- Existing user and training session models

## Risks & Mitigations
- **Risk**: WebSocket scalability issues
  - **Mitigation**: Horizontal scaling with Redis pub/sub, connection limits
- **Risk**: Message loss during disconnection
  - **Mitigation**: Client-side queuing, server-side buffering
- **Risk**: Redis single point of failure
  - **Mitigation**: Redis Sentinel/Cluster for HA

## Future Enhancements
- Video streaming for form analysis
- Multi-athlete group sessions
- Live leaderboards during races
- Integration with smart trainers (Zwift-style)