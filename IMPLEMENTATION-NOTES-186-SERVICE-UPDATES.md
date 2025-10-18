# Recommendation Effectiveness Service - SQLite Update Guide

## Overview
The `recommendation_effectiveness_service.rs` file requires extensive updates for SQLite compatibility. This document outlines all necessary changes.

## Required Changes

### 1. Pool Type (Lines 4, 17, 21)
```rust
// Change:
use sqlx::{PgPool, Row};
db: PgPool,
pub fn new(db: PgPool) -> Self

// To:
use sqlx::{SqlitePool, Row};
db: SqlitePool,
pub fn new(db: SqlitePool) -> Self
```

### 2. Query Macros → Runtime Queries
All `query!()` and `query_as!()` macros need to be converted to runtime queries because they rely on PostgreSQL compile-time checking.

**Lines 42-80**: `track_outcome` INSERT query
**Lines 101-117**: `update_outcome_with_next_day_score` SELECT query
**Lines 129-155**: UPDATE with `NOW()` → `datetime('now')`
**Lines 178-183, 190-200**: Template effectiveness queries with `NOW()`
**Lines 223-241**: `process_daily_outcomes` SELECT query
**Lines 308-322**: `get_latest_recovery_score` SELECT query
**Lines 461-473**: System analytics aggregation
**Lines 484-499**: Category analytics with type casts
**Lines 601-625**: Flag underperforming with `NOW()` and `INTERVAL`

### 3. PostgreSQL-Specific SQL → SQLite

**Type Casts** (`::text`, `::float`):
- Line 333: `rt.category::text` → `rt.category`
- Line 348: `rt.category::text` → `rt.category`
- Line 384: `::float` → `CAST(... AS REAL)` or remove
- Line 487: `rt.category::text` → `rt.category`
- Line 547: `rt.category::text` → `rt.category`
- Line 576: `rt.category::text` → `rt.category`
- Line 588: `::float` → `CAST(... AS REAL)` or remove

**Date/Time Functions**:
- Line 138: `NOW()` → `datetime('now')`
- Line 193: `NOW()` → `datetime('now')`
- Line 367: `NOW() - INTERVAL '30 days'` → `datetime('now', '-30 days')`
- Line 604: `NOW()` → `datetime('now')`
- Line 612: `NOW() - INTERVAL '30 days'` → `datetime('now', '-30 days')`

**Row Type**:
- Line 631: `sqlx::postgres::PgRow` → `sqlx::sqlite::SqliteRow`

### 4. Function Name
- Line 9: `calculate_profile_effectiveness_score` should be `calculate_outcome_effectiveness_score` (check models)
- Line 121: Same function call

## Recommendation

Due to the extensive changes needed (20+ locations with PostgreSQL-specific features), consider one of these approaches:

### Option A: Incremental Refactoring
1. Start with simpler methods that don't use query! macros
2. Convert query! → query() for each method
3. Test each method individually
4. Handle type conversions manually with `row.get()`

### Option B: Defer to Phase 5 Testing
1. Leave service commented out for now
2. Focus on testing `recommendation_tracking_service` and `recommendation_engine_service`
3. Complete this service once the simpler services are validated

### Option C: Simplified Implementation
1. Create a minimal version with just `track_outcome` and `update_outcome_with_next_day_score`
2. Comment out analytics methods for MVP
3. Restore full analytics in future PR

## Status
**Current**: Service file needs updates
**Recommended**: Option B - defer to Phase 5, focus on simpler services first
