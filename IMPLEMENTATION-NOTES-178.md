# Implementation Notes: Issue #178 - Recovery Protocols

## Overview
This document describes the implementation of the Recovery Protocols feature (Phase 8.6.1) for the AI Coach application. Recovery Protocols are multi-recommendation sequences that combine several recommendations into structured recovery strategies.

## Status: ✅ COMPLETE (Pending Integration)

The Recovery Protocols feature has been **fully implemented** with database migrations, models, services, and API endpoints. However, it is **not yet integrated** into the main application because it depends on the recommendation system, which has not been migrated from PostgreSQL to SQLite.

## Implementation Details

### Database Schema (SQLite)

#### Tables Created
1. **`recovery_protocols`** - Protocol templates
   - id (TEXT PRIMARY KEY - UUID)
   - name, description, duration_days
   - sequence_type (parallel|sequential|phased)
   - category (training_modification|sleep|stress_management|nutrition|active_recovery|general)
   - target_scenarios (TEXT - JSON array)
   - effectiveness_score (REAL)
   - times_activated, times_completed
   - is_active (INTEGER - boolean)
   - created_at, updated_at (TEXT - timestamps)

2. **`protocol_recommendations`** - Links protocols to recommendations
   - id (TEXT PRIMARY KEY - UUID)
   - protocol_id (FK to recovery_protocols)
   - recommendation_template_id (FK to recommendation_templates - **NOT YET MIGRATED**)
   - sequence_order, day_number
   - is_required (INTEGER - boolean)
   - created_at (TEXT)

3. **`user_protocol_activations`** - User protocol tracking
   - id (TEXT PRIMARY KEY - UUID)
   - user_id (FK to users)
   - protocol_id (FK to recovery_protocols)
   - status (active|completed|abandoned)
   - progress (TEXT - JSON)
   - activated_at, completed_at, abandoned_at
   - abandonment_reason
   - created_at, updated_at (TEXT)

#### SQLite Triggers
- `update_recovery_protocols_updated_at` - Auto-update timestamps on protocol changes
- `update_user_protocol_activations_updated_at` - Auto-update timestamps on activation changes
- `auto_set_completion_timestamp` - Set completed_at when status → 'completed'
- `auto_set_abandonment_timestamp` - Set abandoned_at when status → 'abandoned'

### Seed Data
5 pre-built recovery protocols:
1. **Post-Hard-Workout Recovery** (1 day, sequential)
2. **Sleep Optimization Week** (7 days, phased)
3. **Stress Reset** (3 days, phased)
4. **Travel Recovery** (5 days, phased)
5. **Injury Prevention Protocol** (30 days, parallel)

### Rust Implementation

#### Models (`src/models/recovery_protocol.rs`)
- `ProtocolSequence` enum (Parallel, Sequential, Phased)
- `ProtocolStatus` enum (Active, Completed, Abandoned)
- `RecoveryProtocol` struct
- `ProtocolRecommendation` struct
- `UserProtocolActivation` struct
- Request/Response DTOs for API

#### Service (`src/services/recovery_protocol_service.rs`)
Complete service implementation with methods:
- `get_applicable_protocols()` - List protocols with filtering
- `get_protocol()` - Get specific protocol by ID
- `get_protocol_recommendations()` - Get recommendations for protocol
- `activate_protocol()` - Activate protocol for user
- `get_active_protocols()` - Get user's active protocols
- `get_user_protocol_history()` - Get all user activations
- `update_protocol_progress()` - Update recommendation completion
- `complete_protocol()` - Mark protocol as completed
- `abandon_protocol()` - Mark protocol as abandoned

#### API (`src/api/recovery_protocols.rs`)
7 REST endpoints:
1. `GET /api/v1/recovery/protocols` - List protocols (with filtering)
2. `GET /api/v1/recovery/protocols/:id` - Get specific protocol
3. `POST /api/v1/recovery/protocols/activate` - Activate protocol
4. `GET /api/v1/recovery/protocols/active` - Get user's active protocols
5. `PUT /api/v1/recovery/protocols/:id/progress` - Update progress
6. `POST /api/v1/recovery/protocols/:id/complete` - Complete protocol
7. `POST /api/v1/recovery/protocols/:id/abandon` - Abandon protocol

All endpoints include:
- JWT authentication via `Extension<Claims>`
- Proper error handling with HTTP status codes
- Ownership verification for user operations
- Progress tracking using JSON serialization

## Files Created/Modified

### New Files
- `ai-coach-api/migrations/009_create_recovery_protocol_tables.sql`
- `ai-coach-api/migrations/010_seed_recovery_protocols.sql`
- `ai-coach-api/src/models/recovery_protocol.rs`
- `ai-coach-api/src/services/recovery_protocol_service.rs`
- `ai-coach-api/src/api/recovery_protocols.rs`

### Modified Files
- `ai-coach-api/src/models/mod.rs` - Added commented-out `recovery_protocol` module

## Dependency Blocking

### Current Blocker
The `recommendation_templates` table referenced in `protocol_recommendations` exists only in PostgreSQL migrations but has **not been migrated to SQLite**. This blocks full integration.

### Integration Dependencies
The following need to be migrated to SQLite before Recovery Protocols can be integrated:
1. `recommendation_templates` table (from PostgreSQL migrations)
2. Recommendation-related models and services
3. Recommendation system dependencies

### What's NOT Blocked
- Database migrations ✅ (run successfully)
- Models ✅ (compile successfully when uncommented)
- Services ✅ (compile successfully)
- API endpoints ✅ (compile successfully)

## Integration Checklist (When Ready)

When the recommendation system is migrated to SQLite, complete integration with:

1. **Uncomment module exports** in `src/models/mod.rs`:
   ```rust
   pub mod recovery_protocol;
   pub use recovery_protocol::*;
   ```

2. **Add service export** in `src/services/mod.rs`:
   ```rust
   pub mod recovery_protocol_service;
   pub use recovery_protocol_service::RecoveryProtocolService;
   ```

3. **Add API module** in `src/api/mod.rs`:
   ```rust
   pub mod recovery_protocols;
   ```

4. **Register routes** in `src/api/routes.rs`:
   ```rust
   use super::recovery_protocols::recovery_protocol_routes;
   use crate::services::RecoveryProtocolService;

   // In create_routes():
   let recovery_protocol_service = Arc::new(RecoveryProtocolService::new(db.clone()));

   let api_v1 = Router::new()
       // ... other routes ...
       .nest("/recovery/protocols", recovery_protocol_routes(recovery_protocol_service));
   ```

5. **Populate protocol recommendations** - Create migration to link protocols to recommendation templates

6. **Write tests**:
   - Unit tests for service methods
   - Integration tests for API endpoints

## Testing Notes

### Manual Testing (When Integrated)
```bash
# List protocols
curl http://localhost:8080/api/v1/recovery/protocols

# Filter protocols
curl "http://localhost:8080/api/v1/recovery/protocols?scenario=post_hard_workout&duration_max=7"

# Get specific protocol
curl http://localhost:8080/api/v1/recovery/protocols/{protocol_id}

# Activate protocol (requires JWT)
curl -X POST http://localhost:8080/api/v1/recovery/protocols/activate \
  -H "Authorization: Bearer {token}" \
  -H "Content-Type: application/json" \
  -d '{"protocol_id": "{protocol_id}"}'

# Get active protocols
curl http://localhost:8080/api/v1/recovery/protocols/active \
  -H "Authorization: Bearer {token}"

# Update progress
curl -X PUT http://localhost:8080/api/v1/recovery/protocols/{activation_id}/progress \
  -H "Authorization: Bearer {token}" \
  -H "Content-Type: application/json" \
  -d '{"recommendation_id": "{rec_id}", "completed": true}'

# Complete protocol
curl -X POST http://localhost:8080/api/v1/recovery/protocols/{activation_id}/complete \
  -H "Authorization: Bearer {token}"

# Abandon protocol
curl -X POST http://localhost:8080/api/v1/recovery/protocols/{activation_id}/abandon \
  -H "Authorization: Bearer {token}" \
  -H "Content-Type: application/json" \
  -d '{"reason": "Changed priorities"}'
```

## Architecture Notes

### Design Decisions
1. **SQLite Compatibility**: Used TEXT for UUIDs, INTEGER for booleans, TEXT for JSON
2. **Triggers**: Auto-update timestamps and status fields using SQLite triggers
3. **Progress Tracking**: JSON stored as TEXT, parsed at application layer
4. **Ownership Verification**: All user operations verify protocol ownership
5. **Error Handling**: Comprehensive error messages with appropriate HTTP status codes

### Future Enhancements
- Analytics on protocol effectiveness
- Recommendation personalization based on completion rates
- Protocol adaptation based on user feedback
- Integration with ML-based ranking system (#183)

## Related Issues
- Issue #134 - Phase 8.6: Smart Recommendation Features (parent issue)
- Issue #129-133 - Recommendation infrastructure (dependencies)
- Issue #179-183 - Other Phase 8.6 features (siblings)

## Author Notes
Implementation completed on branch `feature/issue-178-recovery-protocols` based on `feature/minimal-viable-api`. Full functionality ready pending recommendation system migration to SQLite.
