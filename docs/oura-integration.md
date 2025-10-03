# Oura Ring Integration

This document describes the Oura Ring wearable integration for recovery monitoring in AI Coach.

## Overview

The Oura integration enables automatic synchronization of recovery data including:
- **HRV (Heart Rate Variability)**: RMSSD from readiness metrics
- **Sleep Data**: Total sleep, deep/REM/light phases, efficiency, latency
- **Resting Heart Rate**: Derived from daily heart rate minimums

## Architecture

### Components

1. **OuraApiClient** (`src/services/oura_api_client.rs`)
   - OAuth 2.0 flow management
   - API rate limiting (5,000 requests/day)
   - Data retrieval from Oura API v2
   - Token refresh handling

2. **OuraIntegrationService** (`src/services/oura_integration_service.rs`)
   - Connection management
   - Data synchronization
   - Database storage
   - Error handling

3. **API Endpoints** (`src/api/oura_wearable.rs`)
   - `/api/v1/recovery/wearables/oura/authorize` - Start OAuth flow
   - `/api/v1/recovery/wearables/oura/callback` - OAuth callback
   - `/api/v1/recovery/wearables/oura/sync` - Manual data sync
   - `/api/v1/recovery/wearables/oura/disconnect` - Remove connection

## Setup Instructions

### 1. Oura Developer Account

1. Visit [Oura Cloud](https://cloud.ouraring.com/)
2. Create a developer account
3. Create a new OAuth application
4. Note your Client ID and Client Secret

### 2. Configure OAuth Redirect URI

Set your redirect URI in the Oura developer console:
```
http://localhost:3000/api/v1/recovery/wearables/oura/callback  # Development
https://your-domain.com/api/v1/recovery/wearables/oura/callback  # Production
```

### 3. Environment Variables

Add the following to your `.env` file:

```bash
# Oura OAuth Configuration
OURA_CLIENT_ID=your_client_id_here
OURA_CLIENT_SECRET=your_client_secret_here
OURA_REDIRECT_URI=http://localhost:3000/api/v1/recovery/wearables/oura/callback
```

### 4. Restart API Server

The Oura routes will only be enabled if all three environment variables are present:

```bash
cargo run
```

You should see:
```
Oura wearable integration enabled
```

## Usage

### User Connection Flow

1. **Initiate OAuth**:
   ```bash
   GET /api/v1/recovery/wearables/oura/authorize
   Authorization: Bearer <jwt_token>
   ```

   This redirects the user to Oura's OAuth page.

2. **User Authorizes**: User logs in to Oura and grants permissions

3. **OAuth Callback**: Oura redirects back with authorization code

4. **Token Exchange**: System automatically exchanges code for access token

5. **Connection Stored**: Connection saved to `wearable_connections` table

### Data Synchronization

#### Manual Sync
```bash
POST /api/v1/recovery/wearables/oura/sync?days_back=30
Authorization: Bearer <jwt_token>
```

Response:
```json
{
  "success": true,
  "sleep_records": 25,
  "hrv_readings": 28,
  "rhr_readings": 30,
  "errors": []
}
```

#### Automatic Sync
Configure background job to sync all users periodically:
```rust
// TODO: Implement in background_job_service.rs
// Run daily at 6 AM for all connected users
```

### Disconnect

```bash
DELETE /api/v1/recovery/wearables/oura/disconnect
Authorization: Bearer <jwt_token>
```

Response:
```json
{
  "success": true,
  "message": "Oura Ring successfully disconnected"
}
```

## Data Mapping

### Sleep Data

| Oura Field | Our Field | Transformation |
|------------|-----------|----------------|
| `total_sleep_duration` (seconds) | `total_sleep_hours` | Divide by 3600 |
| `deep_sleep_duration` | `deep_sleep_hours` | Divide by 3600 |
| `rem_sleep_duration` | `rem_sleep_hours` | Divide by 3600 |
| `light_sleep_duration` | `light_sleep_hours` | Divide by 3600 |
| `awake_time` | `awake_hours` | Divide by 3600 |
| `efficiency` (0-1) | `sleep_efficiency` (0-100) | Multiply by 100 |
| `latency` (seconds) | `sleep_latency_minutes` | Divide by 60 |
| `bedtime_start` | `bedtime` | Direct mapping |
| `bedtime_end` | `wake_time` | Direct mapping |

### HRV Data

| Oura Field | Our Field | Notes |
|------------|-----------|-------|
| `contributors.hrv_balance` | `rmssd` | Simplified mapping (needs improvement) |
| `readiness_score` | `metadata.readiness_score` | Stored in metadata |
| `temperature_deviation` | `metadata.temperature_deviation` | Stored in metadata |

**Note**: Oura provides HRV balance score (0-100) rather than raw RMSSD. Current implementation uses a simplified mapping. For more accurate HRV tracking, users should also log manual HRV measurements.

### Resting HR Data

| Oura Field | Our Field | Transformation |
|------------|-----------|----------------|
| Heart rate readings | `resting_hr` | Daily minimum BPM |

## Rate Limiting

- **Oura API Limit**: 5,000 requests per day per user
- **Recommended Sync Frequency**: Once per day
- **Error Handling**: Exponential backoff on rate limit errors

## Error Handling

### Common Errors

1. **Token Expired**
   - Automatically refreshed using refresh token
   - If refresh fails, user must re-authorize

2. **Rate Limit Exceeded**
   - Returns HTTP 429 with `Retry-After` header
   - Service respects retry delay

3. **OAuth Errors**
   - User cancels authorization: Return error in callback
   - Invalid credentials: Log error and notify user

### Monitoring

Log all sync operations:
```rust
tracing::info!("Oura sync completed for user {}: {} sleep, {} HRV, {} RHR",
    user_id, sleep_count, hrv_count, rhr_count);
tracing::error!("Failed to sync Oura data: {}", error);
```

## Security Considerations

1. **Token Storage**: Access tokens encrypted at rest in database
2. **HTTPS Required**: All OAuth flows require HTTPS in production
3. **Token Refresh**: Automatic refresh before expiration
4. **User Privacy**: Users can disconnect anytime
5. **GDPR Compliance**: Support data export and deletion

## Future Enhancements

1. **Webhook Support**: Real-time data updates from Oura
2. **HRV Improvement**: Better RMSSD calculation from Oura data
3. **Additional Metrics**: Activity, workout, and tag data
4. **Batch Operations**: Efficient multi-user sync
5. **Analytics Dashboard**: Visualize synced recovery data

## Troubleshooting

### "Oura wearable integration disabled" message

**Cause**: Missing environment variables

**Solution**:
```bash
# Check if all variables are set
echo $OURA_CLIENT_ID
echo $OURA_CLIENT_SECRET
echo $OURA_REDIRECT_URI

# Add to .env file if missing
```

### OAuth redirect fails

**Cause**: Redirect URI mismatch

**Solution**: Ensure redirect URI in:
1. Oura developer console
2. `.env` file
3. OAuth authorization URL

Match exactly (including http/https, port, path).

### No data synced

**Cause**: User has no data in date range

**Solution**: Adjust `days_back` parameter or check Oura app for data.

## References

- [Oura API v2 Documentation](https://cloud.ouraring.com/v2/docs)
- [OAuth 2.0 Specification](https://oauth.net/2/)
- [Issue #54: Recovery Monitoring - Wearable Integration](https://github.com/your-repo/issues/54)
