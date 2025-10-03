# Recovery Monitoring MVP - Manual Data Entry

## Overview

This document describes the MVP implementation of the Recovery Monitoring feature (Issue #54). This initial phase focuses on manual data entry for three key recovery metrics:

- **Heart Rate Variability (HRV)**: RMSSD, SDNN, pNN50
- **Sleep Data**: Total sleep, sleep stages, efficiency
- **Resting Heart Rate**: Daily resting HR measurements

## Architecture

### Database Schema

The recovery monitoring system uses five core tables:

1. **`hrv_readings`**: Heart Rate Variability measurements
2. **`sleep_data`**: Sleep metrics and stages
3. **`resting_hr_data`**: Resting heart rate measurements
4. **`recovery_baselines`**: Calculated baseline values per user
5. **`wearable_connections`**: OAuth connections to wearable devices (for future phases)

All tables support multiple data sources: `manual`, `oura`, `whoop`, `apple_health`, `garmin`, `polar`, `fitbit`.

### Data Models

Location: `ai-coach-api/src/models/recovery_data.rs`

**Database Models:**
- `HrvReading`: HRV measurements with RMSSD (primary), SDNN, pNN50
- `SleepData`: Sleep duration, stages (deep, REM, light), efficiency
- `RestingHrData`: Resting heart rate measurements
- `RecoveryBaseline`: Calculated 30-day averages
- `WearableConnection`: Device OAuth credentials (future use)

**Request DTOs:**
- `CreateHrvReadingRequest`: Validated HRV input (0-200ms for RMSSD/SDNN, 0-100% for pNN50)
- `CreateSleepDataRequest`: Validated sleep input (0-24 hours, 0-100% efficiency)
- `CreateRestingHrRequest`: Validated RHR input (30-150 bpm)

**Response DTOs:**
- `HrvReadingResponse`, `HrvReadingsListResponse`: HRV data with pagination
- `SleepDataResponse`, `SleepDataListResponse`: Sleep data with pagination
- `RestingHrResponse`, `RestingHrListResponse`: RHR data with pagination
- `RecoveryBaselineResponse`: Calculated baselines

### Service Layer

Location: `ai-coach-api/src/services/recovery_data_service.rs`

`RecoveryDataService` provides:

**HRV Operations:**
- `create_hrv_reading()`: Create new HRV reading with validation
- `get_hrv_readings()`: List HRV readings with date filtering and pagination

**Sleep Operations:**
- `create_sleep_data()`: Create new sleep data with validation
- `get_sleep_data()`: List sleep data with date filtering and pagination

**Resting HR Operations:**
- `create_resting_hr()`: Create new RHR reading with validation
- `get_resting_hr_data()`: List RHR data with date filtering and pagination

**Baseline Operations:**
- `get_or_calculate_baseline()`: Get existing or calculate new baseline
- `calculate_baseline()`: Compute 30-day averages (requires 14+ days of data)

### API Endpoints

Location: `ai-coach-api/src/api/recovery.rs`

Base path: `/api/v1/recovery`

#### HRV Endpoints

**POST `/api/v1/recovery/hrv`**
- Create HRV reading
- Auth: Required (JWT)
- Body:
  ```json
  {
    "rmssd": 45.5,
    "sdnn": 60.2,
    "pnn50": 25.3,
    "measurement_timestamp": "2024-01-15T08:00:00Z",
    "metadata": {}
  }
  ```
- Response: `HrvReadingResponse`

**GET `/api/v1/recovery/hrv`**
- List HRV readings
- Auth: Required (JWT)
- Query params:
  - `from_date`: Filter start date (YYYY-MM-DD)
  - `to_date`: Filter end date (YYYY-MM-DD)
  - `limit`: Results per page (1-1000, default: 100)
  - `page`: Page number (default: 1)
- Response: `HrvReadingsListResponse` with pagination

#### Sleep Endpoints

**POST `/api/v1/recovery/sleep`**
- Create sleep data
- Auth: Required (JWT)
- Body:
  ```json
  {
    "total_sleep_hours": 7.5,
    "deep_sleep_hours": 1.5,
    "rem_sleep_hours": 1.8,
    "light_sleep_hours": 4.2,
    "sleep_efficiency": 92.5,
    "sleep_latency_minutes": 12,
    "bedtime": "2024-01-14T22:30:00Z",
    "wake_time": "2024-01-15T06:00:00Z",
    "sleep_date": "2024-01-15"
  }
  ```
- Response: `SleepDataResponse`

**GET `/api/v1/recovery/sleep`**
- List sleep data
- Auth: Required (JWT)
- Query params: Same as HRV endpoints
- Response: `SleepDataListResponse` with pagination

#### Resting HR Endpoints

**POST `/api/v1/recovery/resting-hr`**
- Create resting HR reading
- Auth: Required (JWT)
- Body:
  ```json
  {
    "resting_hr": 52.0,
    "measurement_timestamp": "2024-01-15T08:00:00Z"
  }
  ```
- Response: `RestingHrResponse`

**GET `/api/v1/recovery/resting-hr`**
- List resting HR data
- Auth: Required (JWT)
- Query params: Same as HRV endpoints
- Response: `RestingHrListResponse` with pagination

#### Baseline Endpoint

**GET `/api/v1/recovery/baseline`**
- Get or calculate recovery baseline
- Auth: Required (JWT)
- Calculates 30-day averages for:
  - HRV baseline (RMSSD)
  - Resting HR baseline
  - Typical sleep duration
- Requires minimum 14 days of data
- Response: `RecoveryBaselineResponse`
- Error: 404 if insufficient data

## Data Validation

### Input Validation

All requests use the `validator` crate for comprehensive validation:

**HRV Validation:**
- RMSSD: 0-200ms (typical physiological range)
- SDNN: 0-200ms (optional)
- pNN50: 0-100% (optional)

**Sleep Validation:**
- Total sleep: 0-24 hours
- Sleep stages: 0-24 hours each (optional)
- Sleep efficiency: 0-100%
- Sleep latency: ≥0 minutes (optional)

**Resting HR Validation:**
- Resting HR: 30-150 bpm (physiological range)

### Database Constraints

- Unique constraints prevent duplicate measurements
- Check constraints enforce physiological ranges
- Foreign keys ensure data integrity
- Indexes optimize query performance

## Baseline Calculation

The baseline calculation uses a 30-day rolling window:

1. **Data Collection Period**: Last 30 days from current date
2. **Minimum Data**: Requires 14+ days with measurements
3. **Calculations**:
   - HRV Baseline: AVG(rmssd) over 30 days
   - RHR Baseline: AVG(resting_hr) over 30 days
   - Typical Sleep: AVG(total_sleep_hours) over 30 days
4. **Auto-Update**: Baseline recalculated on each GET request
5. **Data Points**: Tracks number of unique days with data

## Usage Examples

### 1. Log HRV Reading

```bash
curl -X POST http://localhost:3000/api/v1/recovery/hrv \
  -H "Authorization: Bearer <JWT_TOKEN>" \
  -H "Content-Type: application/json" \
  -d '{
    "rmssd": 45.5,
    "sdnn": 60.2,
    "pnn50": 25.3
  }'
```

### 2. Log Sleep Data

```bash
curl -X POST http://localhost:3000/api/v1/recovery/sleep \
  -H "Authorization: Bearer <JWT_TOKEN>" \
  -H "Content-Type: application/json" \
  -d '{
    "total_sleep_hours": 7.5,
    "deep_sleep_hours": 1.5,
    "rem_sleep_hours": 1.8,
    "light_sleep_hours": 4.2,
    "sleep_efficiency": 92.5
  }'
```

### 3. Get HRV Readings (Last 30 Days)

```bash
curl -X GET "http://localhost:3000/api/v1/recovery/hrv?from_date=2024-01-01&limit=50" \
  -H "Authorization: Bearer <JWT_TOKEN>"
```

### 4. Get Recovery Baseline

```bash
curl -X GET http://localhost:3000/api/v1/recovery/baseline \
  -H "Authorization: Bearer <JWT_TOKEN>"
```

## Error Handling

### Validation Errors (400 Bad Request)

```json
{
  "error_code": "VALIDATION_ERROR",
  "message": "Invalid HRV reading data",
  "details": {
    "errors": "rmssd: RMSSD must be between 0 and 200 ms"
  }
}
```

### Authentication Errors (401 Unauthorized)

Returned when JWT token is missing or invalid.

### Insufficient Data (404 Not Found)

```json
{
  "error_code": "INSUFFICIENT_DATA",
  "message": "Not enough data to calculate baseline (need at least 14 days of data)"
}
```

### Database Errors (500 Internal Server Error)

```json
{
  "error_code": "DATABASE_ERROR",
  "message": "Failed to create HRV reading"
}
```

## Testing

### Manual Testing Steps

1. **Start PostgreSQL**:
   ```bash
   docker-compose up -d db
   ```

2. **Set Database URL**:
   ```bash
   export DATABASE_URL=postgresql://postgres:password@localhost:5432/ai_coach
   ```

3. **Run Migrations** (automatic on server start):
   ```bash
   cargo run
   ```

4. **Create Test User**:
   ```bash
   curl -X POST http://localhost:3000/api/v1/auth/register \
     -H "Content-Type: application/json" \
     -d '{"email": "test@example.com", "password": "Test123!"}'
   ```

5. **Login and Get Token**:
   ```bash
   curl -X POST http://localhost:3000/api/v1/auth/login \
     -H "Content-Type: application/json" \
     -d '{"email": "test@example.com", "password": "Test123!"}'
   ```

6. **Test Recovery Endpoints** (use examples above with obtained JWT)

### Integration Tests

Location: `ai-coach-api/tests/integration/recovery_test.rs` (to be created)

Test coverage should include:
- ✅ HRV CRUD operations
- ✅ Sleep CRUD operations
- ✅ Resting HR CRUD operations
- ✅ Baseline calculation with sufficient data
- ✅ Baseline error with insufficient data
- ✅ Validation error handling
- ✅ Date filtering and pagination
- ✅ Authentication requirements

## Future Enhancements

This MVP provides the foundation for:

1. **Phase 2: Oura Ring Integration** (OAuth, webhook, sync)
2. **Phase 3: Whoop Integration** (OAuth, webhook, sync)
3. **Phase 4: Apple Health Integration** (XML import)
4. **Phase 5: Data Quality & Validation** (outlier detection, quality scores)
5. **Phase 6: Recovery Analysis** (trend analysis, injury prediction)

See issue #54 for the complete roadmap.

## Migration Information

**Migration File**: `ai-coach-api/migrations/018_create_recovery_monitoring_tables.sql`

Run migrations:
```bash
export DATABASE_URL=postgresql://postgres:password@localhost:5432/ai_coach
cargo run  # Migrations run automatically
```

## Security Considerations

1. **Authentication**: All endpoints require valid JWT token
2. **User Isolation**: Users can only access their own recovery data
3. **Input Validation**: Comprehensive validation prevents invalid data
4. **SQL Injection**: SQLx query macros prevent SQL injection
5. **Token Storage**: OAuth tokens encrypted at rest (future phases)

## Performance Considerations

1. **Indexes**: Optimized for common queries (user_id, date ranges)
2. **Pagination**: Default limit of 100 items prevents large payloads
3. **Baseline Caching**: Consider caching baselines for 24 hours
4. **Query Optimization**: Use of indexes for date-based filtering

## API Documentation

Full API documentation available at: `/api/v1/docs` (Swagger UI)

OpenAPI spec includes:
- All endpoint definitions
- Request/response schemas
- Validation rules
- Error responses
- Authentication requirements

## Support & Issues

For issues related to recovery monitoring, create a GitHub issue with:
- API endpoint affected
- Request payload
- Error response
- Expected behavior

Reference: Issue #54
