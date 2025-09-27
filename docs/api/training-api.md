# Training Data Integration API Documentation

This document describes the REST API endpoints for the Training Data Integration Service, which allows users to upload, process, and analyze training data files (TCX, GPX, CSV).

## Base URL

All endpoints are prefixed with `/api/training`

## Authentication

All endpoints require JWT authentication. Include the JWT token in the `Authorization` header:

```
Authorization: Bearer <jwt_token>
```

## Endpoints

### 1. Upload Training File

Upload a training data file for processing.

**Endpoint:** `POST /upload`

**Content-Type:** `multipart/form-data`

**Parameters:**
- `file` (form field): The training data file (TCX, GPX, or CSV)
- `process_immediately` (query param, optional): Boolean to process file immediately (default: false)

**Supported File Types:**
- **TCX**: Training Center XML files from Garmin and other devices
- **GPX**: GPS Exchange Format files
- **CSV**: Comma-separated values with training data

**Response:**
```json
{
  "file_id": "uuid",
  "filename": "workout.tcx",
  "file_path": "/uploads/user_id/unique_filename.tcx",
  "processing_status": "uploaded|processing|processed|failed",
  "metrics": null | {...},
  "job_id": "uuid"
}
```

**Example:**
```bash
curl -X POST http://localhost:3000/api/training/upload \
  -H "Authorization: Bearer <token>" \
  -F "file=@workout.tcx" \
  -F "process_immediately=true"
```

### 2. Get Training Sessions

Retrieve user's training sessions with pagination.

**Endpoint:** `GET /sessions`

**Query Parameters:**
- `limit` (optional): Number of sessions to return (1-100, default: 50)
- `offset` (optional): Number of sessions to skip (default: 0)

**Response:**
```json
[
  {
    "id": "uuid",
    "user_id": "uuid",
    "date": "2023-01-01",
    "trainrs_data": {...},
    "uploaded_file_path": "/path/to/file.tcx",
    "session_type": "training",
    "duration_seconds": 3600,
    "distance_meters": 40000.0,
    "created_at": "2023-01-01T10:00:00Z",
    "updated_at": "2023-01-01T10:00:00Z"
  }
]
```

### 3. Get Training Metrics

Get processed training metrics for a specific session.

**Endpoint:** `GET /sessions/{session_id}/metrics`

**Path Parameters:**
- `session_id`: UUID of the training session

**Response:**
```json
{
  "session_id": "uuid",
  "metrics": {
    "duration_seconds": 3600,
    "distance_meters": 40000.0,
    "average_power": 220.0,
    "normalized_power": 235.0,
    "average_heart_rate": 150.0,
    "tss": 85.0,
    "intensity_factor": 0.94,
    "power_zones": {
      "zone_1": 15.0,
      "zone_2": 25.0,
      "zone_3": 30.0,
      "zone_4": 20.0,
      "zone_5": 8.0,
      "zone_6": 2.0,
      "zone_7": 0.0
    }
  },
  "processing_status": "processed",
  "last_updated": "2023-01-01T10:30:00Z"
}
```

### 4. Process Training Session

Manually trigger processing of an uploaded training session.

**Endpoint:** `POST /process/{session_id}`

**Path Parameters:**
- `session_id`: UUID of the training session to process

**Response:**
```json
{
  "session_id": "uuid",
  "metrics": {...},
  "processing_status": "processed"
}
```

### 5. Get Performance Management Chart

Retrieve Performance Management Chart (PMC) data for training load analysis.

**Endpoint:** `GET /pmc`

**Query Parameters:**
- `days` (optional): Number of days to analyze (7-365, default: 90)

**Response:**
```json
{
  "user_id": "uuid",
  "pmc_data": [
    {
      "date": "2023-01-01",
      "ctl": 45.2,
      "atl": 52.1,
      "tsb": -6.9,
      "tss_daily": 85.0
    }
  ],
  "date_range": "90 days",
  "calculated_at": "2023-01-01T10:00:00Z"
}
```

**PMC Metrics:**
- **CTL (Chronic Training Load)**: Long-term fitness (42-day rolling average)
- **ATL (Acute Training Load)**: Short-term fatigue (7-day rolling average)
- **TSB (Training Stress Balance)**: Form = CTL - ATL
- **TSS Daily**: Training Stress Score for each day

### 6. Get Background Job Status

Check the status of a background processing job.

**Endpoint:** `GET /jobs/{job_id}`

**Path Parameters:**
- `job_id`: UUID of the background job

**Response:**
```json
{
  "job_id": "uuid",
  "job_type": "ProcessTrainingFile { session_id: uuid, user_id: uuid, file_path: string }",
  "status": "Pending|Running|Completed|Failed",
  "created_at": "2023-01-01T10:00:00Z",
  "started_at": "2023-01-01T10:01:00Z",
  "completed_at": "2023-01-01T10:05:00Z",
  "error_message": null,
  "retries": 0
}
```

### 7. Get User Jobs

Retrieve all background jobs for the authenticated user.

**Endpoint:** `GET /jobs`

**Response:**
```json
[
  {
    "job_id": "uuid",
    "job_type": "ProcessTrainingFile {...}",
    "status": "Completed",
    "created_at": "2023-01-01T10:00:00Z",
    "started_at": "2023-01-01T10:01:00Z",
    "completed_at": "2023-01-01T10:05:00Z",
    "error_message": null,
    "retries": 0
  }
]
```

## Error Responses

All endpoints return appropriate HTTP status codes and error messages:

### 400 Bad Request
```json
{
  "error_code": "VALIDATION_ERROR",
  "message": "Invalid request parameters",
  "details": {
    "field": "limit",
    "error": "Limit must be between 1 and 100"
  }
}
```

### 401 Unauthorized
```json
{
  "error_code": "UNAUTHORIZED",
  "message": "Invalid or missing authentication token"
}
```

### 403 Forbidden
```json
{
  "error_code": "FORBIDDEN",
  "message": "Access denied to this resource"
}
```

### 404 Not Found
```json
{
  "error_code": "NOT_FOUND",
  "message": "Resource not found"
}
```

### 413 Payload Too Large
```json
{
  "error_code": "FILE_TOO_LARGE",
  "message": "File size exceeds maximum limit of 50MB"
}
```

### 415 Unsupported Media Type
```json
{
  "error_code": "UNSUPPORTED_FILE_TYPE",
  "message": "File type not supported. Supported types: TCX, GPX, CSV"
}
```

### 500 Internal Server Error
```json
{
  "error_code": "INTERNAL_ERROR",
  "message": "An internal server error occurred"
}
```

## Training Data Processing

### Supported Metrics

The system extracts the following metrics from training files:

**Basic Metrics:**
- Duration (seconds)
- Distance (meters)
- Elevation gain (meters)

**Power Metrics:**
- Average power (watts)
- Normalized power (watts)
- Intensity Factor (IF)
- Training Stress Score (TSS)
- Total work (kilojoules)

**Heart Rate Metrics:**
- Average heart rate (bpm)
- Maximum heart rate (bpm)

**Other Metrics:**
- Average/maximum cadence (rpm)
- Average/maximum speed (m/s)
- Calories burned

**Zone Analysis:**
- Power zones (7 zones based on FTP)
- Heart rate zones (5 zones based on LTHR)
- Time/percentage in each zone

### File Format Requirements

**TCX Files:**
- Must be valid XML with TrainingCenterDatabase root element
- Should contain Activities with Activity elements
- Supports power, heart rate, GPS, and cadence data

**GPX Files:**
- Must be valid XML with gpx root element
- Should contain track (trk), route (rte), or waypoint (wpt) data
- Primarily GPS data, may include extensions for power/HR

**CSV Files:**
- Must have recognizable headers (time, power, heart_rate, etc.)
- Comma-separated format
- First row should contain column names

### Processing Flow

1. **Upload**: File is uploaded and validated
2. **Storage**: File is stored in user-specific directory
3. **Processing**: File is parsed to extract trackpoint data
4. **Analysis**: Metrics are calculated from trackpoint data
5. **Storage**: Metrics are stored in database
6. **Caching**: Results are cached for performance

### Performance Management Chart

The PMC is calculated using exponential weighted moving averages:

- **CTL**: 42-day time constant (long-term fitness)
- **ATL**: 7-day time constant (short-term fatigue)
- **TSB**: CTL - ATL (training stress balance)

PMC data helps athletes understand:
- Fitness trends over time
- Training load balance
- Optimal timing for events
- Recovery requirements

## Rate Limits

- File uploads: 10 files per hour per user
- API requests: 1000 requests per hour per user
- Background jobs: 20 concurrent jobs per user

## File Size Limits

- Maximum file size: 50MB
- Minimum file size: 10 bytes
- Supported formats: TCX, GPX, CSV

## Data Retention

- Uploaded files: Retained for 1 year
- Processed metrics: Retained indefinitely
- Background job logs: Retained for 30 days
- PMC cache: 1 hour TTL